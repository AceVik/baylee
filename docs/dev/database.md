# Database and catalog

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
- `baylee-db` is the gateway's store (accounts: login by username, email optional, guests; sessions, decks, confirmations, preferences, upload owners; entities + migrator); no `DATABASE_URL`, no gateway. A leftover `gateway-store.json` is imported into an empty DB (its `automation` map is dropped), then moved aside.
- `DELETE /account` (#292): a registered account names its password again (403 if wrong; the sign-in limiter counts it under `account:{id}`), a guest sends `{}`; one statement, the rest by cascade. Every account deletion, the guest purge included, goes through `account::depart`, which holds `cosmetics::Store::hold` until the files of pictures nobody claims any more are gone; an upload writes its owner row before its file. The lock is per process: one image directory per gateway. A deck names only pictures its player uploaded (`check_pictures`, 403). As the gateway starts, before it serves, `account::sweep_pictures` removes every stored picture with no owner row that no deck names (#301), under the same lock, logging a count, never ids.
- The card catalog is optional (same Postgres, hand-written SQL); without an ingest games still run without card text: the client draws faces from the engine's projection. Ingest every language: the client asks `/catalog/text` in its own, English fallback per printing. Plain `RUST_LOG=info` logs every INSERT.
- `/catalog/text`, `/catalog/search`, `/pool` and `/printings` answer signed-in sessions only, as `/art` does, and the gateway never asks Scryfall on a caller's behalf (Scryfall: no plain proxy or republish, #270); the client asks the lobby's gateway with its session, and signed in nowhere asks Scryfall itself. `docs/privacy.md` inventories what the gateway keeps about people.
- Search reads `card_search` (per oracle face; bigram and `tsvector` GINs), not `cards`. `Catalog::project()` rebuilds it after ingest, `migrate()` when it is empty or the stamped `SCHEMA_VERSION` (`crates/baylee-catalog/src/lib.rs`) differs; bump that on a shape change.
- Never put name and rules-text matches in one predicate: union the tiers, resolve the printing after `LIMIT`. Names use bigrams, not `pg_trgm`.
- The projection joins `type_names` (committed `data/type-names.tsv`, keyed on English name, never `SubtypeId`) inside its `INSERT`; no second `UPDATE`, no padded `LIKE`. `mine-types` needs a full catalog and writes only fully understood pairs; ties: most frequent form, then newest printing.
- Migrations: `unaccent` `WITH SCHEMA public`; no unqualified `CREATE EXTENSION`/`DROP INDEX`; `information_schema` lookups filter `table_schema = current_schema()`. Two tests keep a schema behind the sandbox to catch escapes.
- `released_at` is `date`; render it with `to_char(released_at, 'YYYY-MM-DD')`, not `::text` (sqlx pins `DateStyle` to ISO today; the SQL should not depend on the driver); `finishes`, `frame_effects`, `rarity`, `layout`, `border_color` stay `text` deliberately.
