use atrium_api::com::atproto::repo::strong_ref::MainData;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{config::Config, session::Session};

pub struct State {
    pub sessions: Arc<HashMap<String, Session>>,
}

impl State {
    pub async fn init(config: &Config) -> Result<State, Box<dyn std::error::Error>> {
        let objects: Arc<Mutex<HashMap<String, MainData>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut sessions: HashMap<String, Session> = HashMap::new();
        for (fingerprint, account) in &config.accounts {
            sessions.insert(
                fingerprint.clone(),
                Session::new(&account, objects.clone()).await?,
            );
        }

        Ok(State {
            sessions: Arc::new(sessions),
        })
    }
}
