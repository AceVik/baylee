# Central personal chat, archive and device restoration

The native client keeps chat keys and history in the client-encrypted Central account.
OPAQUE authenticates the account; only the serial account worker owns the SDK session
and master key. A gateway receives an origin-bound admission capability and its own
session token. `GatewayHttp` has no password operation. The only legacy password
transport is Central's discovered legacy sign-in over verified TLS (or an explicit
numeric-loopback development fixture); no gateway receives that password. Account deletion is refused in the native Central dispatcher
until a Central step-up and deletion flow exists.

Central permits one active account session. A successful login revokes previous
sessions and gateway leases; failed logins do not. The account worker checks session
status at least every ten seconds between jobs. A confirmed rejection causes the
native account UI to sign out and drop chat keys. Transport failure never enables a
legacy login or substitutes empty account data. A disconnected client learns of
revocation when it can communicate again; no protocol can erase an offline device's
already decrypted memory remotely.

Personal contacts, direct messages and groups now use the account-owned Central
social API. Central supplies their device directory; the gateway still supplies
keys and membership for game/table chat. These are separate trust namespaces,
including pending directory requests, safety numbers, confirmations and persisted
pins. A gateway update cannot approve or replace a Central peer's keys.

Both directories retain the existing TOFU limitation. The chat key cache checks
each device's self-signature and binds its ID to that key, but a malicious directory
can insert its own self-signed device on a peer's first lookup. Compare safety numbers through another
trusted channel before relying on that first contact. Changes to a known IK/DK set
now block sends until explicit verification of the exact displayed key snapshot;
refetch/reconnect cannot silently clear that requirement. Native Central also pins
every own-account recipient key to the authenticated encrypted archive, so a gateway
cannot add an attacker device to the sender's own recipients. Invalid or duplicated
directory entries fail closed. A separately authenticated account-bound peer-key
directory would still be required to eliminate first-contact TOFU. Central archive
ciphertext remains protected by account keys independently of this limitation.
Previously accepted sender keys stay pinned while a changed directory is awaiting
confirmation. Old pinned inbox messages remain readable; messages from unconfirmed
new identities are held without display or ACK. The held inbox is bounded to 256
messages and 4 MiB. Backpressure pauses reception; exact confirmation permits retry
and redial rather than acknowledging a gap.

Central authorizes personal group membership; the gateway supplies table membership. Verifying individual
peer keys does not prove that each listed member is authorized: a malicious membership service
could introduce another tag and its first-contact keys. Strong protection against
that attack requires authenticated membership/transcripts or explicit member
approval. This milestone does not introduce a group PKI or claim that guarantee.
Legacy clients can review their own changed device keys through the safety-number
button beside their devices; Central's archive whitelist cannot be expanded that way.

## Key ownership and local migration

`AccountData.chat` is a bounded archive of up to 512 device records. Each record binds
the full canonical gateway origin, public account tag, a stable archive-slot UUIDv7,
the signing-key-derived device ID, and exactly 96 secret bytes. Secret buffers wipe
on drop and have no Debug/Display implementation. They are serialized only as part
of the account document, which the SDK encrypts before HTTP transmission. Plaintext
device keys are never written by the new native adapter.

The sole active account session derives one logical personal chat identity from its
master key, issuer, immutable account ID and vault epoch. It is shared by the browser
and native implementation. Gateway-scoped encrypted archives remain separate.
This depends on Central's single-session revocation and the gateway's live lease
validation: it is not a protocol for two concurrent chat writers. The current message
encryption has no shared ratchet. Known installation UUIDv7 values are separately
retained in the encrypted archive (up to 1,024); restoring an installation does not
create a new signing identity. Older identities remain readable archives.

The reusable `ContextGuard` uses the same private directory validation and process
lock as the previous password vault. The local `central-installation.id` is public
metadata. A corrupt ID is refused, never silently replaced. Existing password-vault
keys are opened locally with the account's vault key, which a password sign-in gave
the vault (`password-vault.md` §"The account key"), and imported unchanged; the
original encrypted file is retained. Existing legacy history and supported
legacy key files/keychain entries are imported without overwriting unequal files.
If an old wrapper needs a different password, the existing unlock/import UI remains
available. Failure to read/decrypt existing data is not permission to generate keys.
When a newly authenticated session replaces an older local client, startup waits up
to 15 seconds for that client's real OS guard to be released, with cancellation.
It never breaks the lock or applies state while another process owns the directory.

Startup first restores and validates the Central document. It imports existing local
keys before selecting the account-derived identity. During migration the original
archive record remains intact; authenticated state is re-encrypted for the new wire
identity while retaining its old at-rest key, peer pins, verification barriers and
history. Own-device cache/registration state is rebuilt. Conflicting or unauthenticated
state fails closed. An interrupted initial publication can reconstruct from the
unchanged former archive. Later installations select the same derived identity. The complete restored state and histories are
authenticated and merged in memory before local writes. The complete initial snapshot
must be acknowledged by Central before registration or dialing is permitted.
Only the first import, before any Central history root exists, may initialize the
complete protocol state from a strictly authenticated matching local identity. This
retains its verified peer keys. Once a Central root exists, local files contribute
history only; local state can never replace the authoritative remote trust state.
Conflicting keys, retired-only archives, corrupt data and unavailable Central storage
fail closed. Gateway-only device removal and "forget this device" are disabled for
Central identities; account sign-out disconnects the installation. Global identity
rotation requires a distinct account-level operation. Archive keys are retained.

## History objects and publication

The archive manifest is inside the encrypted account document. A manifest names only
`state.bin` and `history/<16 lowercase hex>.log`. Files contain the existing
authenticated chat ciphertext. They are split into immutable objects of at most
256 KiB, encrypted again by the Central SDK with account/object/revision binding.
Object IDs come from PostgreSQL UUIDv7. A manifest records exact revisions, sizes,
chunk order and SHA-256 digests; the digest is also private inside the encrypted
manifest. Unchanged chunks are reused after exact length/digest comparison.

The client bounds one snapshot to 1,025 files, 16 MiB per file and 256 MiB total.
Central enforces its independent account quota. Every new chunk is acknowledged
before a compare-and-swap publishes a new manifest generation. Only after that
acknowledgement are superseded, unreferenced chunks deleted. Lost replies and failed
manifest writes never cause deletion of potentially referenced objects. An interrupted
upload can leave unreferenced encrypted objects; server quota/garbage collection
remains necessary. Explicit HTTP 429 responses receive at most three delayed retries;
ambiguous transport failures are not blindly retried.

Local disk changes schedule a snapshot on the account worker, with a five-second
debounce and no network calls while holding the local disk lock. Account preferences,
gateway edits, key records and history manifests all refresh and preserve unrelated
fields before a revision-checked write. A failed upload keeps the previous complete
manifest and encrypted local files. On ordinary shutdown/logout the disk adapter
attempts a bounded 30-second drain, including macOS World-drop shutdown. Forced
termination, loss of connectivity or a failed drain cannot promise upload completion;
the retained local encrypted files are retried on the next successful login.

The chat panel and encryption sheet show pending, failed or confirmed backup state.
The core emits history/state persistence before delivery ACK effects. Native Central
chat retains each ACK until the corresponding local-write generation is covered by
an acknowledged manifest CAS. Failed local writes block publication and ACKs until
strict restart/recovery. A failed upload retains the gateway's encrypted inbox.
Official sends include an encrypted self-wrap, so a lost `Sent` reply can also be
replayed to the same logical identity on a new installation. Old clients without a
self-wrap remain gateway-compatible but cannot gain this additional sender guarantee.
Gateway retention/availability still bounds pending-message recovery; the protocol
cannot recover data deleted by an unavailable or malicious gateway before backup.

## Restore semantics

Device B downloads the archive from Central; device A need not be online. It verifies
every object's revision, size and digest, then authenticates the chat state/history
with the corresponding archived device key. For the shared active identity it restores
the complete validated Central protocol state, including registration, key cache and
replay-suppression state. Transport ACK counters restart for safe inbox replay.
`Social::restore_archive` merges older identities and newer local files only as history:
their registration, key cache and seen state cannot roll back the authoritative
Central state. It emits no network receipts.

All records in one archive and affected local logs are checked before that import
mutates the social state. Equal message IDs with different content are conflicts.
Complete local logs, including messages outside the UI's 500-line window, are merged
and re-encrypted under the active logical identity's vault key. Only local state/history
effects are emitted; importing an archive sends no receipts or network messages.
Each import's disk effects are applied before the next archive, and startup state
effects are applied before imports so an old startup snapshot cannot overwrite the
restored conversation map. Current history-retention settings remain authoritative.

## Personal transport and migration boundary

Personal operations use the native Central bearer session inside the serial account
worker. The render thread neither performs HTTP nor owns the account master key.
One bounded flight and at most 16 queued operations serve contacts, groups, device
registration, key lookup and encrypted delivery. Inbox polling coalesces at two
seconds, membership refreshes every five seconds, and failures back off to thirty
seconds. Definitive session rejection drops unlocked keys and prevents automatic
legacy fallback. Late inbox pages cannot advance a restarted or locked session's
cursor. Uncertain sends retain the exact ciphertext and request ID across retries;
retries leave room for inbox and directory reads.
Local queue saturation keeps a send pending for retry. A terminal refusal releases
only that request's ciphertext-cache slot; uncertain or rate-limited requests keep
their exact ciphertext. This prevents rejected sends from exhausting the bounded
retry cache and blocking unrelated future messages.

Central inbox reads are non-destructive, with a separate cursor and at most 50
messages per page. Their cursor never becomes a gateway ACK. Core checks still
verify signatures, pinned devices, original conversation and receiver wrap before
archiving. Already delivered group history may be read after leaving; it creates
no membership or send permission. Native history and age bounds still apply.
The browser/native crypto interoperability test uses both actual implementations.

The gateway still owns game chat, public chat, presence, table invitations and
moderation. It cannot overwrite Central contacts/groups or acknowledge a personal
send. Gateway registration retries independently, and outstanding game sends are
bounded. Old gateway group deliveries addressed to the current identity can be
archived read-only using gateway pins, with encrypted publication before ACK.

Existing archived histories and identity records are preserved. This change does
not silently turn old gateway friendships or memberships into Central relationships:
participants request/accept contacts and group invitations in Central. Unread old
queues addressed only to an older gateway device are not migrated by this adapter.
On 2026-11-02 the owner authorized discarding the existing live messages and contact
links at cutover instead of importing them. Their preservation is no longer a
cutover prerequisite. Existing account keys and encrypted archive documents remain
outside that live-data reset. No production deletion was executed by the local PWA
work; startup does not automatically purge legacy records.

Central currently retains at most 5000 sent messages per account and 5000 deliveries
per device, without automatic deletion; full quotas reject new sends. An owner-approved
retention policy and operational capacity planning remain release prerequisites.
Account deletion, recovery UI, decks and replays are outside this transport change.

## Verification

- Account unit tests: nested unknown preferences, key bounds, duplicate/conflicting
  device records, manifest path/object bounds, hash changes and bounded Busy retries.
- Context tests: exclusive process ownership, stable installation ID, corrupt-ID
  rejection, wrong-password failure, unchanged original vault ciphertext and bounded,
  cancellable handoff of an existing OS guard.
- Core archive tests: complete cross-device history beyond the UI window, deduplication,
  idempotence, authentication failures and no imported protocol state.
- `e2e_central/chat.rs`: real Central HTTP/PostgreSQL objects between two OPAQUE logins,
  revoked first session, 600 encrypted message records, chunk reuse, malformed digest,
  stale manifest refusal, retained known installations and offline-source restoration
  of the same logical identity and protocol state.
- Social regressions: separate persisted personal/game trust and confirmations,
  simultaneous directory fetches, retained cursor isolation, former-group history,
  wrong send receipts, account identity migration and real browser/native ciphertext.
- Bridge regressions: queue bounds, late/locked inbox responses, disconnected worker
  recovery, session revocation and exclusion of gateway personal metadata.
- Native regression: Central deletion cannot dispatch the password to a gateway;
  the bounded snapshot collector refuses incomplete or unexpected local namespaces;
  ACKs wait for the complete corresponding snapshot and survive failed socket sends.

Run the ordinary account/client/core tests plus `cargo dev central integration`.
For coordinated sibling development, use its `--cargo-config target/central-patch.toml`
option; use `cargo clippy --config ...` with the config after the subcommand.

On 2026-11-02, the backend Central revision `24bf90dae9ae0175c6bcd4701bdebd87dd3e19d1` passed the real PostgreSQL 18 Central/Gateway integration, including the personal transport scenario. The focused core social tests (334), native social tests (153), account tests (9), workspace formatting and Clippy for those crates and the integration target passed. This verification used the committed Git dependency, without a local path override. Publish that Central revision before publishing the dependent native revision; these changes do not deploy either service.

The coordinated deployment now pins merged Central main
`2b54a3944150a1aecbc20b94854966d787d31cd5` (Central PR #60).
The hook selects Central and its website from the SDK pin, so it must include the
completed PWA and its generated WASM, not just the earlier backend milestone.
Central published this revision before the dependent native revision. It also
includes the reviewed migration-purpose checks and browser push/logout fixes;
the crypto source and dependency graph remain unchanged from the preceding
PWA pin `f66568b985656eb1677c33c6f6515e107b126c93`.

The preceding PWA-inclusive pin passed `cargo dev central integration` against real
PostgreSQL 18.6 (including personal chat and legacy cutover), all five advisory-guard
regressions, and `cargo dev gate shipped-features`. Its lockfile change contains only
the eight Central Git source revisions; no registry package or feature changed.
