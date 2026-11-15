CREATE TABLE @tower.host_locations (
    host uuid PRIMARY KEY REFERENCES @tower.operators(id),
    origin text NOT NULL UNIQUE CHECK (pg_catalog.octet_length(origin) BETWEEN 1 AND 1024),
    development boolean NOT NULL,
    approved_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
