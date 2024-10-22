-- Your SQL goes here
CREATE TABLE IF NOT EXISTS public.repost (
                                           uri character varying NOT NULL,
                                           cid character varying NOT NULL,
                                           author character varying NOT NULL,
                                           "subjectCid" character varying NOT NULL,
                                           "subjectUri" character varying NOT NULL,
                                           "createdAt" character varying NOT NULL,
                                           "indexedAt" character varying NOT NULL
);

ALTER TABLE ONLY public.repost
    DROP CONSTRAINT IF EXISTS repost_pkey;
ALTER TABLE ONLY public.repost
    ADD CONSTRAINT repost_pkey PRIMARY KEY (uri);

-- Your SQL goes here
ALTER TABLE public.repost
    ADD COLUMN prev VARCHAR,
    ADD COLUMN sequence NUMERIC;

ALTER TABLE public.repost
    ADD CONSTRAINT unique_repost_sequence UNIQUE (sequence);

ALTER TABLE public.repost
    ALTER COLUMN sequence TYPE bigint;

