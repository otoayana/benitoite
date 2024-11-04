use std::sync::Arc;

use atrium_api::{
    agent::{store::MemorySessionStore, AtpAgent}, app::bsky::feed::defs::InteractionData, types::{LimitedNonZeroU8, Object, Unknown}
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use ipld_core::ipld::{Ipld, IpldKind};
use serde::Serialize;

use crate::{config::Config, types::{Media, Post}};

pub struct Session {
    agent: Arc<AtpAgent<MemorySessionStore, ReqwestClient>>,
}

impl Session {
    pub async fn from_fingerprint<'a>(
        fingerprint: &'a str,
    ) -> Result<Session, Box<dyn std::error::Error>> {
        let config = Config::parse()?;
        let user = config.accounts.get(fingerprint).unwrap();

        let agent = AtpAgent::new(ReqwestClient::new(&user.pds), MemorySessionStore::default());
        agent.login(&user.username, &user.password).await?;

        Ok(Session {
            agent: Arc::new(agent),
        })
    }

    pub async fn feed(&self) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
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

        let feed: Vec<Post> = action
            .feed
            .iter()
            .map(|v| Post {
                id: v.post.uri.clone().trim_start_matches("at://").to_string(),
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
                    atrium_api::types::Union::Refs(r) => match r {
                        atrium_api::app::bsky::feed::defs::PostViewEmbedRefs::AppBskyEmbedImagesView(image) => {
                            let image_data = Box::leak(image).data.images.first().unwrap().data.clone(); 
                            let alt = if image_data.alt.len() > 0 {
                                image_data.alt.chars().map(|c| if c == 0xA as char { ' ' } else { c }).collect()
                            } else {
                                String::from("Photo")
                            };
                            
                            Some(Media::Image((image_data.fullsize, alt)))
                        },
                        atrium_api::app::bsky::feed::defs::PostViewEmbedRefs::AppBskyEmbedExternalView(external) => {
                            let external_data = Box::leak(external).data.external.clone();
                            Some(Media::External((external_data.uri.clone(), external_data.description.clone())))
                        },
                        atrium_api::app::bsky::feed::defs::PostViewEmbedRefs::AppBskyEmbedVideoView(_) => Some(Media::Video),
                        _ => None,
                    },
                    atrium_api::types::Union::Unknown(_) => None,
                }),
                replies: v.post.reply_count.unwrap_or(0) as u64,
                reposts: v.post.repost_count.unwrap_or(0) as u64,
                likes: v.post.like_count.unwrap_or(0) as u64,
            })
            .collect();
        Ok(feed)
    }

    pub async fn like<'a>(&self, uri: &'a str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
