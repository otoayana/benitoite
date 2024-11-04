use askama::Template;
use fluffer::Client;
use fluskama::FluffTemplate;

use crate::session::Session;

#[derive(Debug, Template)]
#[template(path = "feed.gmi", escape = "txt")]
pub struct Feed {
    posts: Vec<String>,
}

pub async fn feed(c: Client) -> FluffTemplate<Feed> {
    let fingerprint = c.fingerprint().unwrap();
    let session = Session::from_fingerprint(&fingerprint).await.unwrap();
    let feed = session.feed().await.unwrap();

    FluffTemplate::from(Feed { posts: feed })
}
