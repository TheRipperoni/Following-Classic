use diesel::prelude::*;

#[derive(Queryable, Selectable, Clone, Debug, PartialEq, Default, Serialize, Deserialize, AsChangeset, Insertable)]
#[diesel(table_name = crate::schema::following_preference)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FollowingPreference {
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "did")]
    pub did: String,
    #[serde(rename = "show_reposts")]
    pub show_reposts: bool,
    #[serde(rename = "show_quote_posts")]
    pub show_quote_posts: bool,
}