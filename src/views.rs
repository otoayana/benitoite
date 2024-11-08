use crate::{
    state::State,
    types::{Media, Post, PostContext},
};
use askama::Template;
use fluffer::Fluff;
use fluskama::FluffTemplate;

type Client = fluffer::Client<State>;

#[derive(Debug, Template)]
#[template(path = "feed.gmi", escape = "txt")]
pub struct Feed {
    posts: Vec<Post>,
}

pub async fn feed<'a>(c: Client) -> FluffTemplate<Feed> {
    if let Some(fingerprint) = c.fingerprint() {
        let feed = c
            .state
            .clone()
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
        let session = c.state.clone().sessions.get(&fingerprint).unwrap().clone();

        match input.as_str() {
            "l" => session.like(&id).await.unwrap(),
            "r" => session.repost(&id).await.unwrap(),
            _ => (),
        }
    }

    Fluff::RedirectTemporary("/".to_string())
}
