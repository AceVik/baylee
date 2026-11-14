ALTER TABLE @tower.players
    ADD COLUMN consent_revision smallint NOT NULL DEFAULT 1 CHECK (consent_revision=1),
    ADD COLUMN consent_at timestamptz,
    ADD COLUMN receipt_bytes bigint NOT NULL DEFAULT 0 CHECK (receipt_bytes BETWEEN 0 AND 67108864),
    ADD COLUMN run_count integer NOT NULL DEFAULT 0 CHECK (run_count BETWEEN 0 AND 1024);
UPDATE @tower.players p SET consent_at=created_at,
    receipt_bytes=COALESCE((SELECT sum(pg_catalog.octet_length(r.response)) FROM @tower.requests r WHERE r.account=p.account),0),
    run_count=(SELECT count(*) FROM @tower.runs r WHERE r.account=p.account);
ALTER TABLE @tower.players ALTER COLUMN consent_at SET NOT NULL,
    ALTER COLUMN consent_at SET DEFAULT pg_catalog.clock_timestamp();
