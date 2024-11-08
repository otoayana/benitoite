use std::{collections::HashMap, ops::Deref, str::FromStr, sync::Arc};

use atrium_api::{
    agent::{store::MemorySessionStore, AtpAgent},
    app::bsky::feed::defs::{FeedViewPostReasonRefs, PostViewEmbedRefs, ReplyRefParentRefs},
    com::atproto::repo::strong_ref::MainData,
    types::{
        string::{AtIdentifier, Nsid},
        Collection, LimitedNonZeroU8, Object, TryIntoUnknown, Union, Unknown,
    },
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use blake3::Hasher;
use futures::future::join_all;
use ipld_core::ipld::{Ipld, IpldKind};
use tokio::sync::Mutex;

use crate::{
    config::Account,
    types::{Media, Post, PostContext},
};

#[derive(Clone)]
pub struct Session {
    id: AtIdentifier,
    agent: Arc<AtpAgent<MemorySessionStore, ReqwestClient>>,
    objects: Arc<Mutex<HashMap<String, MainData>>>,
}

impl Session {
    pub async fn new(
        account: &Account,
        objects: Arc<Mutex<HashMap<String, MainData>>>,
    ) -> Result<Session, Box<dyn std::error::Error>> {
        let agent = AtpAgent::new(
            ReqwestClient::new(&account.pds),
            MemorySessionStore::default(),
        );
        agent.login(&account.username, &account.password).await?;

        let session = agent.api.com.atproto.server.get_session().await?;
        let id = AtIdentifier::Did(session.did.clone());

        println!("session spawned for user @{}", session.handle.to_string());

        Ok(Session {
            id,
            agent: Arc::new(agent),
            objects,
        })
    }

    pub async fn feed(self) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
        let action = self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_timeline(Object::from(
                atrium_api::app::bsky::feed::get_timeline::ParametersData {
                    algorithm: None,
                    cursor: None,
                    limit: Some(LimitedNonZeroU8::try_from(10)?),
                },
            ))
            .await?;

        let feed: Vec<Post> = join_all(action.feed.iter().map(|v| async {
            // Create a hash to use in URIs
            let mut hasher = Hasher::new();
            hasher.update(v.post.uri.clone().as_bytes());
            let hash = hasher.finalize();

            self.objects.lock().await.insert(
                hash.to_string(),
                MainData {
                    cid: v.post.cid.clone(),
                    uri: v.post.uri.clone(),
                },
            );

            Post {
                id: hash.to_string(),
                username: v.post.author.handle.as_str().to_string(),
                body: {
                    if let Unknown::Object(values) = &v.post.record {
                        values
                            .get("text")
                            .unwrap()
                            .iter()
                            .filter(|v| matches!(v.kind(), IpldKind::String))
                            .map(|v| {
                                if let Ipld::String(val) = v {
                                    val.clone()
                                } else {
                                    String::new()
                                }
                            })
                            .next()
                            .unwrap()
                    } else {
                        String::new()
                    }
                },
                media: v.post.embed.clone().map_or(None, |v| match v {
                    Union::Refs(r) => match r {
                        // TODO(otoayana): Add multiple media items
                        PostViewEmbedRefs::AppBskyEmbedImagesView(image) => {
                            let image_data =
                                Box::leak(image).data.images.first().unwrap().data.clone();
                            let alt = if image_data.alt.len() > 0 {
                                image_data
                                    .alt
                                    .chars()
                                    .map(|c| if c == 0xA as char { ' ' } else { c })
                                    .collect()
                            } else {
                                String::from("Photo")
                            };
                            Some(Media::Image((image_data.fullsize, alt)))
                        }
                        PostViewEmbedRefs::AppBskyEmbedExternalView(external) => {
                            let external_data = Box::leak(external).data.external.clone();
                            Some(Media::External((
                                external_data.uri.clone(),
                                external_data.description.clone(),
                            )))
                        }
                        PostViewEmbedRefs::AppBskyEmbedVideoView(_) => Some(Media::Video),
                        _ => None,
                    },
                    Union::Unknown(_) => None,
                }),
                replies: v.post.reply_count.unwrap_or(0) as u64,
                reposts: v.post.repost_count.unwrap_or(0) as u64,
                likes: v.post.like_count.unwrap_or(0) as u64,
                context: if let Some(Union::Refs(FeedViewPostReasonRefs::ReasonRepost(r))) =
                    v.reason.clone()
                {
                    PostContext::Repost(r.deref().by.handle.to_string())
                } else {
                    if let Some(Union::Refs(ReplyRefParentRefs::PostView(reply))) =
                        v.reply.clone().map(|v| v.parent.clone())
                    {
                        PostContext::Reply(reply.author.handle.to_string())
                    } else {
                        PostContext::None
                    }
                },
            }
        }))
        .await;
        Ok(feed)
    }

    pub async fn like<'a>(self, id: &'a str) -> Result<(), Box<dyn std::error::Error>> {
        let hash_map = self.objects.lock().await;
        let object = hash_map.get(id).unwrap();
        let post = self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_posts(Object::from(
                atrium_api::app::bsky::feed::get_posts::ParametersData {
                    uris: vec![object.uri.clone()],
                },
            ))
            .await?;

        if let Some(like) = post
            .clone()
            .posts
            .first()
            .unwrap()
            .viewer
            .clone()
            .unwrap()
            .like
            .clone()
        {
            self.agent
                .api
                .com
                .atproto
                .repo
                .delete_record(Object::from(
                    atrium_api::com::atproto::repo::delete_record::InputData {
                        collection: Nsid::from_str(atrium_api::app::bsky::feed::Like::NSID)?,
                        repo: self.id.clone(),
                        rkey: like.split('/').last().unwrap().to_string(),
                        swap_commit: None,
                        swap_record: None,
                    },
                ))
                .await?;
        } else {
            self.agent
                .api
                .com
                .atproto
                .repo
                .create_record(Object::from(
                    atrium_api::com::atproto::repo::create_record::InputData {
                        collection: Nsid::from_str(atrium_api::app::bsky::feed::Like::NSID)?,
                        record: atrium_api::record::KnownRecord::AppBskyFeedLike(Box::new(
                            Object::from(atrium_api::app::bsky::feed::like::RecordData {
                                created_at: atrium_api::types::string::Datetime::now(),
                                subject: Object::from(object.clone()),
                            }),
                        ))
                        .try_into_unknown()?,
                        repo: self.id.clone(),
                        rkey: None,
                        swap_commit: None,
                        validate: None,
                    },
                ))
                .await?;
        }

        Ok(())
    }

    pub async fn repost<'a>(self, id: &'a str) -> Result<(), Box<dyn std::error::Error>> {
        let hash_map = self.objects.lock().await;
        let object = hash_map.get(id).unwrap();
        let post = self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_posts(Object::from(
                atrium_api::app::bsky::feed::get_posts::ParametersData {
                    uris: vec![object.uri.clone()],
                },
            ))
            .await?;

        if let Some(repost) = post
            .clone()
            .posts
            .first()
            .unwrap()
            .viewer
            .clone()
            .unwrap()
            .repost
            .clone()
        {
            self.agent
                .api
                .com
                .atproto
                .repo
                .delete_record(Object::from(
                    atrium_api::com::atproto::repo::delete_record::InputData {
                        collection: Nsid::from_str(atrium_api::app::bsky::feed::Like::NSID)?,
                        repo: self.id.clone(),
                        rkey: repost.split('/').last().unwrap().to_string(),
                        swap_commit: None,
                        swap_record: None,
                    },
                ))
                .await?;
        } else {
            self.agent
                .api
                .com
                .atproto
                .repo
                .create_record(Object::from(
                    atrium_api::com::atproto::repo::create_record::InputData {
                        collection: Nsid::from_str(atrium_api::app::bsky::feed::Repost::NSID)?,
                        record: atrium_api::record::KnownRecord::AppBskyFeedRepost(Box::new(
                            Object::from(atrium_api::app::bsky::feed::repost::RecordData {
                                created_at: atrium_api::types::string::Datetime::now(),
                                subject: Object::from(object.clone()),
                            }),
                        ))
                        .try_into_unknown()?,
                        repo: self.id.clone(),
                        rkey: None,
                        swap_commit: None,
                        validate: None,
                    },
                ))
                .await?;
        }

        Ok(())
    }
}
