use std::{collections::HashMap, str::FromStr, sync::Arc};

use atrium_api::{
    agent::{store::MemorySessionStore, AtpAgent},
    com::atproto::repo::strong_ref::MainData,
    record::KnownRecord,
    types::{
        string::{AtIdentifier, Datetime, Did, Nsid},
        Collection, LimitedNonZeroU8, Object, TryFromUnknown, TryIntoUnknown,
    },
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use futures::future::join_all;
use tokio::sync::Mutex;

use crate::{
    config::Account,
    types::{Post, Profile},
};

#[derive(Clone)]
pub struct Session {
    id: AtIdentifier,
    agent: Arc<AtpAgent<MemorySessionStore, ReqwestClient>>,
    objects: Arc<Mutex<HashMap<String, MainData>>>,
    pub handle: String,
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

        Ok(Session {
            id,
            agent: Arc::new(agent),
            objects,
            handle: session.handle.to_string(),
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

        let feed: Vec<Post> = join_all(
            action
                .feed
                .iter()
                .map(|v| async { Post::push(v, &self.objects).await }),
        )
        .await;
        Ok(feed)
    }

    pub async fn profile<'a>(self, id: &'a str) -> Result<Profile, Box<dyn std::error::Error>> {
        let identifier = AtIdentifier::from_str(id)?;
        let account = self
            .agent
            .api
            .app
            .bsky
            .actor
            .get_profile(Object::from(
                atrium_api::app::bsky::actor::get_profile::ParametersData {
                    actor: identifier.clone(),
                },
            ))
            .await?;
        let account_feed = self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_author_feed(Object::from(
                atrium_api::app::bsky::feed::get_author_feed::ParametersData {
                    actor: identifier.clone(),
                    cursor: None,
                    filter: None,
                    include_pins: Some(true),
                    limit: LimitedNonZeroU8::try_from(10).ok(),
                },
            ))
            .await?;

        Ok(Profile {
            id: account.handle.clone(),
            name: account
                .display_name
                .clone()
                .unwrap_or(account.handle.to_string()),
            bio: account.description.clone().unwrap_or("".to_string()),
            followers: account.followers_count.unwrap_or(0) as u64,
            follows: account.follows_count.unwrap_or(0) as u64,
            following: account.viewer.clone().unwrap().following.is_some(),
            posts: join_all(
                account_feed
                    .feed
                    .iter()
                    .map(|p| async { Post::push(p, &self.objects).await }),
            )
            .await,
        })
    }

    pub async fn follow<'a>(self, id: &'a str) -> Result<(), Box<dyn std::error::Error>> {
        let identifier = AtIdentifier::from_str(id)?;
        let account = self
            .agent
            .api
            .app
            .bsky
            .actor
            .get_profile(Object::from(
                atrium_api::app::bsky::actor::get_profile::ParametersData {
                    actor: identifier.clone(),
                },
            ))
            .await?;
        let following = account.viewer.clone().unwrap().following.clone();

        if let Some(uri) = following {
            self.agent
                .api
                .com
                .atproto
                .repo
                .delete_record(Object::from(
                    atrium_api::com::atproto::repo::delete_record::InputData {
                        collection: Nsid::from_str(atrium_api::app::bsky::graph::Follow::NSID)?,
                        repo: self.id.clone(),
                        rkey: uri.split("/").last().unwrap().to_string(),
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
                        collection: Nsid::from_str(atrium_api::app::bsky::graph::Follow::NSID)?,
                        record: atrium_api::app::bsky::graph::follow::RecordData {
                            created_at: Datetime::now(),
                            subject: account.did.clone(),
                        }
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

    pub async fn reply<'a>(
        self,
        id: &'a str,
        body: &'a str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let hash_map = self.objects.lock().await;
        let object = hash_map.get(id).unwrap();
        let post = self
            .agent
            .api
            .com
            .atproto
            .repo
            .get_record(Object::from(
                atrium_api::com::atproto::repo::get_record::ParametersData {
                    cid: Some(object.cid.clone()),
                    collection: Nsid::from_str(atrium_api::app::bsky::feed::Post::NSID)?,
                    repo: AtIdentifier::Did(Did::from_str(
                        object.uri.split("/").skip(2).next().unwrap(),
                    )?),
                    rkey: object.uri.split("/").last().unwrap().to_string(),
                },
            ))
            .await?;

        self.agent
            .api
            .com
            .atproto
            .repo
            .create_record(Object::from(
                atrium_api::com::atproto::repo::create_record::InputData {
                    collection: Nsid::from_str(atrium_api::app::bsky::feed::Post::NSID)?,
                    record: atrium_api::record::KnownRecord::AppBskyFeedPost(Box::from(
                        Object::from(atrium_api::app::bsky::feed::post::RecordData {
                            created_at: Datetime::now(),
                            embed: None,
                            entities: None,
                            facets: None,
                            labels: None,
                            langs: None,
                            reply: Some(Object::from(
                                atrium_api::app::bsky::feed::post::ReplyRefData {
                                    parent: Object::from(
                                        atrium_api::com::atproto::repo::strong_ref::MainData {
                                            cid: object.cid.clone(),
                                            uri: object.uri.clone(),
                                        },
                                    ),
                                    root: Object::from({
                                        if let KnownRecord::AppBskyFeedPost(post_boxed) =
                                            KnownRecord::try_from_unknown(post.value.clone())?
                                        {
                                            if let Some(reply) = Box::leak(post_boxed).reply.clone() {
                                                reply
                                                .root
                                                .clone()
                                            } else {
                                                Object::from(atrium_api::com::atproto::repo::strong_ref::MainData {
                                                    cid: object.cid.clone(),
                                                    uri: object.uri.clone(),
                                                })
                                            }
                                        } else {
                                            return Err(Box::from("root not found"));
                                        }
                                    }),
                                },
                            )),
                            tags: None,
                            text: body.to_string(),
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

        Ok(())
    }

    pub async fn post<'a>(self, body: &'a str) -> Result<(), Box<dyn std::error::Error>> {
        self.agent
            .api
            .com
            .atproto
            .repo
            .create_record(Object::from(
                atrium_api::com::atproto::repo::create_record::InputData {
                    collection: Nsid::from_str(atrium_api::app::bsky::feed::Post::NSID)?,
                    record: atrium_api::record::KnownRecord::AppBskyFeedPost(Box::from(
                        Object::from(atrium_api::app::bsky::feed::post::RecordData {
                            created_at: Datetime::now(),
                            embed: None,
                            entities: None,
                            facets: None,
                            labels: None,
                            langs: None,
                            reply: None,
                            tags: None,
                            text: body.to_string(),
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

        Ok(())
    }
}
