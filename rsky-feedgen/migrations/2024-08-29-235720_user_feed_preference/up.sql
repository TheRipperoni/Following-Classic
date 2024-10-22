-- Your SQL goes here
CREATE TABLE IF NOT EXISTS public.user_feed_preference (
     did character varying NOT NULL,
     show_replies BOOLEAN NOT NULL,
     reply_filter_likes INTEGER NOT NULL,
     reply_filter_followed_only BOOLEAN NOT NULL,
     show_reposts BOOLEAN NOT NULL,
     show_quote_posts BOOLEAN NOT NULL
);

ALTER TABLE ONLY public.user_feed_preference
    ADD CONSTRAINT user_feed_preference_pkey PRIMARY KEY (did);