CREATE TABLE @tower.host_availability (
    host uuid NOT NULL REFERENCES @tower.operators(id),
    executor bytea NOT NULL CHECK (pg_catalog.octet_length(executor) BETWEEN 1 AND 2048),
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (executor, host)
);
CREATE INDEX host_availability_owner ON @tower.host_availability(host);
