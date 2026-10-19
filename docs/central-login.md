# Central native login integration

The native client authenticates against `https://central.baylee.app` before it
loads saved gateways. `baylee-account` wraps the pinned Central SDK using a
transport that always verifies TLS, rejects redirects, bounds responses and
discards remote diagnostics. Passwords and central sessions are never forwarded
to arbitrary saved gateways. OPAQUE and authenticated client encryption are in
the reusable Central libraries, not a client-defined password hash scheme.

## Legacy migration

Only an explicit live Central discovery response permits the last legacy proof
to `https://baylee.acevik.de/auth/central/migrate`. Network failures, incorrect
passwords and development endpoints never permit that fallback. The gateway
checks the password, bans and forced-reset state, then reserves an immutable
central identity using its separately configured bridge credential.

The snapshot contains preferences, owned decks and deck version history. No
password hash or session token is exported. The client encrypts and enrolls the
snapshot, logs in through OPAQUE, downloads/decrypts it and validates the schema
before confirming restore. Source data is retained. Export, link and admission
use distinct one-use 30-second capabilities. Linking also checks the exact source
digest held in the restored document. Concurrent edits or another export cannot
silently authorize an older copy. Interrupted enrollment resumes centrally;
already-committed enrollment cannot be overwritten by another legacy login.

Cutover is serialized against ordinary gateway requests, reconciles the source,
fixes the legacy-to-central mapping and revokes legacy HTTP sessions atomically.
The same transaction retires the gateway's password verifier. A database
constraint prevents legacy reset code from restoring it. Migrations retain the
immutable mapping and refuse a rollback once credentials have been retired;
only unmigrated/test accounts can roll back normally. This cannot erase old
backups, which still need the operator's documented retention policy.
Linked accounts reject the old password routes even if the bridge is later
disabled. Lobby and chat sockets are asked to reconnect. Original gameplay data
and identifiers remain at the gateway; this first integration does not relocate
replay blobs, uploaded images, social history or device-local chat keys. A
short-lived, zeroized in-process password handoff can still unlock the existing
local chat vault after official gateway admission; it is never sent to that
gateway. This preserves local access, not cross-device chat key recovery.

The first snapshot format is deliberately bounded: at most 192 KiB and 2,048
source rows. Larger accounts fail before enrollment and keep their old account
and data. Chunked large-object migration is still required before admitting such
accounts; never work around this by dropping history or truncating an export.

## Settings and synchronization

The restored preference document replaces device defaults only after validation.
It is then written encrypted to Central, with compare-and-swap revisions and a
three-way merge for independent preference edits. Conflicting edits to the same
value are refused. Failed preference writes retain a separate
`preferences-unsynced.json` recovery copy in the settings directory. Gateway
session attachment does not reload the old gateway's preferences over Central's
copy. Guest tokens, monitor placement and other device-only configuration are
excluded from the central document.
The merge base tracks values submitted by the open settings form separately
from the latest remote document, so a later unrelated edit cannot revert an
earlier remote merge.

Central credentials and master keys live on a bounded serial worker. Account
sign-out drops the handles and asks the worker to revoke its central session.
The user may also leave a gateway while retaining central sign-in. Gateways must
be registered with Central before SSO admission; the initial live service
supports the official migration gateway only.

## Development and rollout

A `dev-control` native build accepts `BAYLEE_CENTRAL_DEV_ENDPOINT`, restricted to
an explicit numeric-loopback HTTP origin. Invalid values fail closed. This does
not enable a legacy password fallback. Use synthetic centrally enrolled accounts.
Without the explicit development override the compiled destination is live.
Browser authentication is still the existing separate flow; these SDK changes
target Windows, Linux and macOS.

The gateway bridge is opt-in through `BAYLEE_CENTRAL_CONFIG_FILE`. Its bounded
JSON has `endpoint` (`"live"` or a numeric-loopback development origin) and
`bridge` (a random 32-byte JSON array, shared with the Central operator). Store
it outside the repository, readable only by the service identity. Never put
the value in arguments, logs or client configuration. Unknown settings fail
startup. Without this file the migration routes reject requests.
Use `BAYLEE_BIND=127.0.0.1` for local gateway fixtures; the existing default
remains `0.0.0.0`. All test services must bind only loopback.

Deploy in order: Central with persistent OPAQUE setup and PostgreSQL 18+; the
official gateway bridge and schema; synthetic end-to-end validation; the native
client. Do not release a mandatory Central login while its live endpoint is
unavailable. Production TLS/provisioning, account recovery/reset and the large
object migration gates remain required; the development listener is not a
production launch procedure.

Tests: `cargo test -p baylee-account --all-features` covers actual loopback
transport boundaries, account schema and preference conflicts.
`./scripts/test-central-db.ps1` owns an isolated PostgreSQL 18.6 container and
runs the entire database schema suite, including source isolation, snapshot races,
immutable linking, revocation, verifier retirement/backfill and source retention.
It kills only its owned process tree/container and verifies cleanup,
including with `-ExerciseFailure`. Client UI tests must follow the dev-control
skill, use the extra left monitor, and close the exact owned client afterward.
`./scripts/test-central-integration.ps1` additionally runs a real gateway and
Central against that isolated database: source edits during enrollment,
operation-bound capabilities, cutover, old-login revocation, fresh-device restore,
repeated concurrent preference merges and gateway admission. Its optional
`BAYLEE_CENTRAL_UI_READY` path exposes only synthetic fixture addresses for at
most 180 seconds, until the sibling `.stop` file is written. Never use it with
production configuration.

`./scripts/test-central-client.ps1` exercises the actual native client against
that fixture. The 19 October 2026 Windows run passed: no saved gateways before
login, exactly the restored list afterward, Central sign-out clears access,
repeat login restores it again, and a one-use ticket obtains a gateway session.
Screenshots verified the login and restored-list layouts. Admission is asserted
from application state, not from a transitional frame. The harness reports
front-panel motion so the script waits until controls accept input. Exact
owned client PIDs, HTTP listeners and backend containers were closed/verified.

Current operational limits: Central sessions expire after 30 minutes without
background renewal; sign in again to resume synchronization. New account
registration, password changes/recovery and arbitrary-gateway registration are
not exposed by this first native integration. Never substitute the old gateway
password form for these missing Central operations.

## Follow-up review, 19 October 2026

The full Windows client suite exposed a local vault permission-check bug:
inherited private ACEs may be returned without the `AI` propagation flag. The
check now accepts both Windows representations and either order of the same two
allow entries, while rejecting other SIDs, extra/duplicate entries, changed
rights and missing entries. Owner and reparse-point checks remain mandatory.
The actual create/reopen/password-change flows and corruption refusal now pass.
References: [Security Descriptor String Format](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format),
[ACE Strings](https://learn.microsoft.com/en-us/windows/win32/secauthz/ace-strings).

Rust 1.99's alignment lint also flags existing SSE2 `loadu`/`storeu` pointer
casts, whose documented contract explicitly permits unaligned addresses. The
exception is scoped to those branches; no new unsafe or assembly was added.
Other portability fixes keep Unix-only test helpers and intentional non-Unix
error stubs correctly scoped. No cryptographic parameter was weakened.

The shared gateway test helper now binds loopback, removes inherited live bridge
and mailer configuration, and owns the process before readiness polling so a
startup failure also tears it down. Central end-to-end cleanup requires an actual
connection-refused result for both listeners; a timeout does not count. Its
five-second probe allows Windows' observed roughly two-second refusal delay.

The wider Baylee dependency scan is not green: the existing `ruzstd` and
`ttf-parser` exceptions are recorded in `deny.toml`, and RustSec additionally
reports the Linux keyring chain's `zbus_names`, `zvariant` and `zvariant_derive`
as unmaintained (RUSTSEC-2026-0371/0370/0372). These reports are not suppressed by
this change. A maintained, Linux-tested keyring dependency path remains a release
gate; passing Central's separate dependency checks does not clear Baylee's graph.
