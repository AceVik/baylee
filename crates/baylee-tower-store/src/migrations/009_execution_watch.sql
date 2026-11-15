ALTER TABLE @tower.reservations ADD COLUMN automatic_recoveries smallint NOT NULL DEFAULT 0
    CHECK (automatic_recoveries BETWEEN 0 AND 3);
CREATE TABLE @tower.execution_watch (
    run uuid PRIMARY KEY REFERENCES @tower.runs(id) ON DELETE CASCADE,
    fence bigint NOT NULL CHECK (fence>0),
    lease_until bigint NOT NULL CHECK (lease_until>0)
);
CREATE INDEX execution_expiry ON @tower.execution_watch(lease_until,run);
-- One-time migration of the versioned JSON state; normal writes maintain this
-- index through the Rust transaction adapter, without reparsing JSON in SQL.
INSERT INTO @tower.execution_watch(run,fence,lease_until)
SELECT id,(document->'execution'->>'fence')::bigint,
          (document->'execution'->>'lease_until')::bigint
FROM (SELECT id,pg_catalog.convert_from(state,'UTF8')::jsonb AS document FROM @tower.runs) saved
WHERE document->>'phase'='playing';
