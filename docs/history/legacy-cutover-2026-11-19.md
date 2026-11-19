# The legacy accounts' cutover, and its removal from the gateway (19 November 2026)

On 30.10.2026 the owner moved the sign-in of the accounts from before Baylee
Central, and their move to Central, out of the official gateway and into
Central. This note keeps the procedure and the export format that
`docs/central-login.md` §"Cutover" described while the cutover was still to be
run. It is history: the tool is gone and the procedure cannot be run from
`main` any more.

What is still true lives in `docs/central-login.md` §"Legacy migration" and
§"Cutover": Central signs the old accounts in from its legacy store until the
end of 30.06.2027 (Europe/Berlin), and the gateway links each one to its row
here at its first admission.

## What was removed, and where it last was

On 19.11.2026 the owner confirmed that the old accounts' data is in Central's
legacy store and that the gateway is to carry none of the cutover any more.
A commit on `main` that still has the tool is `5658da1f0`:

- `crates/baylee-gateway/src/legacyexport.rs`: the command
  `baylee-gateway legacy export`, its document and digest, and its tests.
- `baylee_db::central::legacy_accounts` (`crates/baylee-db/src/central/legacy.rs`):
  the one consistent read the export was built from.
- `auth::hash_password` (test only) and the gateway's `argon2` dependency.
- The column `account.password_hash`, dropped by migration
  `m20261119_000032_drop_account_password`. The hashes at the gateway are
  destroyed by it; the copies in Central's legacy store are the only ones.

Central keeps its own side (`central-service --legacy-import-stdin`, the
legacy store, the legacy sign-in and the move); see baylee-central
`docs/operations.md` §"The legacy store" and `docs/native-login.md` §"Legacy
login".

To read the removed code: `git show 5658da1f0:crates/baylee-gateway/src/legacyexport.rs`.

## The procedure, as it was run

The move was one maintenance window, the operator's, in this order, so that
no password could change at the gateway after its hash was copied:

1. Stop the official gateway and deploy the release without its password
   routes; let no player in yet.
2. Export and import in one pipe, with the gateway's `DATABASE_URL`:
   `baylee-gateway legacy export | central-service --legacy-import-stdin`.
   The export opened the database without migrating it, read every account
   that was no guest, had a password hash and no link, in one repeatable-read
   transaction, and wrote the document to standard output only, never to a
   terminal. One account that could not move (no username, a snapshot over
   the bound, a hash under other parameters) refused the whole export and was
   named by its id.
3. Compare the two lines on standard error: the export's count (with how
   many must choose a new password and how many are banned) and digest, and
   the import's count and digest, recomputed from what Central stored. They
   were equal, or the import committed nothing.
4. Start Central with its legacy store, then the gateway, then publish the
   client. A client from before still posted to the gateway's
   `/auth/central/migrate`, which was gone: it had to update, and the updater
   needs no sign-in.

The release's migration `m20261030_000026_end_password_sessions` ended every
password session at the gateway's first start.

## The export document

The document was `baylee-legacy-accounts/1`: `{"format", "accounts": [{
"legacy_id", "username" (its key), "display_name" (only one that keeps the
rule, else `null`), "tag", "password": {"salt": 16 bytes, "hash": 32 bytes},
"terms_version", "must_change_password", "ban": null | {"until": Unix seconds
| null}, "snapshot": the source snapshot's bytes}], "count", "digest"}`, bytes
as JSON arrays of numbers. Nothing else left the gateway: no address, no
session, no chat key.

Only hashes written as `$argon2id$v=19$m=19456,t=2,p=1$` with a 16-byte salt
and a 32-byte output were taken: the parameters the gateway hashed with
(argon2's defaults), and the only ones Central verifies.

The digest was SHA-256 over the count as eight big-endian bytes, then each
account in ascending `legacy_id` byte order as `f(legacy_id) f(username)
o(display_name) o(tag in decimal) f(salt) f(hash) o(terms_version)
byte(must_change_password) o(ban: "forever" or its until in decimal)
f(snapshot)`, where `f(x)` is `x`'s length as four big-endian bytes followed
by `x`, and `o(x)` is one zero byte for none or one byte `1` followed by
`f(x)`. Central's import computes the same (`central_postgres::legacy_digest`).

The source snapshot was the account's preferences document and its decks with
their version history, as JSON text, at most 192 KiB and 2,048 source rows. An
account over that bound refused the whole export.

## What a gateway operator must know now

- A gateway that still had password accounts and never ran the export has no
  way left to move them: no build from `main` exports them, and migration 32
  destroys their hashes at the first start. An operator in that position
  stays on a build up to `5658da1f0`, or runs the export from such a build
  before upgrading.
- A `gateway-store.json` imported from now on (`baylee_db::import`) gives
  accounts with their decks and settings but no password: nothing signs in to
  them at the gateway, and nothing exports them to Central.
- The store's deadline is Central's: after 30.06.2027 Central deletes what is
  left of the legacy store, and an account that has not moved by then has no
  way to Central from its old password. Its row and game data stay at the
  gateway until an admin deletes them.
