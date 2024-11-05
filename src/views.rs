use crate::{
    state::State,
    types::{Media, Post},
};
use askama::Template;
use fluffer::Fluff;
use fluskama::FluffTemplate;
use std::sync::Arc;
use tokio::sync::Mutex;

type Client = fluffer::Client<Arc<Mutex<State>>>;

#[derive(Debug, Template)]
#[template(path = "feed.gmi", escape = "txt")]
pub struct Feed {
    posts: Vec<Post>,
}

pub async fn feed<'a>(c: Client) -> FluffTemplate<Feed> {
    let state = c.state.lock().await;
    if let Some(fingerprint) = c.fingerprint() {
        let feed = state
            .sessions
            .get(&fingerprint)
            .unwrap()
            .clone()
            .feed()
            .await
            .unwrap();

        FluffTemplate::from(Feed { posts: feed })
    } else {
        FluffTemplate::from(Feed { posts: Vec::new() })
    }
}

pub async fn interact(c: Client) -> Fluff {
    let id = c.parameter("id").unwrap();

    let Some(input) = c.input() else {
        return Fluff::Input("usage: \"l\" to like, \"r\" to repost, \"R\" to reply".to_string());
    };

    if let Some(fingerprint) = c.fingerprint() {
        let state = c.state.lock().await;
        let session = state.sessions.get(&fingerprint).unwrap().clone();

        match input.as_str() {
            "l" => session.like(&id).await.unwrap(),
            "r" => session.repost(&id).await.unwrap(),
            _ => (),
        }
    }

    Fluff::RedirectTemporary("/".to_string())
}
