CREATE TABLE @tower.operators (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK ((pg_catalog.uuid_extract_version(id)=7) IS TRUE),
    name text NOT NULL CHECK (pg_catalog.octet_length(name) BETWEEN 1 AND 160),
    role text NOT NULL CHECK (role IN ('host','verifier','publisher','supervisor')),
    credential_digest bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(credential_digest)=32),
    enabled boolean NOT NULL DEFAULT true,
    capabilities bytea NOT NULL DEFAULT '\x5b5d' CHECK (pg_catalog.octet_length(capabilities) BETWEEN 2 AND 65536)
);
CREATE TABLE @tower.house_builds (
    cohort uuid NOT NULL REFERENCES @tower.cohorts(id),
    stage smallint NOT NULL CHECK (stage BETWEEN 1 AND 12),
    archetype smallint NOT NULL CHECK (archetype BETWEEN 1 AND 3),
    build bytea NOT NULL CHECK (pg_catalog.octet_length(build) BETWEEN 1 AND 16384),
    PRIMARY KEY(cohort,stage,archetype)
);
