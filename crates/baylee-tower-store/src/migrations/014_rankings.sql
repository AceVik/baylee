ALTER TABLE @tower.players
    ADD COLUMN rank_revision bigint NOT NULL DEFAULT 0 CHECK (rank_revision>=0),
    ADD COLUMN rank_listed boolean NOT NULL DEFAULT false,
    ADD COLUMN rank_name text CHECK (octet_length(rank_name) BETWEEN 1 AND 256),
    ADD COLUMN rank_tag bigint CHECK (rank_tag BETWEEN 1 AND 4294967295),
    ADD CONSTRAINT rank_identity CHECK (
        (rank_listed AND rank_name IS NOT NULL AND rank_tag IS NOT NULL)
        OR (NOT rank_listed AND rank_name IS NULL AND rank_tag IS NULL));
-- Independent of raw run retention: one bounded best per account/cohort/lane.
-- Account deletion cascades; no seed, deck, opponent identity or timing is retained.
CREATE TABLE @tower.personal_bests (
    id uuid NOT NULL UNIQUE DEFAULT pg_catalog.uuidv7() CHECK ((pg_catalog.uuid_extract_version(id)=7) IS TRUE),
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    house boolean NOT NULL,
    wins smallint NOT NULL,
    defeats smallint NOT NULL,
    listed boolean NOT NULL,
    PRIMARY KEY(account,cohort,house),
    CHECK ((wins=8 AND defeats BETWEEN 0 AND 4) OR (wins BETWEEN 0 AND 7 AND defeats=5))
);
CREATE INDEX ranked_bests ON @tower.personal_bests(cohort,house,wins DESC,defeats,id)
    WHERE listed;
