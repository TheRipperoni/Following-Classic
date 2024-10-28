use diesel::prelude::*;

#[derive(
    Queryable, Selectable, Clone, Debug, PartialEq, Default, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = crate::schema::user_feed_preference)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserFeedPreference {
    #[serde(rename = "did")]
    pub did: String,
    #[serde(rename = "show_replies")]
    pub show_replies: bool,
    #[serde(rename = "reply_filter_likes")]
    pub reply_filter_likes: i32,
    #[serde(rename = "reply_filter_followed_only")]
    pub reply_filter_followed_only: bool,
    #[serde(rename = "show_reposts")]
    pub show_reposts: bool,
    #[serde(rename = "show_quote_posts")]
    pub show_quote_posts: bool,
    #[serde(rename = "hide_seen_posts")]
    pub hide_seen_posts: bool,
    #[serde(rename = "hide_no_alt_text")]
    pub hide_no_alt_text: bool,
}
