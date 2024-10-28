-- Your SQL goes here
CREATE TABLE IF NOT EXISTS public.following_pref
(
    did              character varying NOT NULL,
    following_did    character varying NOT NULL,
    reposts_disabled boolean,
    queue_mode       boolean
);

ALTER TABLE ONLY public.following_pref
    ADD CONSTRAINT following_pref_pkey PRIMARY KEY (did);