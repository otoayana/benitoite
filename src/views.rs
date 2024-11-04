use askama::Template;
use fluffer::{Client, Fluff};
use fluskama::FluffTemplate;

use crate::{
    session::Session,
    types::{Media, Post},
};

#[derive(Debug, Template)]
#[template(path = "feed.gmi", escape = "txt")]
pub struct Feed {
    posts: Vec<Post>,
}

pub async fn feed(c: Client) -> FluffTemplate<Feed> {
    if let Some(fingerprint) = c.fingerprint() {
        let session = Session::from_fingerprint(&fingerprint).await.unwrap();
        let feed = session.feed().await.unwrap();

        FluffTemplate::from(Feed { posts: feed })
    } else {
        FluffTemplate::from(Feed { posts: Vec::new() })
    }
}

pub async fn interact(c: Client) -> Fluff {
    let did = c.parameter("did").unwrap();
    let at_type = c.parameter("type").unwrap();
    let id = c.parameter("id").unwrap();

    let Some(input) = c.input() else {
        return Fluff::Input("usage: \"l\" to like, \"r\" to repost, \"R\" to reply".to_string());
    };

    if let Some(fingerprint) = c.fingerprint() {
        let session = Session::from_fingerprint(&fingerprint).await.unwrap();

        match input.as_str() {
            "l" => {
                let uri = format!("at://{}/{}/{}", did, at_type, id);
                dbg!(&uri);
                session.like(&uri).await.unwrap()
            }
            _ => (),
        }
    }

    Fluff::RedirectTemporary("/".to_string())
}
