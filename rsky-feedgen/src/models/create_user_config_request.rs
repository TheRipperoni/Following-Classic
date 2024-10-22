#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserConfigRequest {
    #[serde(rename = "uri")]
    pub did: String,
    #[serde(rename = "show_replies")]
    pub show_replies: bool,
    #[serde(rename = "reply_filter_likes")]
    pub reply_filter_likes: i64,
    #[serde(rename = "reply_filter_followed_only")]
    pub reply_filter_followed_only: bool,
    #[serde(rename = "show_reposts")]
    pub show_reposts: bool,
    #[serde(rename = "show_quote_posts")]
    pub show_quote_posts: bool,
}