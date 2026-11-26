# Database and catalog

## Tower Echoes authority (implementation in progress)

`baylee-tower-store` is a separate PostgreSQL 18+ adapter in Baylee. Its production
database/runtime role must be separate from gateway and Central storage. Tests use
unique owned schemas on loopback with synthetic data. The current adapter exposes
explicit operator migration/readiness, immutable cohort publication, enrollment,
owner reads and shop commands; there is no public unauthenticated listener.

Schema selection accepts only a bounded identifier and every table is qualified.
Runtime connections must disable SQL/value logging and bound pool/timeouts.
Migration is an explicit operator action, checks an exact recorded digest and
never drops/reset existing data. IDs are `DEFAULT uuidv7()` with `RETURNING`;
external account/request IDs must also be UUIDv7. Offline administration checks
effective membership in the schema owner's SQL role; ordinary table grants and
application tokens do not grant this capability. The final least-privilege runtime
grant inventory remains a release check. See [Tower operations](../tower-operations.md).

Owner mutations lock the account before its run. Request digest, original result,
wallet/inventory and revision commit together. Same key with different contents
refuses; the same request returns its original response after later mutations.
An early abandoned run retains its account/cohort starting offer seed. Starting
requires explicit acceptance of the current echo-use explanation. Immutable cohort
contracts cannot be republished with altered starter content; catalog/list ordering
is canonicalized. Per-run receipt count and payloads are bounded. Migration two
backfills exact retained receipt bytes/run counts and the version-one consent
timestamp, then bounds each account to 64 MiB of receipt data and 1,024 retained
runs. Quota charges share the mutation transaction; duplicate requests return
their prior receipt without charging again, including at the limit. Retention
must release these counters with deleted rows before this service is released.

Reservations/chunk ACK, approved operators, global pool indexing, verified
settlement/outbox and record-storage quotas now exist; retention is outstanding.
The adapter does not itself prove legal game records. The service
must use distinct Tower identity credentials, never arbitrary gateway admission.

Migration three adds dedicated Tower operator records with separate host,
verifier, publisher and supervisor roles. Provisioning remains an offline admin
operation; the transport will use the reviewed token library and rate limits.
Authenticated principal fields are private. Role/enabled state is rechecked when
publishing immutable, explicitly labelled house fallbacks or advertising an exact
local executable inventory. Registration at Central alone supplies no Tower role.
Concurrent inventory advertisements use one conditional update, avoiding a shared
row lock upgraded by multiple writers. Host and verifier operations recheck live
roles and fenced assignments; their public transport is still being integrated.

Migrations four through six persist immutable reservations, synchronously
committed chunk acknowledgements, independently fenced verification jobs,
transactional settlement/cosmetic outbox and dense same-stage echo partitions.
Migration seven records a closed technical verification-failure category.
Only the separate supervisor may quarantine a failed job, against its exact
claim/generation/commitment; an older verifier cannot quarantine newer work.
Recovery preserves the evidence and clears the old failure on a new generation.
No technical failure adds gold, a win, a defeat or an accomplishment. A completed
verification receipt can be recovered after response loss without re-awarding.

Migration eight records exact atomic record-batch boundaries. Native live hosting
uses `append_batch`: at most eight MiB/64 chunks and a complete final JSON line,
committed with quota/run/head/batch receipt together. A bulk row insert and one
quota/run write avoid a database round trip per chunk. Retrying a different batch
at the same start refuses. The single-chunk primitive remains for retained legacy
adapters/tests and must not be exposed as the new native operation boundary.

Migration nine adds the indexed execution lease watch, backfilled from existing
playing states. Each run write maintains it transactionally. A supervisor must
recheck the discovered fence and deadline under owner/run/reservation locks;
discovery alone grants no recovery authority. At most three automatic recoveries
retain the exact package, record prefix and original budget without rewards.

Migration ten stores explicitly approved host gateway origins separately from
advertised executable capabilities. Offline approval accepts canonical production
HTTPS or isolated numeric-loopback HTTP, refuses remote address changes and binds
each origin to one host. Owner-only location reads join enabled operators with
this approval and the current reservation/claim under transaction locks. No
unapproved address or historic receipt supplies a fallback destination.

Migration eleven indexes available host slots by the complete canonical executor
metadata and host UUID. At most sixteen entries per host are replaced in the
same transaction as its installed inventory. Their authority timestamp expires
after sixty seconds. Owner discovery joins this index with approved locations
and live operator enablement, using bounded descending cursor pages. Free capacity
and installed compatibility remain separate, so filling an agent cannot revoke
its ability to recover an existing frozen execution.

## Gateway and catalog

The gateway's store (`baylee-db`) and the optional card catalog
(`baylee-catalog`) share one PostgreSQL. What the gateway keeps about people
is inventoried in `docs/privacy.md`.

```bash
docker compose up -d
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
RUST_LOG=baylee_catalog=info cargo run -p baylee-catalog -- ingest   # all languages; --english-only opts out
# also: search "<q>", project, mine-types (writes data/type-names.tsv), corpus (ledger input)
```

- `compose.yaml` pins `name: baylee` so worktrees share one container; keep it (`COMPOSE_PROJECT_NAME`/`-p` still override).
- `baylee-db` is the gateway's store (accounts: guests, the projections of Baylee Central's accounts, and the accounts from before Central, which keep their game data here; sessions, decks, confirmations, preferences, upload owners; entities + migrator); no `DATABASE_URL`, no gateway. No account has a password here: migration 32 (`m20261119_000032_drop_account_password`) dropped `account.password_hash`. It destroys the hashes, and it steps back only on a database without a local account (neither a guest nor Central's), since the older schema holds one to a password (`docs/central-login.md` §"Cutover"). A leftover `gateway-store.json` is imported into an empty DB (its `automation` map and its password hashes are dropped), then moved aside.
- `DELETE /account` (#292): takes `{}` on the session alone; a password from a client older than the cutover is ignored. One statement, the rest by cascade. Every account deletion, the guest purge included, goes through `account::depart`, which holds `cosmetics::Store::hold` until the files of pictures nobody claims any more are gone; an upload writes its owner row before its file. The lock is per process: one image directory per gateway. A deck names only pictures its player uploaded (`check_pictures`, 403). As the gateway starts, before it serves, `account::sweep_pictures` removes every stored picture with no owner row that no deck names (#301), under the same lock, logging a count, never ids.
- The card catalog is optional (same Postgres, hand-written SQL); without an ingest games still run without card text: the client draws faces from the engine's projection. Ingest every language: the client asks `/catalog/text` in its own, English fallback per printing. Plain `RUST_LOG=info` logs every INSERT.
- `/catalog/text`, `/catalog/search`, `/pool`, `/pool/facts` and `/printings` answer signed-in sessions only, as `/art` does, and the gateway never asks Scryfall on a caller's behalf (Scryfall: no plain proxy or republish, #270); the client asks the lobby's gateway with its session, and signed in nowhere asks Scryfall itself. `docs/privacy.md` inventories what the gateway keeps about people.
- Search reads `card_search` (per oracle face; names, bigram GIN) and `card_search_text` (per face and language; `tsvector` GIN), not `cards`. A search reads the client's language and English and no other: a client reads two `card_search_text` rows of a card, and `named` drops a card reached only through a third language's name. `Catalog::project()` rebuilds both after ingest, `migrate()` when they are empty or the stamped `SCHEMA_VERSION` (`crates/baylee-catalog/src/lib.rs`) differs; bump that on a shape change.
- Never put name and rules-text matches in one predicate: union the tiers, resolve the printing after `LIMIT`. Names use bigrams, not `pg_trgm`.
- `card_legalities` is one row per card, the printings' verdicts merged: each format keeps the strongest word (`rows::VERDICT_ORDER`), because Old School answers per printing, and an ingest empties the table when its first batch is in hand so a lifted ban does not stand. `Catalog::pool_records` reads it and each card's distinct `(set, rarity)` pairs for `GET /pool/facts`; the gateway keeps that table per `data_version`.
- `banlists` and `banlist_entries` are the gateway's store (`baylee_db::banlists`, hand-written statements, migration 28), not the catalog's: an own list has an owner and survives an ingest. The official lists are `builtin` rows in the same table, written from `Catalog::official_lists` by `banlists::keep_official` when the catalog's stamp moves; a feed read twice moves no version.
- The projection joins `type_names` (committed `data/type-names.tsv`, keyed on English name, never `SubtypeId`) inside its `INSERT`, each word into its language's row; no second `UPDATE`, no padded `LIKE`. `mine-types` needs a full catalog and writes only fully understood pairs; ties: most frequent form, then newest printing.
- Migrations: `unaccent` `WITH SCHEMA public`; no unqualified `CREATE EXTENSION`/`DROP INDEX`; `information_schema` lookups filter `table_schema = current_schema()`. Two tests keep a schema behind the sandbox to catch escapes.
- `released_at` is `date`; render it with `to_char(released_at, 'YYYY-MM-DD')`, not `::text` (sqlx pins `DateStyle` to ISO today; the SQL should not depend on the driver); `finishes`, `frame_effects`, `rarity`, `layout`, `border_color` stay `text` deliberately.

## Ranked host bindings

`ranked_run` maps a Central match UUID to one immutable game UUID. In a
single-host match, `generation` and `primary_host` are null. The gateway
persists the binding before asking Central to start and consumes its launch
permission before exposing the opening. The completed engine record provides
terminal evidence. The delivery worker reads a bounded batch ordered by
service time and deletes acknowledged bindings.

Replicated matches use the Central match UUID as their game UUID on every
host. Each host stores its assigned generation and whether it is primary.
Retries preserve that binding, and promotion requires a higher generation.
Cleanup only removes the matching generation. The single-host delivery index
and all of its operations exclude these rows. The gateway's PostgreSQL
`schema::ranked` tests exercise both paths through migration 34.
When a host restarts after a match has already closed, the reconciliation
worker removes leftover rows after a day, using a complete and authenticated
list of assigned and pending matches.

Player aliases, seat capabilities and lease deadlines exist only in gateway
memory. Central owns the leagues, deck histories, seasons and rankings.

## Running games

`running_game` (migration 35) keeps each running casual game across a
restart of the gateway: one row a game, the gateway's JSON at a version of
its own (`keep::row::VERSION`, held by `the_row_keeps_its_shape`), written
only while the game's record is started and not complete. The record's last
piece removes the row in the same statement (`records::append`); the start
removes rows of complete records (`running::forget_finished`) and reads the
rest back (`keep::restore`). `records::stored_pieces` refuses a record that
misses a piece, so a resume never goes on from a gap. The `schema::running`
tests exercise it.
