ALTER TABLE @tower.players
    ADD COLUMN record_bytes bigint NOT NULL DEFAULT 0 CHECK (record_bytes BETWEEN 0 AND 2147483648),
    ADD COLUMN record_chunks bigint NOT NULL DEFAULT 0 CHECK (record_chunks BETWEEN 0 AND 262144);
CREATE TABLE @tower.reservations (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK ((pg_catalog.uuid_extract_version(id)=7) IS TRUE),
    run uuid NOT NULL REFERENCES @tower.runs(id) ON DELETE CASCADE,
    stage smallint NOT NULL CHECK (stage BETWEEN 1 AND 12),
    host uuid NOT NULL REFERENCES @tower.operators(id),
    package bytea NOT NULL CHECK (pg_catalog.octet_length(package) BETWEEN 1 AND 131072),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE(run,stage)
);
CREATE TABLE @tower.execution_claims (
    reservation uuid NOT NULL REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    claim uuid NOT NULL CHECK ((pg_catalog.uuid_extract_version(claim)=7) IS TRUE),
    fence bigint NOT NULL CHECK (fence>0),
    host uuid NOT NULL REFERENCES @tower.operators(id),
    PRIMARY KEY(reservation,claim),
    UNIQUE(reservation,fence)
);
CREATE TABLE @tower.record_chunks (
    reservation uuid NOT NULL REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    seq bigint NOT NULL CHECK (seq BETWEEN 0 AND 65535),
    fence bigint NOT NULL CHECK (fence>0),
    previous bytea NOT NULL CHECK (pg_catalog.octet_length(previous) BETWEEN 1 AND 512),
    head bytea NOT NULL CHECK (pg_catalog.octet_length(head) BETWEEN 1 AND 512),
    data bytea NOT NULL CHECK (pg_catalog.octet_length(data) BETWEEN 1 AND 262144),
    PRIMARY KEY(reservation,seq)
);
