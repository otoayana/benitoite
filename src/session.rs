use std::sync::Arc;

use atrium_api::{
    agent::{store::MemorySessionStore, AtpAgent},
    types::{LimitedNonZeroU8, Object, Unknown},
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use ipld_core::ipld::{Ipld, IpldKind};

use crate::config::Config;

pub struct Session {
    agent: Arc<AtpAgent<MemorySessionStore, ReqwestClient>>,
}

impl Session {
    pub async fn from_fingerprint<'a>(
        fingerprint: &'a str,
    ) -> Result<Session, Box<dyn std::error::Error>> {
        let config = Config::parse()?;
        println!("{:#?}", config.accounts);
        let user = config.accounts.get(fingerprint).unwrap();

        let agent = AtpAgent::new(ReqwestClient::new(&user.pds), MemorySessionStore::default());
        agent.login(&user.username, &user.password).await?;

        Ok(Session {
            agent: Arc::new(agent),
        })
    }

    pub async fn feed(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

        let feed: Vec<String> = action
            .feed
            .iter()
            .map(|v| {
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
            })
            .collect();
        Ok(feed)
    }
}
