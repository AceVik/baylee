# Central chat archive and device restoration

The native client keeps chat keys and history in the client-encrypted Central account.
OPAQUE authenticates the account; only the serial account worker owns the SDK session
and master key. A gateway receives an origin-bound admission capability and its own
session token. `GatewayHttp` has no password operation. The only legacy password
transport is the fixed official migration gateway (or an explicit numeric-loopback
development fixture). Account deletion is refused in the native Central dispatcher
until a Central step-up and deletion flow exists.

Central permits one active account session. A successful login revokes previous
sessions and gateway leases; failed logins do not. The account worker checks session
status at least every ten seconds between jobs. A confirmed rejection causes the
native account UI to sign out and drop chat keys. Transport failure never enables a
legacy login or substitutes empty account data. A disconnected client learns of
revocation when it can communicate again; no protocol can erase an offline device's
already decrypted memory remotely.

This archive protection does not upgrade the existing peer-directory trust model.
The chat key cache checks each device's self-signature and binds its ID to that key,
but the gateway supplies the initial list. A malicious gateway can insert its own
self-signed device on a peer's first lookup. Compare safety numbers through another
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

Group and table membership is also supplied by the gateway. Verifying individual
peer keys does not prove that each listed member is authorized: a malicious gateway
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

The sole active account session resumes one logical chat identity per gateway/tag.
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

Startup first restores and validates the Central document. It preserves existing local
keys; an empty account may create the first logical identity. All later installations
select the same nonretired identity. The complete restored state and histories are
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

This does not move gateway moderation, friendship/group membership or server delivery
queues into Central. Those remain gateway-owned. It also does not make account
deletion, password reset/recovery UI, decks or replays part of this chat change.

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
- Native regression: Central deletion cannot dispatch the password to a gateway;
  the bounded snapshot collector refuses incomplete or unexpected local namespaces;
  ACKs wait for the complete corresponding snapshot and survive failed socket sends.

Run the ordinary account/client/core tests plus `cargo dev central integration`.
For coordinated sibling development, use its `--cargo-config target/central-patch.toml`
option; use `cargo clippy --config ...` with the config after the subcommand.
