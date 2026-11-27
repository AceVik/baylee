-- The Tower service's runtime role: what `baylee-tower-service` needs while it
-- serves players, hosts, verification and award delivery, and nothing else.
-- Applied by the schema owner after every migration (`--admin` action
-- `grant_runtime`); it first takes every earlier grant back, so a renewal is
-- exact. `@tower` is the quoted schema, `@runtime` the quoted role.
--
-- Published content, operator records and approved gateway origins are the
-- owner's (`--admin`): the runtime reads them and changes none, except a
-- host's own advertised engine inventory. No runtime DELETE outside the echo
-- pool, the execution watch and host availability; no TRUNCATE, REFERENCES,
-- TRIGGER or CREATE anywhere. `docs/tower-operations.md` §"Runtime grants"
-- lists the same per table; `tests/runtime_grants.rs` plays a whole duel's
-- path under exactly these grants and proves the refusals.
REVOKE ALL ON ALL TABLES IN SCHEMA @tower FROM @runtime;
REVOKE ALL ON SCHEMA @tower FROM @runtime;
GRANT USAGE ON SCHEMA @tower TO @runtime;

-- Reading is the service's own data, including operator credential digests
-- it authenticates against.
GRANT SELECT ON ALL TABLES IN SCHEMA @tower TO @runtime;

-- Players and their runs.
GRANT INSERT ON @tower.players, @tower.enrollment, @tower.runs, @tower.requests TO @runtime;
GRANT UPDATE (receipt_bytes, record_bytes, record_chunks, run_count,
              echo_consent, echo_epoch, consent_revision, consent_at,
              rank_revision, rank_listed, rank_name, rank_tag)
    ON @tower.players TO @runtime;
GRANT UPDATE (start_seed) ON @tower.enrollment TO @runtime;
GRANT UPDATE (state, revision, active, touched_at, expires_at) ON @tower.runs TO @runtime;

-- A host's own engine inventory; this column also lets the service hold an
-- operator row FOR SHARE while it checks the operator is still enabled.
GRANT UPDATE (capabilities) ON @tower.operators TO @runtime;
GRANT INSERT, DELETE ON @tower.host_availability TO @runtime;

-- Reservations, execution and the retained record.
GRANT INSERT ON @tower.reservations, @tower.execution_claims, @tower.record_chunks,
                @tower.record_batches TO @runtime;
GRANT UPDATE (host, automatic_recoveries) ON @tower.reservations TO @runtime;
GRANT INSERT, DELETE ON @tower.execution_watch TO @runtime;
GRANT UPDATE (fence, lease_until) ON @tower.execution_watch TO @runtime;

-- Independent verification and settlement.
GRANT INSERT ON @tower.verification_jobs, @tower.verification_claims, @tower.settled_duels
    TO @runtime;
GRANT UPDATE (fence, generation, verifier, claim, lease_until, ready, failure)
    ON @tower.verification_jobs TO @runtime;

-- Accomplishment delivery to Central.
GRANT INSERT ON @tower.award_outbox TO @runtime;
GRANT UPDATE (courier, claim, lease_until, retry_after, backoff, delivered_at)
    ON @tower.award_outbox TO @runtime;

-- The echo pool.
GRANT INSERT ON @tower.echo_snapshots, @tower.echo_partitions, @tower.eligible_echoes TO @runtime;
GRANT UPDATE (population, generation, prune_slot) ON @tower.echo_partitions TO @runtime;
GRANT UPDATE (slot) ON @tower.eligible_echoes TO @runtime;
GRANT DELETE ON @tower.eligible_echoes TO @runtime;

-- Rankings.
GRANT INSERT ON @tower.personal_bests TO @runtime;
GRANT UPDATE (wins, defeats, listed) ON @tower.personal_bests TO @runtime;
