use diesel::prelude::*;

#[derive(
    Queryable, Selectable, Clone, Debug, PartialEq, Default, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = crate::schema::seen_post)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SeenPost {
    #[serde(rename = "did")]
    pub did: String,
    #[serde(rename = "uri")]
    pub uri: String,
}
