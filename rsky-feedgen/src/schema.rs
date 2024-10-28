// @generated automatically by Diesel CLI.

diesel::table! {
    follow (uri) {
        uri -> Varchar,
        cid -> Varchar,
        author -> Varchar,
        subject -> Varchar,
        createdAt -> Varchar,
        indexedAt -> Varchar,
        prev -> Nullable<Varchar>,
        sequence -> Nullable<Int8>,
    }
}

diesel::table! {
    like (uri) {
        uri -> Varchar,
        cid -> Varchar,
        author -> Varchar,
        subjectCid -> Varchar,
        subjectUri -> Varchar,
        createdAt -> Varchar,
        indexedAt -> Varchar,
        prev -> Nullable<Varchar>,
        sequence -> Nullable<Int8>,
    }
}

diesel::table! {
    post (uri) {
        uri -> Varchar,
        cid -> Varchar,
        replyParent -> Nullable<Varchar>,
        replyRoot -> Nullable<Varchar>,
        indexedAt -> Varchar,
        prev -> Nullable<Varchar>,
        sequence -> Nullable<Int8>,
        text -> Nullable<Varchar>,
        lang -> Nullable<Varchar>,
        author -> Varchar,
        externalUri -> Nullable<Varchar>,
        externalTitle -> Nullable<Varchar>,
        externalDescription -> Nullable<Varchar>,
        externalThumb -> Nullable<Varchar>,
        quoteCid -> Nullable<Varchar>,
        quoteUri -> Nullable<Varchar>,
        media -> Bool,
        alt -> Nullable<Varchar>
    }
}

diesel::table! {
    repost (uri) {
        uri -> Varchar,
        cid -> Varchar,
        author -> Varchar,
        subjectCid -> Varchar,
        subjectUri -> Varchar,
        createdAt -> Varchar,
        indexedAt -> Varchar,
        prev -> Nullable<Varchar>,
        sequence -> Nullable<Int8>,
    }
}

diesel::table! {
    sub_state (service) {
        service -> Varchar,
        cursor -> Int8,
    }
}

diesel::table! {
    user_feed_preference (did) {
        did -> Varchar,
        show_replies -> Bool,
        reply_filter_likes -> Int4,
        reply_filter_followed_only -> Bool,
        show_reposts -> Bool,
        show_quote_posts -> Bool,
        hide_seen_posts -> Bool,
        hide_no_alt_text -> Bool,
    }
}

diesel::table! {
    following_preference (did) {
        author -> Varchar,
        did -> Varchar,
        show_reposts -> Bool,
        show_quote_posts -> Bool,
    }
}

diesel::table! {
    fetched_post (did) {
        id -> Int4,
        did -> Varchar,
        uri -> Varchar,
    }
}

diesel::table! {
    seen_post (did) {
        id -> Int4,
        did -> Varchar,
        uri -> Varchar,
    }
}

diesel::table! {
    video (cid) {
        cid -> Varchar,
        alt -> Nullable<Varchar>,
        postCid -> Varchar,
        postUri -> Varchar,
        createdAt -> Varchar,
        indexedAt -> Varchar,
        labels -> Nullable<Array<Nullable<Text>>>,
    }
}

diesel::table! {
    visitor (id) {
        id -> Int4,
        did -> Varchar,
        web -> Varchar,
        visited_at -> Varchar,
        feed -> Nullable<Varchar>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    follow,
    like,
    post,
    repost,
    sub_state,
    user_feed_preference,
    video,
    visitor,
    following_preference,
    fetched_post,
    seen_post,
);
