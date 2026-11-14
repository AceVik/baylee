CREATE TABLE @tower.verification_jobs (
    reservation uuid PRIMARY KEY REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    fence bigint NOT NULL CHECK (fence>0),
    generation bigint NOT NULL DEFAULT 0 CHECK (generation>=0),
    verifier uuid REFERENCES @tower.operators(id),
    claim uuid CHECK (claim IS NULL OR (pg_catalog.uuid_extract_version(claim)=7) IS TRUE),
    lease_until bigint NOT NULL DEFAULT 0 CHECK (lease_until>=0),
    ready boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CHECK ((claim IS NULL)=(verifier IS NULL))
);
CREATE INDEX verification_queue ON @tower.verification_jobs(lease_until,created_at,reservation) WHERE ready;
CREATE TABLE @tower.verification_claims (
    reservation uuid NOT NULL REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    claim uuid NOT NULL CHECK ((pg_catalog.uuid_extract_version(claim)=7) IS TRUE),
    generation bigint NOT NULL CHECK (generation>0),
    verifier uuid NOT NULL REFERENCES @tower.operators(id),
    PRIMARY KEY(reservation,claim),
    UNIQUE(reservation,generation)
);
CREATE TABLE @tower.settled_duels (
    reservation uuid PRIMARY KEY REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    verifier uuid NOT NULL REFERENCES @tower.operators(id),
    claim uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation>0),
    report bytea NOT NULL CHECK (pg_catalog.octet_length(report) BETWEEN 1 AND 8192),
    response bytea NOT NULL CHECK (pg_catalog.octet_length(response) BETWEEN 1 AND 131072),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    FOREIGN KEY(reservation,claim) REFERENCES @tower.verification_claims(reservation,claim)
);
CREATE TABLE @tower.award_outbox (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK ((pg_catalog.uuid_extract_version(id)=7) IS TRUE),
    reservation uuid NOT NULL UNIQUE REFERENCES @tower.settled_duels(reservation) ON DELETE CASCADE,
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    event bytea NOT NULL CHECK (pg_catalog.octet_length(event) BETWEEN 1 AND 8192),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    delivered_at timestamptz
);
