#[derive(Debug)]
pub enum Media {
    Image((String, String)),
    External((String, String)),
    Video,
}

#[derive(Debug)]
pub struct Post {
    pub id: String,
    pub username: String,
    pub body: String,
    pub media: Option<Media>,
    pub replies: u64,
    pub reposts: u64,
    pub likes: u64,
}
