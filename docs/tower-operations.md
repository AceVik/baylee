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

## Calibration

Measured on 27.11.2026 with the Tower lab, the `baylee-engine-server` example
`tower_lab` (built with `--no-default-features --features tower`),
which plays native duels of this compiled Tower executor in process: the
package → preset → session path of a hosted duel, no database or network.
The Tower Expert has no foresight tier (`AIProfile::TOWER_EXPERT`), so the
house chair plays exactly as at a hosting engine; the human chair is answered
by the same Expert heuristics through its own view. Every pairing is played in
both chair assignments on the same deals, so a chair advantage (the house
chair wins about 55 %) cancels. A duel ended by a bound (30,000 steps, 60
turns, 300 s) is counted apart and never scored. The bands were fixed before
any result was read; a choice among candidates is selection, so only a fresh
measurement of the whole field counts as the result.

- **Starters:** every bare starter against every other, 20 deals per ordered
  pair: 1,960 duels per starter, 49,000 in all. Band: 40–60 % against the
  field.
- **House:** each stage's fifty builds against each other in both chairs,
  and each build against four builds of the stage below. Band: 30–70 %
  within a stage; every stage beats the one below.

**Starters.** The first matrix ranged from 15.7 % (Kykar) to 93.2 % (Lyra);
28 of 50 lay outside the band. the lab's `variants` then measured, for
each of them, every builder archetype under four seed variants against the
field (196 duels each), and replacement commanders of the same colours where
no variant came near. The chosen variant is the third value in `LEADERS`;
Lyra Dawnbringer, Giada, Dwynen and Eutropia gave way to Soraya the Falconer,
Loran of the Third Path, Eladamri and Sab-Sunen. Measured again on fresh deals
(another 49,000 duels): 37.8 % (Talrand) to 61.9 % (Elas il-Kor). A third
round moved Talrand, Arasta, Hapatra and Rishkar to other variants and
replaced Elas il-Kor and Arvad with Neva, Stalked by Nightmares and Selenia,
Dark Angel. The final field, on fresh deals (24,500 duels, 980 per starter):
every starter between 40.1 % (Samut) and 59.6 % (Ayara), all fifty inside
the band; the 95 % intervals are about ±3 points. The human chair wins 45.8 %
of all duels.

**House.** Built from the second round's starters and measured before any
cell was touched (33,800 duels), the ladder rose at every stage (each stage's
builds beat the stage below 50–67 %), but within a stage the builds ranged
from 2 % to 100 %, with 9 to 28 of 50 outside the band per stage. The
winners were creature and token decks whose relics pump the whole team
(Banner and Bulwark recipes, Swarm Banner, Grove, Dawn Aegis); the losers
were spell decks, whose relic options do much less for them. That is the
relic catalogue's balance as much as the house's, and players meet it too.
The house is calibrated cell by cell instead: `PICKS` gives each of the 600
cells the candidate that came closest to even against its stage's field,
out of its own build, one to three of its most valuable relics left in the
bag, and the builds of two further shop runs (86,400 duels). Measured again
on fresh deals (33,800 duels): every stage still beats the one below
(53–70 %); outside the band per stage 1 (stage 1), 6–14 (stages 2–9) and
17–19 (stages 10–12), with the extremes at 9 % and 97 %. A second round
touched only the cells that fresh measurement found outside the band: each
took the best of its pick, one or two relics more or one less in the bag,
and the same trim of the other two shop runs, measured against the
published ladder itself (76,224 duels). The ladder that is published,
measured on fresh deals (33,800 duels, `ladder-v4.json`): every stage beats
the one below by 54–69 %; 505 of the 600 cells lie inside 30–70 % (before
any cell calibration 336), outside per stage 1, 2, 7, 7, 5, 6, 8, 7, 10,
13, 16 and 13, between 12 % and 97 %. The later stages, where whole-team
relic engines are complete, keep most of the rest; tuning those relics is a
change to the run's balance for players too, not to the house alone.

**On Wave 19's rules.** The cohort binds to the Tower engine of 0.2.2,
whose card registry Wave 19 (`fc309f1d4`) set; the AI-player work in it
changes no card, engine, house AI or Tower crate. The frozen starters
measured again under those rules (24,500 duels, `w19-starters.json`):
38.7 % (Ghen) to 60.2 % (Vona); 45 of 50 inside the band and the other five
within 1.3 points of its edge, inside their own intervals. The house ladder
was calibrated afresh on that registry (86,400 duels for the first pick
table, then 55,344 for the 125 cells a fresh measurement still found outside
the band, with up to four relics in the bag and the other shop runs). The
published ladder, on fresh deals (33,800 duels, `w19-ladder-v2.json`):
every stage beats the one below by 51–64 %; 531 of the 600 cells lie inside
30–70 %, outside per stage 2, 3, 6, 7, 1, 2, 5, 6, 12, 7, 9 and 9, between
10 % and 88 %.

**What the numbers hold for.** Both chairs are the Expert heuristics, so
these are measurements of builds, not of human play: a person who shops
and pilots better than the Expert beats the house more often. The shared
builder draws on the whole card registry, and Wave 19 (`fc309f1d4`) would
have moved twenty of the fifty generated decks out of the band. The decks
measured here are therefore frozen data (`expeditions/frozen.rs`), each card
checked against the registry but never rebuilt from it; on Wave 19's
registry they differ from the measured ones in reference printings only.
`baylee_tower_content::CALIBRATED` holds the content's fingerprint (images
left out), and `baylee-tower-provision` refuses to publish any other. A
house refit requires only the core cards its starter holds, so a card wave
cannot reach the ladder through a starter's theme either; the ladder is
still built over the registry's card definitions, which is why it is
measured again on the engine a cohort binds to
(`docs/history/tower-calibration-2026-11-27/`).

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
   that owns nothing. In `pg_hba.conf`, beside Central's lines and above
   every other `host` line: `hostnossl baylee_tower all all reject`, then
   `hostssl baylee_tower tower_owner,tower_runtime all scram-sha-256`, then
   `host all tower_owner,tower_runtime all reject`. The address is `all`, as
   in Central's lines: the container sees the Docker bridge, never
   `127.0.0.1`. Check `pg_hba_file_rules` for an `error` before
   `pg_reload_conf()`. Create the roles in one superuser session that first
   sets `log_statement = 'none'`, `log_min_duration_statement = -1` and
   `log_min_error_statement = panic`, with the passwords on standard input.
   Both URLs use `sslmode=verify-full&sslrootcert=/etc/baylee-tower/db-ca.pem`
   (a copy of the database's private CA certificate, 0644) with host
   `127.0.0.1`.
2. **The cohort's engine.** After the release deploy, check that
   `/opt/baylee/bin/baylee-engine-tower --metadata` names the release in
   `build` and `cards`, and copy it to
   `/opt/baylee/engines/tower-<short commit>/`. The name is deliberately not
   a bare revision: a deploy that keeps games (`docs/deploy-hooks.md`)
   prunes every `engines/<revision>/` no game plays on. The service's
   verifier replays records with that copy, whatever a later deploy
   installs. A deploy that changes the Tower engine strands the cohort's
   open runs (their next duel finds no host; players can abandon), unless
   the agent keeps the copy: a drop-in
   `/etc/systemd/system/baylee-agent.service.d/tower-engine.conf` with
   `Environment=BAYLEE_ENGINE_BINS=/opt/baylee/bin/baylee-engine-standard-loops:<the
   copy>`. A set value turns sibling discovery off, so it lists
   standard-loops; it never lists the deployed Tower engine (the same
   identity twice stops the agent, and a newer one is not the cohort's). The
   agent reads it at its next start; restart it once, while no game runs,
   rather than let the next deploy find out. A keeping deploy writes
   `/etc/baylee/agent-engines.env`, which as the unit's `EnvironmentFile`
   overrides the drop-in and names the deployed Tower engine: whoever turns
   keeping on carries the pin into it. An agent keeps an engine of an older
   protocol installed, but the gateway starts a Tower duel only on one of
   its own protocol, so a protocol bump strands the cohort in any case.
   Where the server's configuration is managed as code, that is where the
   pin goes too; pause enrollment before such a deploy.
3. **Binaries.** `baylee-tower-service` and `baylee-tower-provision`
   (`cargo build --release --locked -p baylee-tower-service -p
   baylee-tower-provision`, or the same commit built on the same OS and
   C library elsewhere) in `/opt/baylee-tower/bin`; a system user
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
   The publication is immutable, so before it compare a hash prefix of the
   seed with the one the calibration measured with, and rehearse the run
   with that seed on a throwaway PostgreSQL first: the stored builds' count
   of each relic rarity, potions and mean items per stage must equal the
   report's `loadout` (`docs/history/tower-calibration-2026-11-27/`), and
   the production builds must then be byte-equal to the rehearsal's.
5. **Central.** Provision the award issuer for `https://tower.baylee.app`
   with the migrator's connection (`central_identity.tower_award_issuers`:
   the SHA-256 digest of a fresh 32-byte `Token`'s 32 bytes, not of its hex
   text, and never the Courier's; a wrong digest stays silent until the
   first award), set `tower_origin` to `https://tower.baylee.app` in
   `central.json` (check it with `--check-accounts-stdin`), restart
   Central. Do this last: Central then shows the Tower to every client.
6. **Service.** `/etc/baylee-tower/service.json` (root 0600): the runtime
   URL, `"development": false`, and `service`: `listen` 127.0.0.1:28796,
   `origin` `https://tower.baylee.app`, `record_bytes`, one `workers` entry
   (the cohort, the retained engine, the verifier and supervisor
   credentials) and `awards` (the courier and the issuer credentials).
   Install `scripts/server/baylee-tower.service` and
   `scripts/server/tower.caddy`, `systemctl enable --now baylee-tower`,
   reload Caddy, and check `https://tower.baylee.app/health`. A Caddy
   reload closes the WebSockets of every site without `stream_close_delay`,
   the gateway's among them: reload in a window without games.
7. **Gateway.** `BAYLEE_TOWER_ORIGIN=https://tower.baylee.app` and
   `BAYLEE_TOWER_HOST_TOKEN=<host credential>` in the gateway's environment;
   the gateway restart ends running games, so it waits for an empty table
   list.
8. **Acceptance.** One duel played to its settled result, its award
   recorded in Central. Until then the switch can be taken back: remove
   `tower_origin`, restart Central, and pause enrollment
   (`--admin pause_enrollment`).

## Reproducible focused checks

With an owned synthetic PostgreSQL fixture, run the service's `admin` integration
target and binary tests, then the existing `bootstrap` and store `operators`
targets. The admin fixture exercises real child stdin/stdout, owner and restricted
SQL roles, every command, retry/immutability, live revocation and fixed diagnostics.
It bounds child output/deadlines, reaps commands and removes its owned schemas and
login role. Its all-basic-land publication is deliberately structural test data,
not a balanced release deck or approved content package.
