# Central native login integration

The native client signs in at `https://central.baylee.app` for a gateway that
takes Central sign-ins: `central: true` in its `GET /auth/config`, which a
gateway with `BAYLEE_CENTRAL_CONFIG_FILE` and a mode that takes Central's
accounts answers (§"Who a gateway lets in"). Choosing one turns the
account form into Central's; the sign-in restores the account's gateways and
admits the player to the one chosen. Since the cutover (§"Cutover") nobody
registers or signs in with a password at any gateway. `‹` on Central's form
goes back to the device's own gateways, and so does the end of a Central
session. The official gateway is bound to Central through its bridge since
beta.8a (22.10.2026): new accounts register at baylee.app, and an account from
before moves to Central at its first sign-in in the client (§"Legacy
migration"). `baylee-account` wraps the pinned Central SDK using a
transport that always verifies TLS, rejects redirects, bounds responses and
discards remote diagnostics. Passwords and central sessions are never forwarded
to arbitrary saved gateways. OPAQUE and authenticated client encryption are in
the reusable Central libraries, not a client-defined password hash scheme.

## Who a gateway lets in

Each gateway's operator chooses (owner's decision of 30.10.2026): its auth
mode is `guests` (guests only), `central` (Baylee Central's accounts only)
or `mixed` (both), and with nobody's choice it is `central`, the official
gateway included. The mode holds whether or not the gateway is connected to
Central: the connection (a token pasted into the console, §"Linking a
gateway with a token", or the operator's `BAYLEE_CENTRAL_CONFIG_FILE`) is
how a Central sign-in reaches the gateway today, and how it is listed, and
says nothing about guests.

- **Where it is chosen.** An admin chooses it in the console (*Who comes
  in*, `PUT /admin/auth-mode`), without a file or a restart; the gateway
  keeps the choice in its database (migration 000027, one row, with who set
  it and when), and the feedback service writes an audit row for it.
  `BAYLEE_AUTH_MODE` takes precedence where the operator sets it: the
  console then shows the mode as set by the environment and changes
  nothing (`409`). A value of it that names no mode stops the gateway at
  startup.
- **At the door, not after it.** A change applies to the sign-ins from then
  on. `POST /auth/guest` answers `403` `this gateway takes no guests` where
  the mode takes none, and `POST /auth/central/login` a bare `403` where it
  takes no Central account, before it asks Central anything. A session
  already made goes on until it ends, as it did under `BAYLEE_GUESTS=off`,
  which the mode replaces (the gateway warns at startup when that variable
  is still set, and reads it no more). The console's own Central sign-in
  (`/auth/central/admin`) is no player's door and follows no mode.
- **What a client is told.** `GET /auth/config` says `auth_mode`,
  `guests_enabled` (the mode takes guests) and `central` (connected, and the
  mode takes Central's accounts); `GET /info` says `auth_mode` and
  `guests`; the console's overview says `auth_mode` and where it comes from
  (`auth_mode_source`). A client offers what these say: signed out, a
  `mixed` gateway's form is Central's sign-in with the gateway's own *Play as
  guest* beside it, which turns Central's form off again and opens the
  gateway's guest face, the gateway still chosen (`lobby/central.rs`
  `guest_instead`). A guest door for a player signed in at Central comes with
  the device list (a later release).
- **A gateway nobody can enter.** Until a Central sign-in reaches a gateway
  without a connection (the short-lived identity proof, a later release),
  `central` on an unconnected gateway takes nobody. The gateway says so in
  its log at startup, and the console's overview and *Who comes in* page
  say so with the two ways out: paste a Central token (the console's Baylee
  Central page) or allow guests.

## Bigint advisory scope

[RUSTSEC-2026-0387](https://github.com/RustSec/advisory-db/blob/main/crates/crypto-bigint/RUSTSEC-2026-0387.md)
reports variable-time arithmetic in `crypto-bigint`, fixed in 0.7.6. The
current dependency chain contains 0.5.5 through `elliptic-curve` 0.13.8,
`voprf` 0.5.0 and `opaque-ke` 4.0.1. The elliptic-curve dependency requires
bigint 0.5, so changing the lockfile to 0.7.6 is not a compatible upgrade.

The specific Central revision
`0a941953f8c0d5573e82e0979e7faf1fb9e40299` contains the `central-crypto`
source tree reviewed independently by two models on 24.10.2026, with two
additions. The 31.10.2026 pin (Central #54, and #52's version) adds the
pairing of the website with this client: `central-crypto::pairing` seals the
master key with RFC 9180 HPKE (hpke 0.15.1 over x25519-dalek 3.0.0 and
curve25519-dalek 5.0.0, which Baylee's graph already held; the owner's
approval of 30.10.2026), and `vault::derived` derives the key check and a
local vault's key with HKDF-SHA-512. Neither touches the OPAQUE suite, and
none of those crates depends on `elliptic-curve` or `crypto-bigint`; a second
model reviewed the pairing, and Central's guard pins the fingerprint again.
The 30.10.2026 pin added the legacy store's sign-in, whose
`central-crypto::legacy` (behind `server`, which the client does not enable)
derives the official gateway's Argon2id with argon2 0.5.3, the version
opaque-ke already uses, and touches no suite, group, OPRF or key-exchange
type. An independent agent reviewed that change for this scope without a
finding. The same day argon2's `zeroize` feature came on (owner's approval),
named by `central-crypto` directly: argon2 wipes its own blocks for that
check and the OPAQUE stretch alike, and the lockfile gains only the edges
`argon2 → zeroize` and `central-crypto → argon2`. Central's own guard pins
the crate's new fingerprint (baylee-central
`docs/crypto-proposal/bigint-advisory.md`). The 28.10.2026 pin had added the
memorial website and that fail-closed source/graph guard for Central's
matching advisory exception. The 30.10.2026 pins also bring gateway tokens,
the account area and the administration (Central #48, #49), the website's
follow-ups (#50) and the official gateway's move to gateway.baylee.app with
sign-in by token before approval (#51), over password change, roles, reset,
deletion and export (#42 to #47); `git diff 3ae5035 0f8005d --
crates/central-crypto` is empty, so the crypto crate changes only by the
two additions above. Argon High
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

*Baylee Central took the old accounts' sign-in and their move over from the
official gateway at the cutover (owner's decision of 30.10.2026, §"Cutover").*
The official gateway has no password route since: Central signs in every
account from before it that has not moved yet, from its legacy store, and
moves it. At the cutover the operator copies every such account into that
store: its id here, its username, display name and tag, its Argon2id salt and
output, the version of this gateway's terms it accepted, a forced password
change and a ban in force, and its source snapshot. Nothing else: no address,
no session, no chat key (baylee-central `docs/native-login.md` §"Legacy
login"). The store lasts until the owner's deadline, the end of 30.06.2027 in
Europe/Berlin; then Central deletes what is left of it, and an account that
has not moved by then has no way to Central from its old password.

Only an explicit Central discovery response (`Legacy`) permits the legacy
sign-in, and the password goes to Central alone: `LegacyLogin` with the name
and the password, over the verified-TLS transport of every Central request
(in development, to the explicit loopback Central the client was given). It
is sent exactly once. Network failures, wrong passwords and refusals never
permit another destination, and no client sends a password to a gateway.
Central checks the password against the stored hash, a ban and a forced
change, and reserves the account's immutable Central identity under its name
and tag; the answer carries the enrollment ticket and the stored snapshot.
Enrollment and the OPAQUE sign-in that follows derive from the password on the
device, and it is dropped when the sign-in returns: it only ever unwraps the
master key.

The snapshot contains preferences, owned decks and deck version history. The
client encrypts and enrolls it, logs in through OPAQUE, downloads/decrypts it
and validates the schema before confirming restore. `LegacyLink`, with the
source digest the restored document holds, then closes the move at Central
and deletes the account's row from the legacy store; a digest that is not the
stored one fetches the stored snapshot again (`LegacyExport`) and restores it
first. Interrupted enrollment resumes at the next sign-in; already-committed
enrollment cannot be overwritten by another legacy sign-in, and a closed move
is never reopened. Once Central confirms the move closed, future sign-ins have
nothing to resume.

Export and link wait for Central's terms in force, as the gateway tickets
before them did. A sign-in that finds them unaccepted therefore succeeds with
the move held, and holds no password for it; the client shows those terms,
and their acceptance finishes the move before any gateway admission
(`docs/protocol.md` §"Terms of use (WG-1)"). A move that fails then ends the
Central sign-in, as at sign-in; the next one resumes it.

The gateway keeps the account: its id, game data, replays, statistics and
friends. Central names the legacy account to the official gateway's bridge,
with an admission, only once its move is closed; the first such admission
fixes the link (`central_account_link`), makes the account Central-only,
removes its password hash and ends its old sessions in one transaction, and
lobby and chat sockets are asked to reconnect. A link is immutable, and a
Central identity is linked to one account; the same admission again changes
nothing. A database constraint prevents any later code from restoring the
password, and migrations refuse a rollback once credentials have been retired.
This cannot erase old backups, which still need the operator's documented
retention policy. Original gameplay data and identifiers remain at the
gateway; this integration does not relocate replay blobs or uploaded images.
Native chat keys and complete retained private history are backed up in the
client-encrypted Central account archive and restored without the source
device. The sole active account session resumes one logical chat identity per
gateway. An existing local vault is imported with a key derived from the
master key, which a password sign-in attached to it without modifying it; the
password is dropped after that sign-in and never kept for the chat
(`password-vault.md` §"The account key").
Synchronization is asynchronous; delivery acknowledgements wait for a
confirmed backup, so a new installation can replay outstanding messages. See
[encrypted chat synchronization](central-chat-sync.md) for the manifest
protocol, recovery checks and remaining limits.

The first snapshot format is deliberately bounded: at most 192 KiB and 2,048
source rows. The cutover's export refuses an account over it, and with it the
whole export, so the operator decides about it; chunked large-object migration
is still required before admitting such accounts. Never work around this by
dropping history or truncating an export.

### Cutover

The move of the old accounts' sign-in to Central is one maintenance window,
the operator's, in this order, so that no password can change at the gateway
after its hash was copied:

1. Stop the official gateway and deploy the release without its password
   routes; let no player in yet.
2. Export and import in one pipe, with the gateway's `DATABASE_URL`:
   `baylee-gateway legacy export | central-service --legacy-import-stdin`
   (Central's operator document first, baylee-central `docs/operations.md`
   §"The legacy store"). The export opens the database without migrating it,
   reads every account that is no guest, has a password hash and no link, in
   one repeatable-read transaction, and writes the document to standard
   output only, never to a terminal. One account that cannot move (no
   username, a snapshot over the bound, a hash under other parameters)
   refuses the whole export and is named by its id.
3. Compare the two lines on standard error: the export's count (with how
   many must choose a new password and how many are banned) and digest, and
   the import's count and digest, recomputed from what Central stored. They
   are equal, or the import committed nothing.
4. Start Central with its legacy store, then the gateway, then publish the
   client. A client from before still posts to the gateway's
   `/auth/central/migrate`, which is gone: it must update, and the updater
   needs no sign-in.

The document is `baylee-legacy-accounts/1`: `{"format", "accounts": [{
"legacy_id", "username" (its key), "display_name" (only one that keeps the
rule, else `null`), "tag", "password": {"salt": 16 bytes, "hash": 32 bytes},
"terms_version", "must_change_password", "ban": null | {"until": Unix seconds
| null}, "snapshot": the source snapshot's bytes}], "count", "digest"}`, bytes
as JSON arrays of numbers. Only hashes written as
`$argon2id$v=19$m=19456,t=2,p=1$` with a 16-byte salt and a 32-byte output
are taken: the parameters this gateway hashes with, and the only ones Central
verifies. The digest is SHA-256 over the count as eight big-endian bytes, then
each account in ascending `legacy_id` byte order as `f(legacy_id)
f(username) o(display_name) o(tag in decimal) f(salt) f(hash)
o(terms_version) byte(must_change_password) o(ban: "forever" or its until in
decimal) f(snapshot)`, where `f(x)` is `x`'s length as four big-endian bytes
followed by `x`, and `o(x)` is one zero byte for none or one byte `1`
followed by `f(x)`. Central's import computes the same.

The `password_hash` column stays until a later release's migration drops it,
after the cutover.

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

## The official gateway's aliases

The official gateway moved from `https://baylee.acevik.de` to
`https://gateway.baylee.app` on 30.10.2026 (owner's decision). Every client
installed before then has the old address compiled in, so the old host serves
the same gateway as an alias until those clients are gone. Then the old host
only redirects, the aliases are removed, and the account document takes the
new spelling ([#402](https://github.com/AceVik/baylee/issues/402)).

A Central ticket and the lease redeemed from it carry the origin the client
asked for (its audience): `https://baylee.acevik.de` from an installed client,
`https://gateway.baylee.app` from one that pins the new address. The client
pins it only once Central takes it (the flip in #402); until then every
client asks for the old origin. So does a gateway's terms
acceptance (`TermsSubject::Gateway`). Central holds the official gateway's
origin together with its aliases, and it must take a ticket, a lease and a
terms acceptance under any of them as the official gateway's. That is
baylee-central's part (`OFFICIAL_ORIGIN` with its compiled `OFFICIAL_ALIASES`,
pinned since #51), and it ships before a client with the new address: a new
client asks for tickets for an origin an older Central does not know. The
gateway keeps no list of its own. It redeems a ticket with its bridge
credential, and Central checks the ticket's audience against the official
origins; live, its bridge file's `origin` may name either of them
(`Endpoint::official_origins`). Order: Central, then the gateway's half of
this move (its host and bridge file), then the client release.

The client has one mapping, `gateway_list::moved` (client-core): the old
address reads as the new one. Every caller asks it, or `same` and `keyed`,
which are built on it; nothing compares two addresses itself. Its callers do
one of two things with an old address:

- **Read it under the new address.** These are this device's own files, which
  one client version reads at a time: the saved list, the uses, the guest
  tokens and the accepted terms in `client-settings.json` (rewritten on load),
  `resume.json` and the crash report from the client an update replaced, the
  configured and the page's gateway, and the account's gateway list as the
  client shows it.
- **Keep it under the old address** (`gateway_list::keyed`, `KEYED`). The
  vault's scope is both its folder name and its AEAD associated data, so moving
  it would mean unlocking and resealing real keys. The account's chat records
  are matched with `==` by installed clients on the account's other devices: if
  they were rewritten, those clients would find no record and generate a second
  chat identity. The chat folder and the keychain slot follow the vault. The
  account document's gateway list keeps the old spelling too (`filed`), so an
  installed client shows its own pinned row and no second one, and the website
  may keep sealing `["https://baylee.acevik.de"]`.

Do not "repair" the old spelling in these. `KEYED` never changes, not even
after #402, and `legacy.gateway` keeps the old address, because installed
clients refuse a document whose `legacy.gateway` differs. The legacy login and
its migration move into Central, so `/auth/central/migrate` gets no alias.
Browser storage belongs to the page's origin: `/play/` on the new host starts
with an empty `localStorage`, and a browser guest kept on the old host stays
there. That was accepted rather than carrying a token across origins. Tests
use the old spelling: `gateway_list`, `settings`, `vault::password`,
`restore`, `resume`, `vault_access`, `lobby::central` and `crash` in the
client, and `central::tests` and the e2e `central/aliases.rs` (an alias
ticket admits, another origin's is refused) in the gateway (`docs/client.md`
§"The live gateway's address, and its move (30.10.2026)").

## Linking a gateway with a token

*30.10.2026.* A self-hosted gateway links to Baylee Central without an
operator file: its owner registers the gateway's origin on baylee.app
(`/account/gateways`) and makes a **gateway token** there; an administrator
of Central approves the gateway for the public list only (a token signs
players in while the gateway awaits that, and not once it is disabled) (baylee-central `docs/gateway-registry.md`
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
an explicit numeric-loopback HTTP origin. Invalid values fail closed. A legacy
sign-in there goes to that same development Central, when it has a legacy
store. Use synthetic accounts.
Without the explicit development override the compiled destination is live.
Browser authentication is still the existing separate flow; these SDK changes
target Windows, Linux and macOS.

The gateway bridge is opt-in through `BAYLEE_CENTRAL_CONFIG_FILE`. Its bounded
JSON has `endpoint` (`"live"` or a numeric-loopback development origin) and
`bridge` (a random 32-byte JSON array, shared with the Central operator). Store
it outside the repository, readable only by the service identity. Never put
the value in arguments, logs or client configuration. Unknown settings fail
startup. Without this file the gateway takes no Central sign-in; whether it
takes guests is its mode's (§"Who a gateway lets in").
The official gateway admits a moved account as the legacy account it moved
from, linking the two at its first admission (§"Legacy migration"), and an
account registered at Central (no legacy account) as a new local projection,
as a registered gateway does; Central's beta key stood in for the gateway's at
sign-up, so only `BAYLEE_REGISTRATION=off` refuses it (baylee-central
`docs/web-accounts.md`).
A moved account keeps its display name and tag at Central: the cutover's
export carries both, the name only when it keeps `auth::valid_display_name`
(a name from before that rule, or from `gateway-store.json`, stays behind,
and the account chooses one on the website), and the tag always; Central's
reservation keeps them (baylee-central `docs/native-login.md` §"Legacy
login"). At every Central sign-in here the gateway takes the name and tag the
account has at Central (`GatewayProfile`), as every gateway does, so a name
changed on the website arrives with the next one.
The official gateway is listed in Central's directory (`baylee.app/gateways`,
marked "Official") once Central's operator has listed it (baylee-central
`docs/operations.md` §"The official gateway"). Its bridge file then gains
`metadata` (`name`, `version`, `website`) beside `bridge`, for example
`"metadata": {"name": "Baylee", "version": "0", "website":
"https://gateway.baylee.app/"}`: the gateway sends `GatewayHeartbeat` with the
bridge credential every 30 seconds, Central updates the official entry and no
other, and the entry shows online. The website must lie on the bridge's
origin: its `origin` where the file names one, which live is one of the
official gateway's (`https://gateway.baylee.app`, or `https://baylee.acevik.de`
from before the move), and live without one on either of those, as Central
checks it. In development that is `origin`, which a bridge file with
`metadata` must then name. `version` is replaced with
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
Central against that isolated database: the cutover (the gateway's own
`legacy export` into Central's legacy store, and the end of the password
sessions), the legacy sign-in at Central, the link fixed at the first
admission, the move held for Central's terms, session renewal, game-seat
revocation, fresh-device restore, preference reconciliation,
registered-gateway admission and the official gateway's heartbeat in bridge
mode; it provisions Central's `central_legacy` role beside the others. Its optional
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
