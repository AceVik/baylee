ALTER TABLE @tower.players ADD COLUMN echo_epoch bigint NOT NULL DEFAULT 1 CHECK (echo_epoch>0);
ALTER TABLE @tower.reservations ADD COLUMN opponent_account uuid;
CREATE TABLE @tower.echo_snapshots (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK ((pg_catalog.uuid_extract_version(id)=7) IS TRUE),
    reservation uuid NOT NULL UNIQUE REFERENCES @tower.settled_duels(reservation) ON DELETE CASCADE,
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    stage smallint NOT NULL CHECK (stage BETWEEN 1 AND 12),
    consent_epoch bigint NOT NULL CHECK (consent_epoch>0),
    build bytea NOT NULL CHECK (pg_catalog.octet_length(build) BETWEEN 1 AND 16384),
    expires_at timestamptz NOT NULL DEFAULT (pg_catalog.clock_timestamp()+interval '30 days'),
    UNIQUE(id,account,cohort,stage)
);
CREATE TABLE @tower.echo_partitions (
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    stage smallint NOT NULL CHECK (stage BETWEEN 1 AND 12),
    population bigint NOT NULL DEFAULT 0 CHECK (population>=0),
    generation bigint NOT NULL DEFAULT 0 CHECK (generation>=0),
    prune_slot bigint NOT NULL DEFAULT 0 CHECK (prune_slot>=0),
    PRIMARY KEY(cohort,stage)
);
CREATE TABLE @tower.eligible_echoes (
    cohort uuid NOT NULL,
    stage smallint NOT NULL,
    slot bigint NOT NULL CHECK (slot>=0),
    account uuid NOT NULL,
    snapshot uuid NOT NULL UNIQUE,
    PRIMARY KEY(cohort,stage,slot),
    UNIQUE(cohort,stage,account),
    FOREIGN KEY(cohort,stage) REFERENCES @tower.echo_partitions(cohort,stage),
    FOREIGN KEY(snapshot,account,cohort,stage) REFERENCES @tower.echo_snapshots(id,account,cohort,stage) ON DELETE RESTRICT
);
CREATE INDEX echoes_by_owner ON @tower.eligible_echoes(account,cohort,stage);
