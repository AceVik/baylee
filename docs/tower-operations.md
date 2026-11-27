# Tower Echoes operator commands

The separate `baylee-tower-service` executable owns Tower publication, economy
and execution authority in the Baylee repository. Central owns identity and
permanent accomplishment receipts. This runbook describes local/offline
provisioning; release approval, calibration, retention and the final least-privilege
runtime grants remain separate release requirements.

The expanded Commander run catalog uses generator 6 / native `tower-6` and API header
`baylee-tower-version: 8` on the existing `/v2/player` endpoint and host routes.
Native players may also use `/v2/player/ws/8`; reverse proxies must forward
WebSocket upgrades and allow the 15-second ping/pong heartbeat. Migration 015 adds
transactional invalidation triggers with invoker rights and a fixed `pg_catalog`
search path. Each authority listens on its schema name; hints are private-data-free
and snapshots always recheck account ownership. No new database role, external
broker, credential or production network endpoint is required.
Older API clients receive 426 before authentication rather than undecodable new
families. Deploy matching service, gateway and client builds together. Retain the
exact `tower-2` / `tower-house-19` executors for generation-one runs and
`tower-3` / `tower-house-20` for generation two and `tower-4` /
`tower-house-21` for generation three. Publishing a new cohort preserves
existing runs and their snapshots. Retain `tower-5` / `tower-house-22` for
generations four and five. The current executor uses `tower-6` /
`tower-house-23` and refuses retained manifests. All prior catalog/offer streams
remain frozen; new families cannot enter old runs. An old run cannot silently
resume on the current binary, even if its source registry appears compatible.

API 5 introduced the generation-four reward phase, six-slot run inventory and
39+1 Commander formations. Generation six is the new-run default; enrollment
still requires an explicit immutable publication and matching installed executor.
The separate `baylee-tower-content` adapter builds fifty reviewed-card starter
collections through the existing synergy/curve/mana-source builder and validates
their native metadata; it does not certify measured balance. Generation four's
five original lists remain supported. Migration 016 expands the house-archetype
bound to 64; the publication still enforces its exact generation's pool. API 7
offers three stable owner-bound indices and rejects forged selections at Start.
Draft reads do not opt into echo sharing. Enrollment seeds share the existing
per-account run-count bound and cannot be freely resampled by abandoning.
Immutable publications have a separate 256-KiB encoded limit; mutable run state
retains its 128-KiB limit. Cohort pages contain at most three publications so
their envelopes remain below the one-MiB player response bound. Migration 016
notifies only changed existing enrollment seeds: first reads cannot create a
notification/read loop, and seed invalidations reach only that account's sockets.

Generation six expands to 178 families (18 original plus 160 event/reward
recipes), four tiers each. Ten relic slots and two supply slots replace the
earlier eight offers; the guaranteed stage-seven crafting offer uses one of the
last two slots. Existing stock never changes when inventory moves. One progression
slot and bounded additional affinity use exact owned Common/Uncommon definitions;
every family remains reachable. Runic purchase prices are 2/5/14 gold; original
families retain 4/10/26. Three equal items automatically fuse across bag and
equipment; a Rare plus the 24-gold material makes Mythic. Fusion carries actual
paid basis and the first equipped slot. Individual Mythic feedback recipes cost
more than their own generated charges; combinations across items remain allowed.
On 18.11.2026 the runic levels were strengthened in place before release (cheaper,
stronger Common to Rare levels; Mythic costs unchanged; `baylee-tower-relics`
`tuning.rs` is the one source). Within a per-turn cap, five Rare pairs now refill
their own cost (Bloom Seed and Horn, Lore/Lifeblood/Echo Lens); the relics crate's
tests pin that list. These are balance hypotheses, not measured competitive parity.

After verified settlement, claim
the pending cards before shopping or abandoning. Reconnecting preserves the
same candidates. Generation six grants three gold per already-owned selection,
with the same durable claim receipt; retained generations keep one. Its five distinct
nonbasic offers are weighted toward the player's commander identity and higher
rarities, preserving the opponent's frozen printing and finish. Victory takes
three; defeat/draw takes one. The client resolves exact-edition German images
locally where available, with English fallback and no change to replay identity.

API 6 adds cohort-scoped Tower rankings and a separate reversible publication
setting. Migration 014 stores one best completed verified run per account/cohort/
opponent lane. Every all-echo run belongs to the echo board; any verified house
opponent puts the entire run in the house board. Sort by victories descending,
then defeats/rules draws ascending; true ties share competition rank. Time, gold,
life, and stable row IDs never break a score tie. The response contains the top 25
and the authenticated owner's own best. Abandoned/partial/technical runs do not
count. A final pending card reward does not delay the already verified score.

Public listing defaults off, independent of echo consent. The service obtains
display name/tag from live Central introspection, never from request fields.
Names refresh when the owner reads a board; removing the public identity withdraws
the listing. Publication and withdrawal use a monotonic compare-and-set revision;
an old publish retry cannot undo a later withdrawal. Deleting the Tower player
cascades its bests. This does not implement cross-service Central account-erasure
delivery: operators must include the Tower player in their deletion workflow and
must not claim that Central deletion alone has removed Tower data. Private bests
survive raw-run retention until player/cohort removal; the new projection starts
with verified completions after migration, without inventing scores for old runs.
Apply migration with the schema owner and renew the reviewed runtime table grants
before starting the matched API-8 service. Production retention/privacy approval
remains a release requirement.

## Input and authority

An owning Rust launcher supplies one JSON value through private stdin and closes
the pipe. Never place database URLs or credentials in files, command arguments,
logs or shell history. Reuse the reviewed `central_api::Token::generate` for
independent credentials; do not derive tokens from UUIDs or names. Retain them in
the operator's approved secret source. Nothing prints or recovers credentials.

`--admin` executes one operation and closes its pool. Its input is:

```text
{
  "configuration": {"database_url": <private URL>, "schema": <schema>,
                    "development": <explicit boolean>},
  "operation": {"action": <closed action>, ...fields below...}
}
```

The whole input is bounded to 196 KiB, unknown fields are errors, and `service`
must be omitted/null. The database TLS, schema and explicit numeric-loopback
development rules are exactly the [service configuration rules](dev/environment.md).
No listener, verifier child or courier starts in this mode. An exact current
migration history and effective membership in the SQL schema-owner role are
required before any operation. A Host, Publisher or other application credential
does not supply that SQL ownership. Do not give runtime roles membership in the
owner role. Database superusers already have ownership-level authority.

`--check-admin` accepts the same input, validates its shape and local bounds, and
exits without opening the database. It does **not** establish operator permissions,
existing IDs, immutable publication compatibility, card coverage or game balance.
Both modes reject extra arguments. Success exits zero; refusal exits nonzero with
a fixed diagnostic containing no input. A failed preflight never changes state.

## Closed operations

| `action` | Additional fields | Result and semantics |
| --- | --- | --- |
| `provision` | `name`, `role`, `credential` | `{"result":"operator","id":<UUIDv7>}`. Role is exactly `host`, `verifier`, `supervisor`, `publisher` or `courier`; credential is 64 lowercase hex digits. Names are 1–160 bytes, trimmed and without control characters. An exact enabled name/role/token retry returns the same ID. Changed name/role or a revoked credential refuses. |
| `revoke` | `operator` | `{"result":"done"}`. Disables an existing operator; exact repeats succeed. Live authenticated principals lose their role on the next protected operation. No history is deleted or replacement credential minted. |
| `approve_host` | `host`, `origin` | `done`. Binds an enabled Host to one canonical approved gateway origin. Production requires HTTPS; development allows only numeric-loopback HTTP. An exact repeat succeeds; changing the origin refuses. |
| `publish` | `publication` | `{"result":"published","cohort":<UUIDv7>}`. Accepts the typed `baylee_tower_store::Publication`: frozen cohort/executor identity, reviewed catalog/prices and 50–64 distinct Commander starters for generation five (5–8 for generation four, three for earlier generations). Exact canonical repeats return the same ID. Different content under the same contract refuses. |
| `publish_house` | `publisher`, `cohort`, `stage`, `archetype`, `build` | `done`. Additionally requires a live dedicated Publisher credential. Stage is 1–12, archetype 1–8 (1–3 for retained cohorts). The typed `Build` contains `formation`, `deck`, `active`, `potions`; it must validate against that cohort's saved catalog and loadout bounds. Run equipment has six slots; retained builds keep three. An exact repeat succeeds; changing a published fallback refuses. |
| `pause_enrollment` | `cohort` | `done`. Closes new enrollment, preserving existing runs, receipts and immutable content. Exact repeats succeed. Republishing the same content cannot reopen it. |
| `supervise` | `supervisor`, `run`, `revision`, `recovery` | `{"result":"supervised","revision":<u64>}`. Additionally requires a live dedicated Supervisor credential and the run's current saved revision. `recovery` is `{"kind":"quarantine"}` (stop a reserved, playing or verifying duel and revoke its generations), `{"kind":"resume","host":<UUIDv7 or null>}` (a quarantined run returns to its exact retained package/prefix; an unsealed duel names an enabled compatible Host, a sealed record names none and returns to verification) or `{"kind":"interrupt"}` (retire a quarantined run without a rules result or compensation). A stale revision or wrong phase refuses without change; the reply's revision is the one to name next. |
| `grant_runtime` | `role` | `done`. Renews the runtime role's reviewed grants (§"Runtime grants") after taking every earlier one back. Refuses a role that is or inherits the schema owner, and any caller that is not the owner. |

All IDs must be UUIDv7. There is no arbitrary SQL, script, state patch, currency
grant, award injection, unrevocation, content rewrite or reopening operation.
`supervise` applies only the three closed technical transitions above; it never
assigns an outcome, a fresh opponent, gold or a rewound shop.

No player waits on an operator: the owner may also retire (`abandon`) a run whose
duel is reserved but never started or held in quarantine. That ends the run with
no outcome or income; a duel being played or verified cannot be left this way
(the queue recovers or quarantines it).

The service's verification queue quarantines a job whose settlement is refused
(for example a spent receipt quota) or whose bounded claims are all spent, and
waits out a passing store fault with a pause doubling from one second to one
minute. Only a revoked worker credential, an executor mismatch or an
incompatible schema stops the service.
No output includes a database URL, credential or private house build. The binary
returns only the closed results above. A lost response is resolved by retrying the
exact command; never substitute a fresh credential merely because a reply was lost.

## Bring up a reviewed cohort

1. Migrate the dedicated PostgreSQL 18+ schema with separate owner credentials
   using `--migrate`. Normal startup refuses missing/different migration history.
2. Prepare and review the complete immutable publication. Play/calibrate all
   starters and fallbacks with its exact retained Tower executor before enabling
   public enrollment. Shape validation is not a playability certificate.
3. Provision separate Host, Verifier, Supervisor, Publisher and Courier credentials.
   No role implies another. Use `approve_host` for each reviewed gateway origin;
   advertisements alone cannot select an arbitrary client destination.
4. Publish the cohort and its reviewed, explicitly labelled house fallbacks. Keep
   the returned cohort ID and exact executor available for outstanding runs.
5. Independently provision Central's scoped award issuer for the same Tower origin.
   An ordinary gateway/challenge bridge cannot issue these receipts. Never reuse
   that issuer credential for the local Courier role.
6. Start the service with its runtime SQL credentials, configured cohort verifier/
   supervisor workers and award courier. Supply gateway Host credentials separately.
   Readiness is not proof of complete cohort coverage or Central availability.

For incident containment, pause enrollment and/or revoke the compromised operator.
Already committed results and outbox records remain durable. Revocation is a
live authorization boundary, not retroactive cancellation of transactions that
committed before it. Replacing a revoked Host requires a fresh credential/ID and
explicit handling of retained assignments; there is no silent origin takeover.

## Runtime grants

The service never runs as the schema owner. Its runtime role gets exactly
`crates/baylee-tower-store/src/runtime_grants.sql`, which the owner renews
after every migration with the closed admin action
`{"action":"grant_runtime","role":"<runtime role>"}`; the action takes every
earlier grant back first and refuses a role that is or inherits the owner.
Production startup (`development: false`) refuses a connection that owns the
schema, may create in it, or may write published cohorts, house builds,
approved gateway origins, operator records or their enabled flag
(`Store::runtime_least_privilege`).

| Table | Runtime may |
| --- | --- |
| every table | `SELECT` |
| `players` | `INSERT`; `UPDATE` of the byte/chunk/run counters, echo consent and ranking listing columns |
| `enrollment` | `INSERT`; `UPDATE (start_seed)` |
| `runs` | `INSERT`; `UPDATE (state, revision, active, touched_at, expires_at)` |
| `requests` | `INSERT` |
| `operators` | `UPDATE (capabilities)` only: a host's own engine inventory, also what lets the service hold an operator row `FOR SHARE` |
| `host_availability` | `INSERT`, `DELETE` |
| `reservations` | `INSERT`; `UPDATE (host, automatic_recoveries)` |
| `execution_claims`, `record_chunks`, `record_batches` | `INSERT` |
| `execution_watch` | `INSERT`, `DELETE`; `UPDATE (fence, lease_until)` |
| `verification_jobs` | `INSERT`; `UPDATE (fence, generation, verifier, claim, lease_until, ready, failure)` |
| `verification_claims`, `settled_duels` | `INSERT` |
| `award_outbox` | `INSERT`; `UPDATE (courier, claim, lease_until, retry_after, backoff, delivered_at)` |
| `echo_snapshots` | `INSERT` |
| `echo_partitions` | `INSERT`; `UPDATE (population, generation, prune_slot)` |
| `eligible_echoes` | `INSERT`, `DELETE`; `UPDATE (slot)` |
| `personal_bests` | `INSERT`; `UPDATE (wins, defeats, listed)` |
| `cohorts`, `house_builds`, `host_locations`, `schema_version` | nothing beyond `SELECT` |

No `TRUNCATE`, `REFERENCES`, `TRIGGER` or `CREATE` anywhere. The store's
`execution` test target plays a whole run (reservation, record, seal,
verification, settlement, reward claims, award delivery, rankings, echo
withdrawal) as a role holding only these grants and proves every
administrative write refused, through the store and in plain SQL
(`tests/execution/grants.rs`).

## Deleted accounts

Tower data is not deleted with a Central account: it is kept until its
season ends, about six months, and pseudonymised (Central's privacy policy
2026-11-27, § 7; the deletion at a season's end comes with the next major
update). What does go is the account's display name and tag on the Tower
leaderboard. The award courier learns of a deletion from Central's answer
`account_gone`: on a delivery, and once a day for every listed account, by
repeating its newest delivered fact exactly (Central deduplicates it and
changes nothing). Either way the listing is withdrawn as if the owner had
removed their public identity (`Store::ranking_identity`). The first daily
check runs ten minutes after the service starts.

## Production on one server

The official cohort runs beside the gateway on the release server
(`docs/releasing.md`). Everything below is root's, done once; credentials go
only into root-owned 0600 files that a process reads on stdin.

1. **Database.** A database `baylee_tower` in the shared PostgreSQL container,
   owned by a login role `tower_owner`, with a login role `tower_runtime`
   that owns nothing. In `pg_hba.conf`, above every `host` line:
   `hostnossl baylee_tower all all reject`, then `hostssl baylee_tower
   tower_owner,tower_runtime 127.0.0.1/32 scram-sha-256`, and both roles
   refused on every other database. Both URLs use
   `sslmode=verify-full&sslrootcert=/etc/baylee-tower/db-ca.pem` (a copy of
   the database's private CA certificate, 0644) with host `127.0.0.1`.
2. **The retained engine.** After the release deploy, copy the deployed
   `/opt/baylee/bin/baylee-engine-tower` to
   `/opt/baylee/engines/tower-<short commit>/` and list that copy, with
   `/opt/baylee/bin/baylee-engine-standard-loops`, in the agent's
   `BAYLEE_ENGINE_BINS` (a set value turns sibling discovery off; the same
   identity twice stops the agent, so never list the deployed sibling too).
   The cohort binds to this copy; a later deploy replaces the sibling and
   leaves every run of the cohort playable. Restart the agent only with no
   game running.
3. **Binaries.** `baylee-tower-service` and `baylee-tower-provision`
   (`cargo build --release --locked -p baylee-tower-service -p
   baylee-tower-provision`) in `/opt/baylee-tower/bin`; a system user
   `baylee-tower` without a shell or home.
4. **Cohort.** `baylee-tower-provision` reads one JSON value:
   `{"service": "/opt/baylee-tower/bin/baylee-tower-service", "engine":
   <retained engine>, "configuration": {"database_url": <owner URL>,
   "schema": "tower", "development": false}, "gateway":
   "https://gateway.baylee.app", "runtime_role": "tower_runtime",
   "house_seed": <64 hex>, "credentials": {"host", "verifier", "supervisor",
   "publisher", "courier"}}`, each credential a fresh `Token` in 64 hex. It
   migrates, renews the runtime grants, provisions the five operators,
   approves the gateway, publishes the cohort bound to the engine's identity
   and the house's 600 builds (12 stages × 50 starters, from the private
   seed; `§"Calibration"`), all through `--migrate`/`--admin`, and prints the
   cohort and operator IDs. An exact repeat succeeds with the same IDs.
   Keep the seed and the credentials in `/etc/baylee-tower/` (root, 0600).
5. **Central.** Provision the award issuer for `https://tower.baylee.app`
   with the migrator's connection (`central_identity.tower_award_issuers`,
   the SHA-256 digest of a fresh 32-byte `Token`, never the Courier's), set
   `tower_origin` to `https://tower.baylee.app` in `central.json`, restart
   Central.
6. **Service.** `/etc/baylee-tower/service.json` (root 0600): the runtime
   URL, `"development": false`, and `service`: `listen` 127.0.0.1:28796,
   `origin` `https://tower.baylee.app`, `record_bytes`, one `workers` entry
   (the cohort, the retained engine, the verifier and supervisor
   credentials) and `awards` (the courier and the issuer credentials).
   Install `scripts/server/baylee-tower.service` and
   `scripts/server/tower.caddy`, `systemctl enable --now baylee-tower`,
   reload Caddy, and check `https://tower.baylee.app/health`.
7. **Gateway.** `BAYLEE_TOWER_ORIGIN=https://tower.baylee.app` and
   `BAYLEE_TOWER_HOST_TOKEN=<host credential>` in the gateway's environment;
   the gateway restart ends running games, so it waits for an empty table
   list. Central then shows the Tower tile in the client.

## Reproducible focused checks

With an owned synthetic PostgreSQL fixture, run the service's `admin` integration
target and binary tests, then the existing `bootstrap` and store `operators`
targets. The admin fixture exercises real child stdin/stdout, owner and restricted
SQL roles, every command, retry/immutability, live revocation and fixed diagnostics.
It bounds child output/deadlines, reaps commands and removes its owned schemas and
login role. Its all-basic-land publication is deliberately structural test data,
not a balanced release deck or approved content package.
