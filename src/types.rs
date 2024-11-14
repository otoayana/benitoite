use std::{collections::HashMap, ops::Deref, sync::Arc};

use askama::Template;
use atrium_api::{
    app::bsky::feed::defs::{
        FeedViewPostData, FeedViewPostReasonRefs, PostViewEmbedRefs, ReplyRefParentRefs,
    },
    com::atproto::repo::strong_ref::MainData,
    record::KnownRecord,
    types::{string::Handle, Object, TryFromUnknown, Union},
};
use blake3::Hasher;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum Media {
    Image((String, String)),
    External((String, String)),
    Quote(Quote),
    Video,
}

#[derive(Debug)]
pub struct Profile {
    pub id: Handle,
    pub name: String,
    pub bio: String,
    pub followers: u64,
    pub following: u64,
    pub posts: Vec<Post>,
}

#[derive(Debug, Template)]
#[template(path = "components/post.gmi", escape = "txt")]
pub struct Post {
    pub id: String,
    pub username: String,
    pub body: String,
    pub media: Option<Media>,
    pub replies: u64,
    pub reposts: u64,
    pub likes: u64,
    pub context: PostContext,
}

#[derive(Debug)]
pub struct Quote {
    pub author: String,
    pub body: String,
}

#[derive(Debug)]
pub enum PostContext {
    Reply(String),
    Repost(String),
    None,
}

impl Post {
    pub async fn push(
        post: &Object<FeedViewPostData>,
        objects: &Arc<Mutex<HashMap<String, MainData>>>,
    ) -> Post {
        // Create a hash to use in URIs
        let mut hasher = Hasher::new();
        hasher.update(post.post.uri.clone().as_bytes());
        let hash = hasher.finalize();

        objects.lock().await.insert(
            hash.to_string(),
            MainData {
                cid: post.post.cid.clone(),
                uri: post.post.uri.clone(),
            },
        );

        Post {
            id: hash.to_string(),
            username: post.post.author.handle.as_str().to_string(),
            body: if let KnownRecord::AppBskyFeedPost(body) =
                KnownRecord::try_from_unknown(post.post.record.clone()).unwrap()
            {
                Box::leak(body)
                    .text
                    .clone()
                    .chars()
                    .map(|v| if v == '#' { '♯' } else { v })
                    .collect::<String>()
            } else {
                String::new()
            },
            media: post.post.embed.clone().map_or(None, |v| match v {
                Union::Refs(r) => match r {
                    // TODO(otoayana): Add multiple media items
                    PostViewEmbedRefs::AppBskyEmbedImagesView(image) => {
                        let image_data = Box::leak(image).data.images.first().unwrap().data.clone();
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
                    PostViewEmbedRefs::AppBskyEmbedRecordView(quote) => {
                        if let Union::Refs(
                            atrium_api::app::bsky::embed::record::ViewRecordRefs::ViewRecord(
                                quote_rec,
                            ),
                        ) = Box::leak(quote).record.clone()
                        {
                            Some(Media::Quote(Quote {
                                author: quote_rec.author.handle.to_string(),
                                body: if let KnownRecord::AppBskyFeedPost(body) =
                                    KnownRecord::try_from_unknown(quote_rec.value.clone()).unwrap()
                                {
                                    body.text.clone()
                                } else {
                                    String::new()
                                },
                            }))
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                Union::Unknown(_) => None,
            }),
            replies: post.post.reply_count.unwrap_or(0) as u64,
            reposts: post.post.repost_count.unwrap_or(0) as u64,
            likes: post.post.like_count.unwrap_or(0) as u64,
            context: if let Some(Union::Refs(FeedViewPostReasonRefs::ReasonRepost(r))) =
                post.reason.clone()
            {
                PostContext::Repost(r.deref().by.handle.to_string())
            } else {
                if let Some(Union::Refs(ReplyRefParentRefs::PostView(reply))) =
                    post.reply.clone().map(|v| v.parent.clone())
                {
                    PostContext::Reply(reply.author.handle.to_string())
                } else {
                    PostContext::None
                }
            },
        }
    }
}
