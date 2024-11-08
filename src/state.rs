use crate::{config::Config, session::Session};
use atrium_api::com::atproto::repo::strong_ref::MainData;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Clone)]
pub struct State {
    pub sessions: Arc<HashMap<String, Session>>,
}

impl State {
    pub async fn init(config: &Config) -> Result<State, Box<dyn std::error::Error>> {
        let objects: Arc<Mutex<HashMap<String, MainData>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut sessions: HashMap<String, Session> = HashMap::new();

        for (fingerprint, account) in &config.accounts {
            let session = Session::new(&account, objects.clone()).await?;
            debug!("session spawned for user @{}", &session.handle);
            sessions.insert(fingerprint.clone().to_lowercase(), session);
        }

        info!("{} sessions spawned", sessions.len());

        Ok(State {
            sessions: Arc::new(sessions),
        })
    }
}
