# Account-bound E2E vault and recovery

Design decision, 19 October 2026. **Not implemented yet.** The existing
implementation is described in `password-vault.md`; this document specifies
its replacement, not security guarantees of the current release.

The owner requires automatic client-side encryption and unlocking, encrypted
account-bound gateway storage, restoration on another device, and password
changes that preserve access. The password must not be transmitted to the
gateway. Recovery through an unlocked device or a personal recovery code is
allowed; saving that code is **optional**, as chosen by the owner on this date.
There is no operator-held decryption key or password-reset backdoor.

## Current gaps

- `baylee-client/src/lobby/http.rs` currently sends password fields for
  registration, login and password change. The gateway's `routes/signin.rs`
  and `moderation.rs` consume those passwords. TLS protects transit, but does
  not hide a password from the receiving gateway.
- `password.vault` is local and contains device identity seeds as well as the
  local history key. Uploading it and using its identity on every device would
  clone a device; it is not an account synchronization protocol.
- Gateway chat delivery has limited retention and per-device acknowledgments.
  Downloading keys alone cannot restore history that only remains on an old
  device. Encrypted history synchronization is a separate required component.
- Existing password resets issue a temporary password. That is account access,
  not recovery of encrypted content.

## Authentication and key hierarchy

Use OPAQUE-3DH through a maintained implementation, not a home-grown challenge
protocol. RFC 9807 permits registration and mutual authentication without
revealing the password. TLS with certificate verification remains mandatory.
No automatic downgrade to the old password-in-JSON endpoints is allowed.

OPAQUE's client-only export key, after successful authentication, derives a
dedicated wrapping key via HKDF with version, gateway identity, immutable
account identity and purpose bound into the derivation/authenticated context.
The shared OPAQUE session key must never protect the vault: the server knows
that key. Use a fixed versioned Argon2id stretching profile, initially the
existing 64 MiB/three-pass/four-lane profile, benchmarked on supported devices.
Do not accept unbounded KDF costs supplied by a server or stored record.

Generate a random 256-bit account master key on the client. Authenticated
XChaCha20-Poly1305 wrapping protects it under the password-derived wrapping
key. Give each data object a random content key, wrapped under a purpose-
separated account key. Bind account, vault generation, object identity, type
and version to authenticated ciphertext. Use fresh cryptographic randomness,
bounded decoding and redacted/zeroizing secret types throughout.

Store only the authentication record, authenticated encrypted master-key
wrappers, encrypted manifests and encrypted objects on the gateway. Enforce
ownership from the authenticated account, never a submitted account id alone.
No plaintext master keys, recovery codes, passwords, history, or content-key
material may enter logs, telemetry, crash payloads or server storage. Account
identifiers, timing and object sizes remain visible metadata unless separately
hidden. Encryption does not promise protection of an already compromised client
or prevent guessing weak passwords after sufficient server secrets are stolen.

Keep device signing/agreement identities separate. A new device creates its
own identity and restores shared content from the account vault. Account-
authorized device certificates/key verification must prevent the gateway from
silently substituting recipient keys. Revocation excludes future access; it
cannot erase plaintext or keys already copied to a revoked device.

## Normal lifecycle

Registration creates credentials and the first encrypted vault automatically.
Login downloads and opens the existing vault without a second password dialog.
An authoritative missing-vault response can initialize a genuinely new vault
using conditional creation. Timeouts, unsupported versions, decryption failure,
and database failure are never interpreted as an empty vault.

On a new device, restore the manifest and encrypted objects before presenting
sync as complete. Use immutable object identifiers and authenticated revisions
with explicit merge rules, deduplication and tombstones. Concurrent devices
must not overwrite each other's history or preferences with whole-file uploads.
Bound storage, object sizes and upload/download concurrency. Offline edits
remain encrypted locally until acknowledged; errors are retried without
replacing the user's existing keys. Respect retention and deletion preferences.

A remembered HTTP session alone cannot unlock E2E data. After a restart,
either obtain the normal login password or use an explicitly trusted device's
OS-protected local key. Never persist the password or put a decryption key
beside a bearer token in settings. Device trust must not fall back to plaintext
when protected storage is unavailable. The owner must not face routine
"create vault", "unlock chat" or manual password-rewrapping dialogs after a
successful password login.

## Password change

1. Authenticate with the old credentials and open the current master wrapper.
2. Register the new OPAQUE credentials and wrap the **same** master key under
   the new client-only export key, with fresh salt/nonce material as required
   by the protocol and wrapping format.
3. Commit the new authentication record and master wrapper in one database
   transaction, conditional on the current account/vault revision. Bind the
   operation to a fresh authentication proof; a bearer session alone does not
   authorize replacement of credentials or recovery configuration.
4. Support idempotent retries and lost responses. Revoke old sessions and
   invalidate pending credential-change attempts consistently with the commit.
   Other clients must reload the new revision, never upload an old wrapper.

Payloads need not all be re-encrypted for an ordinary password change. Copies
of older wrappers remain decryptable with their old password; a password
change cannot revoke an attacker's already downloaded backup. Suspected key
compromise requires a separate master-key epoch rotation and future-device
authorization policy, rather than claiming rewrapping solves that problem.

## Forgotten password: optional recovery

Provide an unobtrusive option to save a cryptographically random recovery code
(256 bits, human-readable encoding with a typo checksum). Show/export it only
on the client. It protects a second master-key wrapper using a separately
derived recovery wrapping key; the gateway never receives the code or that
key. Do not substitute security questions, email contents, or an account-reset
token for a recovery secret. Saving the code is optional and does not block
ordinary use.

Account reset and data recovery are independent:

| Available proof | Account access | Old encrypted data |
| --- | --- | --- |
| Original password | Normal authentication | Opens automatically |
| Authorized unlocked device | Reset/change flow | Device can rewrap the retained master key |
| Personal recovery code plus account-reset authorization | New credentials | Client recovers the master key and rewraps it |
| Account-reset authorization only | New credentials | Remains encrypted; no silent replacement |

An unlocked device must explicitly authorize a reset recovery or transfer;
a reset initiated by someone controlling email must not automatically cause
that device to release a key to the initiator. Bind a transfer to a fresh
end-to-end authenticated pairing and locally confirmed recipient, or perform
the recovery on the trusted device itself. Mere possession of a session token
is insufficient.

After recovery, replace the password wrapper, invalidate old sessions and
offers, and offer a fresh recovery code. Rotation cannot invalidate older
copies of a recovery wrapper already stolen. If all recovery routes are lost,
keep the old vault intact and state that it is inaccessible. Starting a new
empty vault is an explicit, separately confirmed operation; never make a
password reset silently destroy the old ciphertext.

## Migration and release gate

The owner confirmed that real accounts already exist. Preserve them and
their data; this is not a test-account reset. Initiate enrollment from the
next interactive login where possible, with resumable migration. The exact
legacy authentication bridge remains a prerequisite to implementation:
existing Argon2 verification accepts the original password, and merely adding
client-side hashing would break it. Never silently send that password through
the legacy endpoint in order to claim a transparent OPAQUE upgrade. A session
token alone must not become permission to replace account credentials. A
verified migration authorization or an appropriately reviewed legacy proof is
needed, including for accounts without a current session or recovery address.

Preserve legacy vaults and history until the new encrypted backup has been
uploaded, authenticated and restored successfully. Re-encrypt historical
content into account objects; do not reuse the old device's identity on other
devices. Record migration progress so a crash resumes safely. Corruption or a
scope mismatch must leave the source untouched.

Authentication migration also needs a versioned transition. Existing Argon2
verifiers cannot simply be converted into OPAQUE records without the user's
participation. Never claim that changing the protocol retroactively hides a
password the old gateway already received. A new password chosen entirely
through the new protocol provides that clean boundary. Define how existing
accounts authorize enrollment without sending their password through a
fallback endpoint, including logged-out accounts, before enabling the feature.

An untrusted gateway must not be able to request a plaintext-password fallback,
a different credential destination, weaker algorithm parameters, or executable
login code from the official native client. A static client-side password hash
sent as the login credential is replayable as that credential, even if the
gateway hashes it again. It must never double as the vault wrapping key.
OPAQUE is the selected replacement, with gateway/account identity binding and
locally enforced protocol rules. This protects a trusted client talking to a
modified gateway; it cannot protect a user who runs a modified client or a web
client whose JavaScript is supplied by that hostile gateway. Client and update
distribution therefore belong to a separate trusted origin/signing boundary.

Before release, test real client/gateway/database round trips and at least:
wrong passwords; cross-account/gateway substitution; modified ciphertext;
truncated and oversized inputs; weak/invalid group elements; simultaneous
registration and writes; replayed auth/change/reset challenges; interrupted
password-change transactions; stale client uploads; multiple-device restore;
recovery with and without a code/device; revoked-device access; missing
objects; retained old backups; and interrupted legacy migration. Verify that
serialized requests, diagnostics and server records contain no client secret.
Use the library's RFC test vectors and obtain a security review of the complete
application protocol. An upstream library audit is not an audit of this
integration. Agent-created test processes must always be cleaned up.

## Primary references checked

- [RFC 9807: OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html), especially
  registration, export-key use, identity binding and security considerations.
- [RFC 9106: Argon2](https://www.rfc-editor.org/rfc/rfc9106.html#section-7.4).
- [opaque-ke 4.0.1 API](https://docs.rs/opaque-ke/4.0.1/opaque_ke/) and
  [upstream audit history](https://github.com/facebook/opaque-ke#audit).
  The downloaded stable release identifies RFC 9807 as its specification;
  the 2021 audit covers an earlier release, not a new application integration.
