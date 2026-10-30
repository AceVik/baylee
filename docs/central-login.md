# Central native login integration

The native client signs in at `https://central.baylee.app` for a gateway that
takes Central sign-ins: `central: true` in its `GET /auth/config`, which a
gateway with `BAYLEE_CENTRAL_CONFIG_FILE` answers. Choosing one turns the
account form into Central's; the sign-in restores the account's gateways and
admits the player to the one chosen. Every other gateway signs people in
itself, with sign-up and guests where it offers them. `‹` on Central's form
goes back to the device's own gateways, and so does the end of a Central
session. The official gateway is bound to Central through its bridge since
beta.8a (22.10.2026): new accounts register at baylee.app, and an account from
before moves to Central at its first sign-in in the client (§"Legacy
migration"). `baylee-account` wraps the pinned Central SDK using a
transport that always verifies TLS, rejects redirects, bounds responses and
discards remote diagnostics. Passwords and central sessions are never forwarded
to arbitrary saved gateways. OPAQUE and authenticated client encryption are in
the reusable Central libraries, not a client-defined password hash scheme.

## Bigint advisory scope

[RUSTSEC-2026-0387](https://github.com/RustSec/advisory-db/blob/main/crates/crypto-bigint/RUSTSEC-2026-0387.md)
reports variable-time arithmetic in `crypto-bigint`, fixed in 0.7.6. The
current dependency chain contains 0.5.5 through `elliptic-curve` 0.13.8,
`voprf` 0.5.0 and `opaque-ke` 4.0.1. The elliptic-curve dependency requires
bigint 0.5, so changing the lockfile to 0.7.6 is not a compatible upgrade.

The specific Central revision
`59d833ba425f941c793e8de88292b57f95c20e45` contains the same `central-crypto`
source tree reviewed independently by two models on 24.10.2026. The 28.10.2026 pin added the memorial website and a
fail-closed source/graph guard for Central's matching advisory exception;
the 30.10.2026 pin adds gateway tokens, the account area and the
administration (Central #48, #49) over password change, roles, reset,
deletion and export (#42 to #47). `git diff 3ae5035 59d833b --
crates/central-crypto` is empty: the entire crypto crate is unchanged. Argon High
reviewed the reached Ristretto/hash-expansion sources again without a
confirmed finding. Its private, fixed `central-crypto::opaque::Suite`
selects Ristretto255 for OPRF and Triple-DH, with SHA-512. Both upstream
Ristretto implementations use `curve25519-dalek` points and scalars for the
secret arithmetic, including reduction and inversion. The reached
`elliptic-curve::hash2curve::ExpandMsgXmd` helper and its domain handling use
hashes, byte arrays, XOR and public lengths; they perform no bigint arithmetic.
Its `U256` is a typenum length, not `crypto_bigint::U256`. The affected
arithmetic is therefore unreachable through this reviewed fixed suite; this
is not a general safety claim for bigint 0.5.5.

`deny.toml` and the audit job exclude this advisory only under that reviewed
scope. The required `shipped-features` job and local feature gate run
`crypto-bigint-scope`: Cargo metadata resolves all features, including inactive
optional consumers, without compiling them. It must name the exact registry versions
above and the exact Central source revision. Reverse dependencies must remain
`crypto-bigint ← elliptic-curve ← opaque-ke/voprf ← central-crypto`, with no
additional consumers. Changed sources, versions or consumers fail the gate
and require re-review or removal of the exception. The guard checks the
resolved dependency graph without building the client or changing crypto.

Sources reviewed: Central's pinned `crates/central-crypto/src/opaque.rs`;
`opaque-ke`'s `key_exchange/group/ristretto255.rs`; `voprf`'s
`group/ristretto.rs`; and `elliptic-curve`'s hash-to-field message-expansion
and domain implementations. The lasting upgrade path is a compatible
upstream SDK migration to a fixed bigint dependency, with protocol and
stored-credential compatibility checked before changing the suite.

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
The preparation also binds the current gateway password verifier internally;
the verifier never leaves the gateway. Changing or administratively resetting a
legacy password first invalidates pending Central enrollment under the same
exclusive migration barrier. Central archives the encrypted previous envelope,
revokes its sessions and requires a new password proof. If Central is unavailable,
the password mutation fails closed. A source/hash check precedes `LegacyClose`;
only then does the local transaction retire the verifier. If that transaction
fails, a valid OPAQUE login can retry the idempotent link. A closed migration is
never reopened. Completed older migrations repeat this link without exporting
or rewriting their restored data, to establish the closure marker after upgrade.
Older pending preparations retain a marker with an unknown password generation.
Only a typed `428` response from the pinned migration gateway permits one fresh
password proof, bound to the current Central Link capability and immutable legacy
account ID. This reestablishes the source proof without replacing Central credentials.
Network errors, ordinary conflicts, revoked sessions and completed migrations do
not trigger that fallback. Once Central confirms migration closure, future logins
do not contact the legacy gateway.

Link and export capabilities are gateway tickets, which Central refuses until
its own terms in force are accepted. A sign-in that finds them unaccepted
therefore succeeds with the migration held; the client shows those terms, and
their acceptance resumes the migration before any gateway admission
(`docs/protocol.md` §"Terms of use (WG-1)"). A migration that fails then ends
the Central sign-in, as at sign-in; the next one resumes it.

Cutover is serialized against ordinary gateway requests, reconciles the source,
fixes the legacy-to-central mapping and revokes legacy HTTP sessions atomically.
The same transaction retires the gateway's password verifier. A database
constraint prevents legacy reset code from restoring it. Migrations retain the
immutable mapping and refuse a rollback once credentials have been retired;
only unmigrated/test accounts can roll back normally. This cannot erase old
backups, which still need the operator's documented retention policy.
Linked accounts reject the old password routes even if the bridge is later
disabled. Lobby and chat sockets are asked to reconnect. Original gameplay data
and identifiers remain at the gateway; this integration does not relocate
replay blobs or uploaded images. Native chat keys and complete retained private
history are backed up in the client-encrypted Central account archive and
restored without the source device. The sole active account session resumes one
logical chat identity per gateway. A short-lived, zeroized password handoff imports an existing
local vault without modifying its source; it is never sent to a gateway.
Synchronization is asynchronous; delivery acknowledgements wait for a confirmed
backup, so a new installation can replay outstanding messages. See [encrypted chat synchronization](central-chat-sync.md) for
the manifest protocol, recovery checks and remaining limits.

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
The user may also leave a gateway while retaining central sign-in. There is one
active Central session per account. A successful replacement login atomically
revokes previous Central sessions, tickets and gateway leases; failed login
does not sign out the current device. Every authenticated gateway HTTP request
checks its server-only lease with Central, without an authorization cache. Local
activity cannot extend that lease's Central expiration. Idle lobby, chat, watch
and game sockets recheck every five seconds and close when authorization fails;
every received binary game frame is checked before forwarding to the engine.
Already-authorized operations can finish concurrently with revocation. A seat
capability is bound to the session that minted it and cannot inherit the new
device's authorization when that device reclaims the seat.

Registered gateways use distinct redemption and heartbeat credentials. Operator
approval of the claimed origin is a separate database privilege; an ordinary
service or gateway owner cannot approve itself. The gateway publishes its
compiled version and configured website every 30 seconds. Directory metadata
is display data and never a destination for passwords. Foreign Central users
receive a gateway-local projection identified only by their immutable Central
UUID. A matching local username/e-mail never links accounts. At each sign-in
the projection takes the display name and tag the account has at Central
(`GatewayProfile`), so a player is the same `name#tag` here as on the website;
a name this gateway's rule refuses, or a tag another local account holds, keeps
the local value. New gameplay
projections follow `BAYLEE_REGISTRATION`; existing users and explicitly named
gateway admins remain subject to local bans and forced-reset policy.

Gateway admin SSO exchanges a purpose-bound Admin ticket at
`/auth/central/admin` for a separate local bearer. That bearer works only on the
existing loopback admin listener, whose browser/header restrictions still apply.
Every admin request checks both the active Central session and current gateway
admin membership. Removing a member invalidates existing admin bearers. The
audit actor is the Central UUID, not an asserted header. The operator bootstrap
token remains a separate administrative route; neither mode receives a Central
account session token. The gateway-owning account must also be explicitly listed
as an admin if it needs SSO administration.

## Linking a gateway with a token

*30.10.2026.* A self-hosted gateway links to Baylee Central without an
operator file: its owner registers the gateway's origin on baylee.app
(`/account/gateways`), an administrator of Central approves it, and the owner
makes a **gateway token** there (baylee-central `docs/gateway-registry.md`
§"Gateway tokens"): named, scoped (`sso` signs players in, `listing` keeps the
directory entry), expiring within a year, revocable, shown once, stored at
Central as its digest. An admin pastes it into the gateway's console
(`/admin/central`, through the feedback service's UI:
`docs/protocol.md` §"Linking the gateway to Baylee Central").

- The gateway asks Central what the token is (`GatewayStatus`: the
  gateway's id, origin and metadata, its approval, the token's scopes and
  expiry) before it keeps it; an unknown, revoked or expired token is
  refused (`422`).
- It keeps the token in its database (`central_link`, migration 000025),
  because it presents the token to Central and so cannot keep a digest, and
  uses it at once: the bridge is replaced while the gateway runs
  (`central::Link`), every request asks for the bridge in use, and a request
  in flight finishes with the one it started with. The token never reaches a
  log or an answer; the console sees its last four characters.
- **Backups hold the token.** A dump, snapshot or backup of the gateway's
  database holds the token as the database does, in plain text. An operator
  who loses one, or cannot say where one went, revokes the token on the
  website (`/account/gateways`) and pastes a new one; the console's
  **Remove the token** forgets the old one at the gateway.
- One token serves both its players' sign-ins (`RedeemGateway`,
  `GatewaySession`, `GatewayProfile`, `GatewayAuthorize`,
  `PublishGatewayTerms`) and the heartbeat, for what its scopes allow; the
  heartbeat names the gateway by `BAYLEE_GATEWAY_NAME` (Central's registered
  name when unset or refused) and its own version, on its origin.
- `BAYLEE_CENTRAL_ENDPOINT` (`live` unless a numeric-loopback development
  origin) names where a pasted token talks to.
- `BAYLEE_CENTRAL_CONFIG_FILE` takes precedence: the console shows a link
  from it and cannot change it (`409`). The official gateway keeps its
  bridge file.
- Removing the token unlinks at once: nobody signs in through Central
  there, those who did are signed out at their next check, and the entry
  goes offline. The token stays valid at Central until it is revoked on the
  website.

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
The official gateway admits a migrated account as the legacy account it was
linked to, and an account registered at Central (no legacy account) as a new
local projection, as a registered gateway does; Central's beta key stood in for
the gateway's at sign-up, so only `BAYLEE_REGISTRATION=off` refuses it
(baylee-central `docs/web-accounts.md`).
A moved account keeps its display name and tag at Central (baylee-central
`docs/native-login.md` §"The bridge's reservation carries the name and
tag"): the bridge sends both with `LegacyReserve` at the move, the name only
when it keeps `auth::valid_display_name` (a name from before that rule, or
from `gateway-store.json`, stays behind, and the account chooses one on the
website), and the tag always. An account that moved before Central kept them
gets them at its next Central sign-in here: `login_for` sends
`LegacyProfile` for a linked legacy account at every such sign-in, since
this side records nothing about it; Central sets name and tag only while the
account has no tag, answers `Conflict` after that, and any answer leaves the
sign-in as it is. No background job: every moved account that still plays
signs in here. Showing Central's names on gateways is the accounts design's
step 4; until then a gateway shows its own.
The official gateway is listed in Central's directory (`baylee.app/gateways`,
marked "Official") once Central's operator has listed it (baylee-central
`docs/operations.md` §"The official gateway"). Its bridge file then gains
`metadata` (`name`, `version`, `website`) beside `bridge`, for example
`"metadata": {"name": "Baylee", "version": "0", "website":
"https://baylee.acevik.de/"}`: the gateway sends `GatewayHeartbeat` with the
bridge credential every 30 seconds, Central updates the official entry and no
other, and the entry shows online. The website must lie on the bridge's origin,
the compiled `https://baylee.acevik.de` live; in development that is `origin`,
which a bridge file with `metadata` must then name. `version` is replaced with
the running binary's version. Without `metadata` the bridge sends no heartbeat
and the entry stays listed and offline.
Registered gateways instead use `endpoint`, canonical `origin`, `credential`,
`heartbeat` and `metadata` (`name`, `version`, `website`). Both credentials are
independent random 32-byte JSON arrays returned once by registration. Their
purpose is enforced centrally; `version` is replaced with the running binary's
version. Never combine `credential` or `heartbeat` with `bridge`: the official
legacy bridge alone may reserve/export/close old account migrations. The reusable Central SDK
offers registration, directory paging and versioned admin-list management; no
registration or admin-management UI is implied by these server APIs.
Use `BAYLEE_BIND=127.0.0.1` for local gateway fixtures; the existing default
remains `0.0.0.0`. All test services must bind only loopback.

Deploy in order: Central with persistent OPAQUE setup and PostgreSQL 18+; the
official gateway bridge and schema; synthetic end-to-end validation; the native
client. Do not release a mandatory Central login while its live endpoint is
unavailable; a gateway turns Central sign-in on for itself by its
configuration, so the official one turns it on only once the rest is ready. Production TLS/provisioning, account recovery/reset and the large
object migration gates remain required; the development listener is not a
production launch procedure.

Tests: `cargo test -p baylee-account --all-features` covers actual loopback
transport boundaries, account schema and preference conflicts.
`cargo dev central db` owns an isolated PostgreSQL 18.6 container and
runs the entire database schema suite, including source isolation, snapshot races,
immutable linking, revocation, verifier retirement/backfill and source retention.
Its optional `-GatewayTests @('e2e_account', 'e2e_admin', 'e2e_social_messages')`
adds only named, allowlisted gateway regression targets after the schema suite.
Each target runs serially and must report executed tests; this mode reserves a
bounded 1 GiB PostgreSQL filesystem for repeated schema/WAL work and checkpoints
between targets. No production database URL is inherited by either mode.
It kills only its owned process tree/container and verifies cleanup,
including with `--exercise-failure`. Client UI tests must follow the dev-control
skill, use the extra left monitor, and close the exact owned client afterward.
`cargo dev central integration` additionally runs a real gateway and
Central against that isolated database: source edits during enrollment,
operation-bound capabilities, cutover, upgrade-era source-proof recovery,
old-login and game-seat revocation, fresh-device restore, preference reconciliation,
registered-gateway admission and the official gateway's heartbeat in bridge
mode. Its optional
`BAYLEE_CENTRAL_UI_READY` path exposes only synthetic fixture addresses for at
most 180 seconds, until the sibling `.stop` file is written. Never use it with
production configuration.

For coordinated local development of the sibling repositories, both database
scripts accept `--cargo-config target/central-patch.toml`, pointing to a Cargo
patch configuration for the sibling Central crates. This explicit mode permits
lockfile resolution of local paths; the default still uses `--locked` with the
pinned Git revision. Keep the patch file local and restore final dependency pins
before commit. For Clippy, pass the option after the subcommand:
`cargo clippy --config target/central-patch.toml ...`; Cargo's external Clippy
subcommand does not preserve this global option in the same way as built-in
`check`/`test`. Both test scripts own their temporary PostgreSQL fixture and
verify cleanup on success, failure and cancellation.

`cargo dev central client` exercises the actual native client against
that fixture. The 19 October 2026 Windows run passed: no saved gateways before
login, exactly the restored list afterward, Central sign-out clears access,
repeat login restores it again, and a one-use ticket obtains a gateway session.
Screenshots verified the login and restored-list layouts. Admission is asserted
from application state, not from a transitional frame. The harness reports
front-panel motion so the script waits until controls accept input. Exact
owned client PIDs, HTTP listeners and backend containers were closed/verified.

Sessions renew (since beta.8b): the account worker renews its Central session
every ten minutes (`Account::renew`; a failure is retried a minute later and
ends nothing), so it, and every gateway session admitted from it, lasts while
the client runs, up to 12 hours after the password sign-in (baylee-central
`docs/operations.md` §"Sessions"). The gateway takes the later lease expiry
Central reports on its next check and stores it on the lease, which bounds
the session (`central::authorize`, `baylee_db::central::extend_session`); an
earlier one is refused. A gateway
built before this compares the expiry exactly and so ends a session at its
first renewal: gateways update together with Central.

Current operational limits: after the 12 hours, sign in again. New account
registration and password changes/recovery are not exposed by this native UI.
Gateway registration/admin APIs are available in the reusable Central SDK, with
operator approval still required. Never substitute the old gateway
password form for these missing Central operations.

### Deployment handoff to the owner's MacBook

The owner chose on 19 October 2026 to continue production provisioning from the
MacBook, where the SSH identity for `viktor@acevik.de` is configured. This Windows
review performs no production deployment or live-user migration. Central's
reviewed integration is on public `main` at
`21d7e120120c1e5f5e4bdd2b487519384fabaac6` (registration with beta keys and the
browser listener, 21.10.2026); Baylee pins that exact revision.

Before publishing a client release, implement and review the production account
launcher and database connector. Central currently exposes the isolated
`--dev-accounts-stdin` launcher; putting that development process behind a public
reverse proxy is not a production setup. Preserve live issuer/audience checks,
verified database TLS, least-privilege runtime grants, persisted OPAQUE/bridge
secrets, backup/restore evidence and bounded login/discovery abuse controls.
Provision `https://central.baylee.app`, then deploy the compatible gateway bridge
and perform staging migration/restore checks before any real-account cutover.
See Central's `docs/status.md`, `docs/native-login.md`, `docs/postgresql.md` and
`docs/gateway-registry.md` for the existing boundaries and exact grants.

The private deck/replay library is a separate unfinished requirement; the
encrypted legacy deck snapshot is not a replacement for its operational CRUD.
The next implementation contract and source-size migration gap are recorded in
[the private library plan](proposals/central-private-library-plan.md).

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

The Linux keyring chain initially blocked the dependency check because
`zbus_names`, `zvariant` and `zvariant_derive` were unmaintained
(RUSTSEC-2026-0371/0370/0372). The shared `baylee-keyring` adapter now removes that
chain and uses the system libsecret provider through safe Rust APIs. Its isolated
Linux fixture verifies compatibility with old entries, round trips and refusal
of ambiguous matches; see [native credential storage](native-credential-store.md).
The final graph passes the advisory check with the existing, separately recorded
`ruzstd` and `ttf-parser` exceptions in `deny.toml`; no new advisory suppression
was added. Central's separate graph is checked independently.

The broader workspace run completed in package groups after its first temporary
PostgreSQL filesystem filled. Fourteen failures in three test targets were
reproduced and fixed: Windows drive/verbatim prefixes in the private-output
fence, platform-neutral generated-card paths, Node worker loading, isolated CLI
temporary directories, and a seat socket dropping TCP before its closing
handshake. The socket now drains only during a one-second deadline, with byte
and message limits, without forwarding any further input to the engine. This
follows the closing sequence in [RFC 6455, section 7.1.2](https://www.rfc-editor.org/rfc/rfc6455#section-7.1.2).
The regression also withholds the peer's closing reply and verifies that the
server still terminates the connection.

Targeted reruns passed all 258 developer-tool tests, 34 CLI tests and both real
gateway frame/rate tests; their temporary PostgreSQL container/listener was
removed. The web admin's shadowed translation name was corrected, and all 276
web tests, type checks, strict lint and production build passed. A Unix-only
vault migration test now imports its filesystem trait explicitly; its Linux
verification belongs to CI. These results do not claim a production rollout or
an independent security audit.

## Account-bound game recovery

A replacement login invalidates the old Central session but preserves the
gateway's game and account-bound chair. The fresh client discovers its own
resumable seats through authenticated `/lobby/seats`, even if public lobby
pagination hides the game. Claiming that chair rotates its capability; socket
generations prevent delayed traffic or shutdown from an older connection from
affecting its replacement. Ordinary window close, crash and updater restart
disconnect without an explicit table departure.

The active-game hold belongs to one gateway. Leaving a multiplayer table keeps
the participation while other humans still play; confirmed concession releases
it immediately. Explicit departure against only AI ends the game. The engine
provides authoritative participation endings, and admission rechecks them under
the same lock as starting another game. See the protocol's seat-discovery and
leaving sections for wire details and bounds.

The complete pre-recovery baseline at `b31289052` passed 20,158 nextest tests
(23 skipped), the remaining feature gates and the isolated Windows UI smoke.
At the owner's request, final recovery changes are verified only through the
affected client, engine, protocol, gateway and real Central integration tests;
the full workspace matrix is not repeated locally.
