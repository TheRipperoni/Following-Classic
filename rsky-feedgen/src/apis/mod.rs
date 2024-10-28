use crate::agent::{get_agent, get_follows};
use crate::db::*;
use crate::models::post_result::PostResultReason;
use crate::models::*;
use crate::schema::follow::dsl as FollowSchema;
use crate::schema::user_feed_preference::dsl as UserFeedSchema;
use crate::schema::user_feed_preference::dsl::user_feed_preference;
use crate::{ReadReplicaConn, WriteDbConn};
use chrono::offset::Utc as UtcOffset;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use rsky_lexicon::app::bsky::embed::Embeds;
use std::collections::HashSet;
use std::fmt::Write;
use std::time::SystemTime;

const SHOW_REPLIES_FOR_FOLLOWING_ONLY: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyouhr7z26";
const DONT_SHOW_REPOSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyptviqu2f";
const DONT_SHOW_QUOTEPOSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyqh7fbr26";
const NUMBER_OF_LIKES: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyvglu472z";
const RESET_PREF: &str = "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5g74kd7my26";
const HIDE_SEEN_POSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l7edu2ufdp2u";
const HIDE_NOT_ALT_TEXT_POSTS: &str = "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3lbsxswsgus2f";
const USER_PREF_OPTIONS: [&str; 6] = [
    RESET_PREF,
    DONT_SHOW_QUOTEPOSTS,
    DONT_SHOW_REPOSTS,
    SHOW_REPLIES_FOR_FOLLOWING_ONLY,
    HIDE_SEEN_POSTS,
    HIDE_NOT_ALT_TEXT_POSTS
];

fn update_seen_posts(did: &str, conn: &mut PgConnection) {
    println!("Checking if seen posts should be updated");
    let fetched_posts = get_fetched_posts(did, conn);
    insert_seen_posts(fetched_posts.clone(), conn);

    let mut uri_list: Vec<String> = Vec::new();
    for fetched_post in fetched_posts {
        uri_list.push(fetched_post.uri);
    }

    invalidate_fetched_posts(did, uri_list, conn);
}

fn post_media_query_str(
    following: &str,
) -> String {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
       \"media\",
\"alt\"
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             p1.\"indexedAt\",
p1.\"media\",
p1.\"alt\"
      from post p1
      where p1.author in ({authors})
        and (p1.media is true)
      group by p1.uri, p1.cid, p1.author) as x
where true=true",
            authors = following,
        )
}

fn post_query_str(
    hide_seen_posts: bool,
    hide_no_alt_text: bool,
    following: &str,
    user_config: &UserFeedPreference,
    did: &str,
) -> String {
    if hide_seen_posts {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
       \"media\",
\"alt\"
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             (select count(*) from public.like m where p1.uri = m.\"subjectUri\") as likeCount,
             p1.\"indexedAt\",
p1.\"media\",
p1.\"alt\"
      from post p1
               left join post p2
                         on p1.\"replyParent\" = p2.uri
               LEFT OUTER JOIN seen_post s1 ON s1.did = '{did}' and s1.uri = p1.uri
      where p1.author in ({authors})
        and ({quotes_included} or p1.\"quoteUri\" is null)
        and ({hide_no_alt_text}=false or p1.\"media\" is false or p1.\"alt\" is not null)
        and ({replies_included} or p1.\"replyParent\" is null)
        and s1.id is null
        and ({all_replies} or p2.author is null or (p2.author in ({authors})))
      group by p1.uri, p1.cid, p1.author) as x
where (\"replyParent\" is null or likeCount >= {like_threshold})",
            authors = following,
            quotes_included = user_config.show_quote_posts,
            replies_included = user_config.show_replies,
            all_replies = !user_config.reply_filter_followed_only,
            like_threshold = user_config.reply_filter_likes,
            did = did
        )
    } else {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
        \"media\",
        alt
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             (select count(*) from public.like m where p1.uri = m.\"subjectUri\") as likeCount,
             p1.\"indexedAt\",
            p1.\"media\",
p1.\"alt\"
      from post p1
               left join post p2
                         on p1.\"replyParent\" = p2.uri
      where p1.author in ({authors})
        and ({quotes_included} or p1.\"quoteUri\" is null)
        and ({hide_no_alt_text}=false or p1.\"media\" is false or p1.\"alt\" is not null)
        and ({replies_included} or p1.\"replyParent\" is null)
        and ({all_replies} or p2.author is null or (p2.author in ({authors})))
      group by p1.uri, p1.cid, p1.author) as x
where (\"replyParent\" is null or likeCount >= {like_threshold})",
            authors = following,
            quotes_included = user_config.show_quote_posts,
            replies_included = user_config.show_replies,
            all_replies = !user_config.reply_filter_followed_only,
            like_threshold = user_config.reply_filter_likes
        )
    }
}



fn repost_query_str(hide_seen_posts: bool, following_reposts_string: &str, did: &str) -> String {
    if hide_seen_posts {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       null   as \"replyParent\",
       null   as \"replyRoot\",
       prev,
       \"sequence\",
       null   as \"text\",
       null   as lang,
       author as author,
       null   as \"externalUri\",
       null   as \"externalTitle\",
       null   as \"externalDescription\",
       null   as \"externalThumb\",
       \"subjectCid\"   as \"quoteCid\",
       \"subjectUri\"   as \"quoteUri\",
false   as \"media\",
null as alt
from (select r1.uri as uri,
             r1.cid as cid,
             r1.\"subjectUri\" as \"subjectUri\",
             r1.\"subjectCid\" as \"subjectCid\",
             r1.author,
             r1.\"indexedAt\",
             r1.prev,
             r1.\"sequence\"
      from repost r1
          LEFT OUTER JOIN seen_post s1 ON s1.did = '{did}' and s1.uri = r1.uri
      where r1.author in ({authors}) and s1.id is null) as x",
            authors = following_reposts_string,
            did = did.clone()
        )
    } else {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       null   as \"replyParent\",
       null   as \"replyRoot\",
       prev,
       \"sequence\",
       null   as \"text\",
       null   as lang,
       author as author,
       null   as \"externalUri\",
       null   as \"externalTitle\",
       null   as \"externalDescription\",
       null   as \"externalThumb\",
       \"subjectCid\"   as \"quoteCid\",
       \"subjectUri\"   as \"quoteUri\",
false   as \"media\",
null as alt
from (select r1.uri as uri,
             r1.cid as cid,
             r1.\"subjectUri\" as \"subjectUri\",
             r1.\"subjectCid\" as \"subjectCid\",
             r1.author,
             r1.\"indexedAt\",
             r1.prev,
             r1.\"sequence\"
      from repost r1
      where r1.author in ({authors})
      ) as x",
            authors = following_reposts_string
        )
    }
}

#[tracing::instrument(skip(connection))]
pub async fn get_posts_by_user_feed(
    did: String,
    _limit: Option<i64>,
    params_cursor: Option<&str>,
    connection: ReadReplicaConn,
) -> Result<AlgoResponse, ValidationErrorMessageResponse> {
    let limit: i64 = _limit.unwrap_or(30);
    let params_cursor = match params_cursor {
        None => None,
        Some(params_cursor) => Some(params_cursor.to_string()),
    };
    let mut following = String::from("");

    let mut follow_dids = get_saved_follows(did.clone(), &connection).await;
    if follow_dids.len() == 0 {
        tracing::info!("Creating followers for {}", did);
        let agent = get_agent().await.unwrap();
        let follows = get_follows(&agent, did.clone().as_ref()).await;
        connection
            .run(move |conn| {
                insert_follows(follows, conn);
            })
            .await;
        follow_dids = get_saved_follows(did.clone(), &connection).await;
    }

    if follow_dids.len() == 0 {
        return Ok(AlgoResponse {
            cursor: None,
            feed: Vec::new(),
        });
    }

    for follow_did in follow_dids.iter() {
        following += ("\'".to_string() + follow_did.as_str() + "\',").as_str();
    }
    following.pop();

    let result = connection
        .run(move |conn| {
            let user_config = match get_user_config(did.clone().as_ref(), conn) {
                Some(config) => config,
                None => UserFeedPreference {
                    did: did.clone().to_string(),
                    show_replies: true,
                    reply_filter_likes: 0,
                    reply_filter_followed_only: false,
                    show_reposts: true,
                    show_quote_posts: true,
                    hide_seen_posts: false,
                    hide_no_alt_text: false,
                },
            };

            if user_config.hide_seen_posts && limit != 1 {
                match params_cursor {
                    None => invalidate_all_fetched_posts(did.as_str(), conn),
                    Some(_) => {
                        if get_total_fetches(did.as_str(), conn) >= 60 {
                            update_seen_posts(did.as_str(), conn)
                        }
                    }
                }
            }

            let following_preferences = get_following_preferences2(did.clone(), conn);
            let mut following_reposts: HashSet<String> = follow_dids.iter().cloned().collect();
            let mut following_reposts_string = String::from("");
            for following_preference in following_preferences {
                if !following_preference.show_reposts {
                    following_reposts.remove(following_preference.did.as_str());
                }
            }
            for following_pref_did in following_reposts.iter() {
                following_reposts_string +=
                    ("\'".to_string() + following_pref_did.as_str() + "\',").as_str();
            }
            following_reposts_string.pop();

            let mut query_str: String = post_query_str(
                user_config.hide_seen_posts,
                user_config.hide_no_alt_text,
                following.as_str(),
                &user_config,
                did.as_str(),
            );
            let mut repost_query_str: String = repost_query_str(
                user_config.hide_seen_posts,
                following_reposts_string.as_str(),
                did.as_str(),
            );

            if params_cursor.is_some() {
                let cursor_str = params_cursor.unwrap();
                let v = cursor_str
                    .split("::")
                    .take(2)
                    .map(String::from)
                    .collect::<Vec<_>>();
                if let [indexed_at_c, _cid_c] = &v[..] {
                    if let Ok(timestamp) = indexed_at_c.parse::<i64>() {
                        let nanoseconds = 230 * 1000000;
                        let datetime = DateTime::<Utc>::from_utc(
                            NaiveDateTime::from_timestamp(timestamp / 1000, nanoseconds),
                            Utc,
                        );
                        let mut timestr = String::new();
                        match write!(timestr, "{}", datetime.format("%+")) {
                            Ok(_) => {
                                let cursor_filter_str =
                                    format!(" AND (\"indexedAt\" < '{0}')", timestr.to_owned());
                                query_str = format!("{}{}", query_str, cursor_filter_str);
                                let cursor_repost_filter_str =
                                    format!(" WHERE (\"indexedAt\" < '{0}')", timestr.to_owned());
                                repost_query_str =
                                    format!("{}{}", repost_query_str, cursor_repost_filter_str);
                            }
                            Err(error) => tracing::error!("Error formatting: {error:?}"),
                        }
                    }
                } else {
                    let validation_error = ValidationErrorMessageResponse {
                        code: Some(ErrorCode::ValidationError),
                        message: Some("malformed cursor".into()),
                    };
                    return Err(validation_error);
                }
            }
            let order_str = format!(" ORDER BY \"indexedAt\" DESC, cid DESC LIMIT {} ", limit);
            let query_str = format!("{}{};", &query_str, &order_str);
            let repost_query_str = format!("{}{};", &repost_query_str, &order_str);

            let mut results = sql_query(query_str)
                .load::<crate::models::Post>(conn)
                .expect("Error loading post records");

            if user_config.show_reposts {
                let mut repost_results = sql_query(repost_query_str)
                    .load::<crate::models::Post>(conn)
                    .expect("Error loading post records");
                results.append(&mut repost_results);
                results.sort_by(|a, b| {
                    let fmt = "%+";
                    let a_date = NaiveDateTime::parse_from_str(a.indexed_at.as_str(), fmt).unwrap();
                    let b_date = NaiveDateTime::parse_from_str(b.indexed_at.as_str(), fmt).unwrap();
                    b_date
                        .and_utc()
                        .timestamp()
                        .cmp(&a_date.and_utc().timestamp())
                });
            }

            let mut slice_size = limit as usize;
            if results.len() < 30 {
                slice_size = results.len();
            }
            let mut final_result = Vec::new();

            for i in 0..slice_size {
                final_result.push(results[i].clone());
            }

            let mut post_results = Vec::new();
            let mut cursor: Option<String> = None;

            if let Some(last_post) = final_result.last() {
                if let Ok(parsed_time) = NaiveDateTime::parse_from_str(&last_post.indexed_at, "%+")
                {
                    cursor = Some(format!(
                        "{}::{}",
                        parsed_time.timestamp_millis(),
                        last_post.cid
                    ));
                }
            }

            final_result
                .clone()
                .into_iter()
                .map(|result| {
                    let post_result;
                    if result.quote_uri.is_some() {
                        let reason = PostResultReason {
                            reason_type: "app.bsky.feed.defs#skeletonReasonRepost".to_string(),
                            repost_uri: result.uri,
                        };
                        post_result = PostResult {
                            post: result.quote_uri.unwrap(),
                            reason: Some(reason),
                        };
                    } else {
                        post_result = PostResult {
                            post: result.uri,
                            reason: None,
                        };
                    }
                    post_results.push(post_result);
                })
                .for_each(drop);

            if user_config.hide_seen_posts && limit != 1 {
                let mut fetched_posts: Vec<FetchedPost> = Vec::new();
                for post_result in final_result.clone() {
                    let fetched_post = FetchedPost {
                        did: did.clone(),
                        uri: post_result.uri,
                    };
                    fetched_posts.push(fetched_post);
                }
                insert_fetched_posts(fetched_posts, conn);
            }

            let new_response = AlgoResponse {
                cursor,
                feed: post_results,
            };
            Ok(new_response)
        })
        .await;
    result
}

#[tracing::instrument(skip(connection))]
pub async fn get_posts_by_following_media(
    did: String,
    _limit: Option<i64>,
    params_cursor: Option<&str>,
    connection: ReadReplicaConn,
) -> Result<AlgoResponse, ValidationErrorMessageResponse> {
    let limit: i64 = _limit.unwrap_or(30);
    let params_cursor = match params_cursor {
        None => None,
        Some(params_cursor) => Some(params_cursor.to_string()),
    };
    let mut following = String::from("");

    let mut follow_dids = get_saved_follows(did.clone(), &connection).await;
    if follow_dids.len() == 0 {
        tracing::info!("Creating followers for {}", did);
        let agent = get_agent().await.unwrap();
        let follows = get_follows(&agent, did.clone().as_ref()).await;
        connection
            .run(move |conn| {
                insert_follows(follows, conn);
            })
            .await;
        follow_dids = get_saved_follows(did.clone(), &connection).await;
    }

    if follow_dids.len() == 0 {
        return Ok(AlgoResponse {
            cursor: None,
            feed: Vec::new(),
        });
    }

    for follow_did in follow_dids.iter() {
        following += ("\'".to_string() + follow_did.as_str() + "\',").as_str();
    }
    following.pop();

    let result = connection
        .run(move |conn| {
            let mut query_str: String = post_media_query_str(
                following.as_str(),
            );

            if params_cursor.is_some() {
                let cursor_str = params_cursor.unwrap();
                let v = cursor_str
                    .split("::")
                    .take(2)
                    .map(String::from)
                    .collect::<Vec<_>>();
                if let [indexed_at_c, _cid_c] = &v[..] {
                    if let Ok(timestamp) = indexed_at_c.parse::<i64>() {
                        let nanoseconds = 230 * 1000000;
                        let datetime = DateTime::<Utc>::from_utc(
                            NaiveDateTime::from_timestamp(timestamp / 1000, nanoseconds),
                            Utc,
                        );
                        let mut timestr = String::new();
                        match write!(timestr, "{}", datetime.format("%+")) {
                            Ok(_) => {
                                let cursor_filter_str =
                                    format!(" AND (\"indexedAt\" < '{0}')", timestr.to_owned());
                                query_str = format!("{}{}", query_str, cursor_filter_str);
                            }
                            Err(error) => tracing::error!("Error formatting: {error:?}"),
                        }
                    }
                } else {
                    let validation_error = ValidationErrorMessageResponse {
                        code: Some(ErrorCode::ValidationError),
                        message: Some("malformed cursor".into()),
                    };
                    return Err(validation_error);
                }
            }
            let order_str = format!(" ORDER BY \"indexedAt\" DESC, cid DESC LIMIT {} ", limit);
            let query_str = format!("{}{};", &query_str, &order_str);

            let mut results = sql_query(query_str)
                .load::<crate::models::Post>(conn)
                .expect("Error loading post records");

            let mut post_results = Vec::new();
            let mut cursor: Option<String> = None;

            if let Some(last_post) = results.last() {
                if let Ok(parsed_time) = NaiveDateTime::parse_from_str(&last_post.indexed_at, "%+")
                {
                    cursor = Some(format!(
                        "{}::{}",
                        parsed_time.timestamp_millis(),
                        last_post.cid
                    ));
                }
            }

            results
                .clone()
                .into_iter()
                .map(|result| {
                    let post_result;
                    if result.quote_uri.is_some() {
                        let reason = PostResultReason {
                            reason_type: "app.bsky.feed.defs#skeletonReasonRepost".to_string(),
                            repost_uri: result.uri,
                        };
                        post_result = PostResult {
                            post: result.quote_uri.unwrap(),
                            reason: Some(reason),
                        };
                    } else {
                        post_result = PostResult {
                            post: result.uri,
                            reason: None,
                        };
                    }
                    post_results.push(post_result);
                })
                .for_each(drop);

            let new_response = AlgoResponse {
                cursor,
                feed: post_results,
            };
            Ok(new_response)
        })
        .await;
    result
}

fn queue_post_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::post::dsl as PostSchema;
    use crate::schema::user_feed_preference::dsl as UserFeedSchema;

    let mut new_posts = Vec::new();

    body.into_iter()
        .map(|req| {
            let system_time = SystemTime::now();
            let dt: DateTime<UtcOffset> = system_time.into();
            let mut post_text_original = String::new();
            let mut post_media_original = false;
            let mut post_alt_original = None;
            let mut new_post = Post {
                uri: req.uri,
                cid: req.cid,
                reply_parent: None,
                reply_root: None,
                indexed_at: format!("{}", dt.format("%+")),
                prev: req.prev,
                sequence: req.sequence,
                text: None,
                lang: None,
                author: req.author.clone(),
                external_uri: None,
                external_title: None,
                external_description: None,
                external_thumb: None,
                quote_cid: None,
                quote_uri: None,
                media: false,
                alt: None,
            };

            if let Lexicon::AppBskyFeedPost(post_record) = req.record {
                post_text_original = post_record.text.clone();
                if let Some(reply) = post_record.reply {
                    new_post.reply_parent = Some(reply.parent.uri);
                    new_post.reply_root = Some(reply.root.uri);
                }
                if let Some(langs) = post_record.langs {
                    new_post.lang = Some(langs.join(","));
                }
                if let Some(embed) = post_record.embed {
                    match embed {
                        Embeds::Images(e) => {
                            post_media_original = true;
                            for image in e.images {
                                if image.alt != "" {
                                    post_alt_original = Some(image.alt);
                                }
                            }
                        }
                        Embeds::Video(e) => {
                            post_media_original = true;
                            post_alt_original =  e.alt;
                        }
                        Embeds::RecordWithMedia(e) => {}
                        Embeds::External(e) => {
                            new_post.external_uri = Some(e.external.uri);
                            new_post.external_title = Some(e.external.title);
                            new_post.external_description = Some(e.external.description);
                            if let Some(thumb_blob) = e.external.thumb {
                                if let Some(thumb_cid) = thumb_blob.cid {
                                    new_post.external_thumb = Some(thumb_cid);
                                };
                            };
                        }
                        Embeds::Record(e) => {
                            new_post.quote_cid = Some(e.record.cid);
                            new_post.quote_uri = Some(e.record.uri);
                        }
                    }
                }
            }

            new_post.text = Some(post_text_original);
            new_post.media = post_media_original;
            new_post.alt = post_alt_original;

            match new_post.reply_parent {
                None => {}
                Some(ref reply_parent) => {
                    if reply_parent == NUMBER_OF_LIKES {
                        let mut new_user_prefs = Vec::new();
                        let result = get_user_config(req.author.as_str(), conn);
                        match result {
                            None => {
                                let new_user_pref = (
                                    UserFeedSchema::did.eq(req.author.clone()),
                                    UserFeedSchema::show_replies.eq(true),
                                    UserFeedSchema::reply_filter_likes.eq(2),
                                    UserFeedSchema::reply_filter_followed_only.eq(false),
                                    UserFeedSchema::show_reposts.eq(true),
                                    UserFeedSchema::show_quote_posts.eq(true),
                                );
                                new_user_prefs.push(new_user_pref);
                                diesel::insert_into(user_feed_preference)
                                    .values(&new_user_prefs)
                                    .execute(conn)
                                    .expect("Error inserting userfeedpref records");
                            }
                            Some(user_pref) => match new_post.text {
                                None => {}
                                Some(ref text) => {
                                    let num_likes = text.parse::<i32>();
                                    match num_likes {
                                        Ok(likes) => {
                                            diesel::update(user_feed_preference)
                                                .filter(
                                                    UserFeedSchema::did.eq(user_pref.did.clone()),
                                                )
                                                .set(
                                                    (UserFeedSchema::reply_filter_likes.eq(likes),),
                                                )
                                                .execute(conn)
                                                .expect("Error update config records");
                                        }
                                        Err(_) => {}
                                    }
                                }
                            },
                        }
                    }
                }
            }

            let uri_ = &new_post.uri;
            let seq_ = &new_post.sequence;
            println!("Sequence: {seq_:?} | Uri: {uri_:?}");

            let new_post = (
                PostSchema::uri.eq(new_post.uri),
                PostSchema::cid.eq(new_post.cid),
                PostSchema::replyParent.eq(new_post.reply_parent),
                PostSchema::replyRoot.eq(new_post.reply_root),
                PostSchema::indexedAt.eq(new_post.indexed_at),
                PostSchema::prev.eq(new_post.prev),
                PostSchema::sequence.eq(new_post.sequence),
                PostSchema::text.eq(new_post.text),
                PostSchema::lang.eq(new_post.lang),
                PostSchema::author.eq(new_post.author),
                PostSchema::externalUri.eq(new_post.external_uri),
                PostSchema::externalTitle.eq(new_post.external_title),
                PostSchema::externalDescription.eq(new_post.external_description),
                PostSchema::externalThumb.eq(new_post.external_thumb),
                PostSchema::quoteCid.eq(new_post.quote_cid),
                PostSchema::quoteUri.eq(new_post.quote_uri),
                PostSchema::media.eq(new_post.media),
                PostSchema::alt.eq(new_post.alt),
            );
            new_posts.push(new_post);
        })
        .for_each(drop);

    diesel::insert_into(PostSchema::post)
        .values(&new_posts)
        .on_conflict(PostSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting post records");
}

fn queue_repost_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::repost::dsl as RepostSchema;

    let mut new_reposts = Vec::new();

    body.into_iter()
        .map(|req| {
            if let Lexicon::AppBskyFeedRepost(repost_record) = req.record {
                let system_time = SystemTime::now();
                let dt: DateTime<UtcOffset> = system_time.into();
                let new_like = (
                    RepostSchema::uri.eq(req.uri),
                    RepostSchema::cid.eq(req.cid),
                    RepostSchema::author.eq(req.author),
                    RepostSchema::subjectCid.eq(repost_record.subject.cid),
                    RepostSchema::subjectUri.eq(repost_record.subject.uri),
                    RepostSchema::createdAt.eq(repost_record.created_at),
                    RepostSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                    RepostSchema::prev.eq(req.prev),
                    RepostSchema::sequence.eq(req.sequence),
                );
                new_reposts.push(new_like);
            }
        })
        .for_each(drop);

    diesel::insert_into(RepostSchema::repost)
        .values(&new_reposts)
        .on_conflict(RepostSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting repost records");
}

fn queue_like_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::like::dsl as LikeSchema;

    let mut new_likes = Vec::new();
    let mut new_user_prefs = Vec::new();

    body.into_iter()
        .map(|req| {
            if let Lexicon::AppBskyFeedLike(like_record) = req.record {
                if USER_PREF_OPTIONS.contains(&like_record.subject.uri.as_str()) {
                    let result = get_user_config(req.author.as_str(), conn);
                    match result {
                        None => {
                            let new_user_pref = (
                                UserFeedSchema::did.eq(req.author.clone()),
                                UserFeedSchema::show_replies.eq(true),
                                UserFeedSchema::reply_filter_likes.eq(2),
                                UserFeedSchema::reply_filter_followed_only.eq(false),
                                UserFeedSchema::show_reposts.eq(true),
                                UserFeedSchema::show_quote_posts.eq(true),
                            );
                            new_user_prefs.push(new_user_pref);
                            diesel::insert_into(user_feed_preference)
                                .values(&new_user_prefs)
                                .execute(conn)
                                .expect("Error inserting userfeedpref records");
                        }
                        Some(user_pref) => match like_record.subject.uri.as_str() {
                            SHOW_REPLIES_FOR_FOLLOWING_ONLY => {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::reply_filter_followed_only.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            DONT_SHOW_REPOSTS => {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::show_reposts.eq(false),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            DONT_SHOW_QUOTEPOSTS => {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::show_quote_posts.eq(false),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            RESET_PREF => {
                                diesel::delete(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            HIDE_SEEN_POSTS => {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::hide_seen_posts.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            HIDE_NOT_ALT_TEXT_POSTS => {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::hide_no_alt_text.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                            _ => {}
                        },
                    }
                }

                let system_time = SystemTime::now();
                let dt: DateTime<UtcOffset> = system_time.into();
                let new_like = (
                    LikeSchema::uri.eq(req.uri),
                    LikeSchema::cid.eq(req.cid),
                    LikeSchema::author.eq(req.author),
                    LikeSchema::subjectCid.eq(like_record.subject.cid),
                    LikeSchema::subjectUri.eq(like_record.subject.uri),
                    LikeSchema::createdAt.eq(like_record.created_at),
                    LikeSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                    LikeSchema::prev.eq(req.prev),
                    LikeSchema::sequence.eq(req.sequence),
                );
                new_likes.push(new_like);
            }
        })
        .for_each(drop);

    diesel::insert_into(LikeSchema::like)
        .values(&new_likes)
        .on_conflict(LikeSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting like records");
}

fn queue_follow_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    let mut new_follows = Vec::new();

    body.into_iter()
        .map(|req| {
            if user_follows_indexed(req.author.as_str(), conn) {
                if let Lexicon::AppBskyFeedFollow(follow_record) = req.record {
                    let system_time = SystemTime::now();
                    let dt: DateTime<UtcOffset> = system_time.into();
                    let new_follow = (
                        FollowSchema::uri.eq(req.uri),
                        FollowSchema::cid.eq(req.cid),
                        FollowSchema::author.eq(req.author),
                        FollowSchema::subject.eq(follow_record.subject),
                        FollowSchema::createdAt.eq(follow_record.created_at),
                        FollowSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                        FollowSchema::prev.eq(req.prev),
                        FollowSchema::sequence.eq(req.sequence),
                    );
                    new_follows.push(new_follow);
                }
            }
        })
        .for_each(drop);

    if new_follows.len() > 0 {
        diesel::insert_into(FollowSchema::follow)
            .values(&new_follows)
            .on_conflict(FollowSchema::uri)
            .do_nothing()
            .execute(conn)
            .expect("Error inserting follow records");
    }
}

pub async fn queue_creation(
    lex: String,
    body: Vec<CreateRequest>,
    connection: WriteDbConn,
) -> Result<(), String> {
    let result = connection
        .run(move |conn| {
            if lex == "posts" {
                queue_post_creation(body, conn);
                Ok(())
            } else if lex == "reposts" {
                queue_repost_creation(body, conn);
                Ok(())
            } else if lex == "likes" {
                queue_like_creation(body, conn);
                Ok(())
            } else if lex == "follows" {
                queue_follow_creation(body, conn);
                Ok(())
            } else {
                Err(format!("Unknown lexicon received {lex:?}"))
            }
        })
        .await;
    result
}

#[tracing::instrument(skip(connection))]
pub async fn queue_deletion(
    lex: String,
    body: Vec<DeleteRequest>,
    connection: WriteDbConn,
) -> Result<(), String> {
    let result = connection
        .run(move |conn| {
            let mut delete_rows = Vec::new();
            body.into_iter()
                .map(|req| {
                    delete_rows.push(req.uri);
                })
                .for_each(drop);
            if lex == "posts" {
                delete_posts_by_uri(delete_rows, conn);
            } else if lex == "reposts" {
                delete_reposts_by_uri(delete_rows, conn);
            } else if lex == "likes" {
                delete_likes_by_uri(delete_rows, conn);
            } else if lex == "follows" {
                delete_follows_by_uri(delete_rows, conn);
            } else {
                tracing::error!("Unknown lexicon received {lex:?}");
            }
            Ok(())
        })
        .await;
    result
}

pub async fn update_cursor(
    service: String,
    sequence: i64,
    connection: WriteDbConn,
) -> Result<(), String> {
    let new_update_state = CursorUpdateState {
        service: service,
        cursor: sequence,
    };

    let result = connection
        .run(move |conn| {
            update_cursor_db(new_update_state, conn);
            Ok(())
        })
        .await;

    result
}

pub fn add_visitor(
    user: String,
    service: String,
    requested_feed: String,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::schema::visitor::dsl::*;

    let connection = &mut establish_connection()?;

    let system_time = SystemTime::now();
    let dt: DateTime<UtcOffset> = system_time.into();

    let new_visitor = (
        did.eq(user),
        web.eq(service),
        visited_at.eq(format!("{}", dt.format("%+"))),
        feed.eq(requested_feed),
    );

    diesel::insert_into(visitor)
        .values(&new_visitor)
        .execute(connection)?;
    Ok(())
}

pub async fn get_cursor(
    service_: String,
    connection: ReadReplicaConn,
) -> Result<SubState, PathUnknownErrorMessageResponse> {
    use crate::schema::sub_state::dsl::*;

    let result = connection
        .run(move |conn| {
            let mut result = sub_state
                .filter(service.eq(service_))
                .order(cursor.desc())
                .limit(1)
                .select(SubState::as_select())
                .load(conn)
                .expect("Error loading cursor records");

            if let Some(cursor_) = result.pop() {
                Ok(cursor_)
            } else {
                let not_found_error = crate::models::PathUnknownErrorMessageResponse {
                    code: Some(crate::models::NotFoundErrorCode::NotFoundError),
                    message: Some("Not found.".into()),
                };
                Err(not_found_error)
            }
        })
        .await;

    result
}
