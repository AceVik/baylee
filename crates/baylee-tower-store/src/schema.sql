CREATE TABLE @tower.cohorts (
    id uuid PRIMARY KEY DEFAULT uuidv7() CHECK ((uuid_extract_version(id)=7) IS TRUE),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest)=32),
    contract bytea NOT NULL UNIQUE CHECK (octet_length(contract)=32),
    publication bytea NOT NULL CHECK (octet_length(publication) BETWEEN 1 AND 131072),
    accepting boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE @tower.players (
    account uuid PRIMARY KEY CHECK ((uuid_extract_version(account)=7) IS TRUE),
    echo_consent boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE @tower.enrollment (
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    start_seed bytea NOT NULL CHECK (octet_length(start_seed)=32),
    PRIMARY KEY(account,cohort)
);
CREATE TABLE @tower.runs (
    id uuid PRIMARY KEY DEFAULT uuidv7() CHECK ((uuid_extract_version(id)=7) IS TRUE),
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    revision bigint NOT NULL CHECK (revision>=0),
    active boolean NOT NULL,
    state bytea NOT NULL CHECK (octet_length(state) BETWEEN 1 AND 131072),
    shop_seed bytea NOT NULL CHECK (octet_length(shop_seed)=32),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    touched_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    expires_at timestamptz NOT NULL DEFAULT (clock_timestamp()+interval '30 days')
);
CREATE UNIQUE INDEX one_active_run ON @tower.runs(account,cohort) WHERE active;
CREATE INDEX runs_by_owner ON @tower.runs(account,created_at,id);
CREATE TABLE @tower.requests (
    account uuid NOT NULL REFERENCES @tower.players(account) ON DELETE CASCADE,
    request uuid NOT NULL CHECK ((uuid_extract_version(request)=7) IS TRUE),
    run uuid NOT NULL REFERENCES @tower.runs(id) ON DELETE CASCADE,
    digest bytea NOT NULL CHECK (octet_length(digest)=32),
    response bytea NOT NULL CHECK (octet_length(response) BETWEEN 1 AND 131072),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(account,request)
);
CREATE INDEX requests_by_run ON @tower.requests(run);
