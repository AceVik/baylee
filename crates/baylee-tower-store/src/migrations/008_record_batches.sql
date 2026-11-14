CREATE TABLE @tower.record_batches (
    reservation uuid NOT NULL REFERENCES @tower.reservations(id) ON DELETE CASCADE,
    seq bigint NOT NULL CHECK (seq BETWEEN 0 AND 65535),
    fence bigint NOT NULL CHECK (fence>0),
    chunks smallint NOT NULL CHECK (chunks BETWEEN 1 AND 64 AND seq+chunks<=65536),
    previous bytea NOT NULL CHECK (pg_catalog.octet_length(previous) BETWEEN 1 AND 512),
    head bytea NOT NULL CHECK (pg_catalog.octet_length(head) BETWEEN 1 AND 512),
    PRIMARY KEY(reservation,seq)
);
