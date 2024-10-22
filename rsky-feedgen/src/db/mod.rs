use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use crate::models::{Follow, FollowingPreference, UserFeedPreference};
use crate::{ReadReplicaConn, WriteDbConn};
use crate::schema::following_preference::dsl::following_preference;
use crate::schema::sub_state::{cursor, service};
use crate::schema::user_feed_preference::dsl::user_feed_preference;

pub fn establish_connection() -> Result<PgConnection, Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap_or("".into());
    let result = PgConnection::establish(&database_url).map_err(|_| {
        eprintln!("Error connecting to {database_url:?}");
        "Internal error"
    })?;

    Ok(result)
}

pub fn get_user_config(_did: &str, conn: &mut PgConnection) -> Option<UserFeedPreference> {
    use crate::schema::user_feed_preference::dsl::*;

    let result = user_feed_preference
        .filter(did.eq(_did))
        .limit(1)
        .select(UserFeedPreference::as_select())
        .load(conn)
        .expect("Error querying user feed");

    if result.len() > 0 {
        Some(result[0].clone())
    } else {
        None
    }
}

pub async fn get_saved_follows(did: String, connection: &ReadReplicaConn) -> Vec<String> {
    use crate::schema::follow::dsl::*;
    let mut follows = Vec::new();

    let result = connection
        .run(move |conn| {
            let result = follow
                .filter(author.eq(did))
                .select(Follow::as_select())
                .load(conn)
                .expect("Error querying follows");
            result
        })
        .await;

    for follow2 in result.iter() {
        follows.push(follow2.subject.clone());
    }
    follows
}

pub async fn get_following_preferences(did: String, connection: &ReadReplicaConn) -> Vec<String> {
    use crate::schema::follow::dsl::*;
    let mut follows = Vec::new();

    let result = connection
        .run(move |conn| {
            let result = follow
                .filter(author.eq(did))
                .select(Follow::as_select())
                .load(conn)
                .expect("Error querying follows");
            result
        })
        .await;

    for follow2 in result.iter() {
        follows.push(follow2.subject.clone());
    }
    follows
}

pub fn user_follows_indexed(did: &str, conn: &mut PgConnection) -> bool {
    use crate::schema::follow::dsl::*;

    let mut follows: Vec<Follow> = Vec::new();

    follows = follow
        .filter(author.eq(did))
        .limit(1)
        .select(Follow::as_select())
        .load(conn)
        .expect("Error querying follows");

    follows.len() > 0
}

pub async fn user_config_creation(
    config: UserFeedPreference,
    connection: WriteDbConn,
) -> Result<(), String> {
    use crate::schema::user_feed_preference::dsl as UserFeedSchema;

    let new_config = (
        UserFeedSchema::did.eq(config.did),
        UserFeedSchema::reply_filter_likes.eq(config.reply_filter_likes),
        UserFeedSchema::reply_filter_followed_only.eq(config.reply_filter_followed_only),
        UserFeedSchema::show_quote_posts.eq(config.show_quote_posts),
        UserFeedSchema::show_replies.eq(config.show_replies),
        UserFeedSchema::show_reposts.eq(config.show_reposts),
    );
    let result = connection
        .run(move |conn| {
            diesel::insert_into(UserFeedSchema::user_feed_preference)
                .values(&new_config)
                .execute(conn)
                .expect("Error inserting member records");
        })
        .await;
    Ok(result)
}

pub fn get_following_preferences2(
    _did: String,
    conn: &mut PgConnection,
) -> Vec<FollowingPreference> {
    use crate::schema::following_preference::dsl::following_preference as FollowingPrefSchema;
    use crate::schema::following_preference::dsl::author;
    FollowingPrefSchema.filter(author.eq(_did))
        .select(FollowingPreference::as_select())
        .load(conn)
        .unwrap()
}

pub async fn following_pref_fetch(
    _did: String,
    connection: WriteDbConn,
) -> Vec<FollowingPreference> {
    use crate::schema::following_preference::dsl::following_preference as FollowingPrefSchema;
    use crate::schema::following_preference::dsl::author;

    let result = connection
        .run(move |conn| {
            FollowingPrefSchema.filter(author.eq(_did))
                .select(FollowingPreference::as_select())
                .load(conn)
                .unwrap()
        })
        .await;
    result
}

pub async fn following_pref_update(
    _following_preference: FollowingPreference,
    connection: WriteDbConn,
) -> Result<(), String> {
    use crate::schema::following_preference::author;
    use crate::schema::following_preference::did;
    let result = connection
        .run(move |conn| {
            diesel::insert_into(following_preference)
                .values(&_following_preference)
                .on_conflict((author, did))
                .do_update()
                .set(&_following_preference)
                .execute(conn)
                .expect("Error update config records");
        })
        .await;
    Ok(result)
}


pub async fn user_config_fetch(
    _did: String,
    connection: WriteDbConn,
) -> Vec<UserFeedPreference> {
    use crate::schema::user_feed_preference::dsl::user_feed_preference as UserFeedSchema;
    use crate::schema::user_feed_preference::dsl::did;

    let result = connection
        .run(move |conn| {
            UserFeedSchema.filter(did.eq(_did))
                .select(UserFeedPreference::as_select())
                .load(conn)
                .unwrap()
        })
        .await;
    result
}

pub async fn user_config_update(
    config: UserFeedPreference,
    connection: WriteDbConn,
) -> Result<(), String> {
    let result = connection
        .run(move |conn| {
            diesel::update(user_feed_preference)
                .set(config)
                .execute(conn)
                .expect("Error update config records");
        })
        .await;
    Ok(result)
}

pub fn insert_follows(follows: Vec<Follow>, conn: &mut PgConnection) {
    use crate::schema::follow::dsl as FollowSchema;
    let mut follows_to_insert = Vec::new();
    for follow in follows.iter() {
        let new_follow = (
            FollowSchema::uri.eq(follow.uri.clone()),
            FollowSchema::cid.eq(follow.cid.clone()),
            FollowSchema::author.eq(follow.author.clone()),
            FollowSchema::subject.eq(follow.subject.clone()),
            FollowSchema::createdAt.eq(follow.created_at.clone()),
            FollowSchema::indexedAt.eq(follow.indexed_at.clone()),
            FollowSchema::prev.eq(follow.prev.clone()),
            FollowSchema::sequence.eq(follow.sequence.clone()),
        );
        follows_to_insert.push(new_follow);
    }

    diesel::insert_into(crate::schema::follow::dsl::follow)
        .values(&follows_to_insert)
        .on_conflict(FollowSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting follow records");
}

pub fn delete_posts_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(crate::schema::post::dsl::post.filter(crate::schema::post::dsl::uri.eq_any(delete_rows)))
        .execute(conn)
        .expect("Error deleting post records");
}

pub fn delete_reposts_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(crate::schema::repost::dsl::repost.filter(crate::schema::repost::dsl::uri.eq_any(delete_rows)))
        .execute(conn)
        .expect("Error deleting repost records");
}

pub fn delete_follows_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(crate::schema::follow::dsl::follow.filter(crate::schema::follow::dsl::uri.eq_any(delete_rows)))
        .execute(conn)
        .expect("Error deleting follow records");
}

pub fn delete_likes_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(crate::schema::like::dsl::like.filter(crate::schema::like::dsl::uri.eq_any(delete_rows)))
        .execute(conn)
        .expect("Error deleting like records");
}

pub struct CursorUpdateState {
    pub service: String,
    pub cursor: i64,
}

pub fn update_cursor_db(update_state: CursorUpdateState, conn: &mut PgConnection) {
    use crate::schema::sub_state::dsl::*;

    let new_update_state = (service.eq(update_state.service), cursor.eq(update_state.cursor));

    diesel::insert_into(sub_state)
        .values(&new_update_state)
        .on_conflict(service)
        .do_update()
        .set(cursor.eq(update_state.cursor))
        .execute(conn)
        .expect("Error updating cursor records");
}