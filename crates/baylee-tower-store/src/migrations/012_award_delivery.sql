ALTER TABLE @tower.operators DROP CONSTRAINT operators_role_check;
ALTER TABLE @tower.operators ADD CONSTRAINT operators_role_check
    CHECK (role IN ('host','verifier','publisher','supervisor','courier'));

ALTER TABLE @tower.award_outbox
    ADD COLUMN courier uuid REFERENCES @tower.operators(id),
    ADD COLUMN claim uuid CHECK (claim IS NULL OR (pg_catalog.uuid_extract_version(claim)=7) IS TRUE),
    ADD COLUMN lease_until bigint NOT NULL DEFAULT 0 CHECK (lease_until>=0),
    ADD COLUMN retry_after bigint NOT NULL DEFAULT 0 CHECK (retry_after>=0),
    ADD COLUMN backoff smallint NOT NULL DEFAULT 0 CHECK (backoff BETWEEN 0 AND 10),
    ADD CONSTRAINT award_claim_pair CHECK ((claim IS NULL)=(courier IS NULL)),
    ADD CONSTRAINT award_claim_expiry CHECK ((claim IS NULL)=(lease_until=0));
CREATE INDEX pending_award_delivery ON @tower.award_outbox(retry_after,created_at,id)
    WHERE delivered_at IS NULL;
