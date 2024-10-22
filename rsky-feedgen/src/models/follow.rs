use crate::schema::follow;
use diesel::backend::Backend;
use diesel::deserialize::{self, Queryable};
use diesel::prelude::Selectable;

type DB = diesel::pg::Pg;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Follow {
    #[serde(rename = "uri")]
    pub uri: String,
    #[serde(rename = "cid")]
    pub cid: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "indexedAt")]
    pub indexed_at: String,
    #[serde(rename = "prev", skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    #[serde(rename = "sequence")]
    pub sequence: Option<i64>,
}

impl Queryable<follow::SqlType, DB> for Follow {
    type Row = (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<i64>,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(Follow {
            uri: row.0,
            cid: row.1,
            author: row.2,
            subject: row.3,
            created_at: row.4,
            indexed_at: row.5,
            prev: row.6,
            sequence: row.7,
        })
    }
}

impl<DB> Selectable<DB> for Follow
where
    DB: Backend,
{
    type SelectExpression = (
        follow::uri,
        follow::cid,
        follow::author,
        follow::subject,
        follow::createdAt,
        follow::indexedAt,
        follow::prev,
        follow::sequence,

    );

    fn construct_selection() -> Self::SelectExpression {
        (
            follow::uri,
            follow::cid,
            follow::author,
            follow::subject,
            follow::createdAt,
            follow::indexedAt,
            follow::prev,
            follow::sequence,
        )
    }
}
