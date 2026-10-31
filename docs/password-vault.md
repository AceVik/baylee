# Password-protected local chat vault

The native client no longer opens the OS credential store when social chat
starts. Registered accounts use `vault/<scope>/password.vault`; guests and
browser tabs keep chat keys and history only in memory. `BAYLEE_KEY_STORE=off`
is no longer a chat persistence mode. It cannot enable a plaintext fallback.

## Key hierarchy and limits

Version 1 is exactly 232 bytes: `BAYVAU01`, a 16-byte random salt, a 24-byte
nonce and 48-byte wrapped data key, then a 24-byte nonce and 112-byte encrypted
device-secrets payload. The random data key is 256 bits. The payload contains
the existing signing seed, key-agreement seed, and chat-history vault key,
96 bytes total. Existing `baylee-seal` history encryption and its HKDF-separated
subkeys remain unchanged, so importing an old device preserves its identity,
safety number, and access to existing encrypted history.

The password wrapping key is Argon2id v1.3 with 64 MiB, three passes, four lanes,
and 32 output bytes: RFC 9106's second recommended profile. Version 1 fixes
these parameters rather than accepting attacker-supplied costs. UTF-8 passwords
are limited to 1–1024 bytes and are neither trimmed nor normalized. Each wrapper
uses fresh salt and a fresh XChaCha20-Poly1305 nonce. Payload encryption uses a
separate nonce and domain-separated authenticated data. OS entropy failure
stops the operation.

The scope is SHA-256 over a versioned domain, validated account tag, delimiter,
and canonical full HTTP(S) gateway URL. It preserves path case, scheme, and
non-default port; URL userinfo, query, and fragment are rejected. The full scope
is both the filesystem namespace and AEAD associated data. Legacy OS entry
names remain `tag@host[:port]` only when explicitly importing those entries.

References checked for this implementation:

- [RFC 9106, section 7.4](https://www.rfc-editor.org/rfc/rfc9106.html#section-7.4).
- [RustCrypto Argon2](https://docs.rs/argon2/latest/argon2/), including its
  `zeroize` feature for allocated KDF working memory.
- [RustCrypto XChaCha20-Poly1305](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/).
- [Microsoft security descriptor format](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format)
  and [CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw).

## Session lifecycle and recovery

For a gateway's own accounts (not Baylee Central's, which use the account key
below), a successful account-auth response hands its password to the vault worker,
bound to the gateway and returned session token. Failed responses discard the
handoff. Unconsumed handoffs expire after two minutes and are discarded when
the account/gateway changes. The worker performs the KDF, key-file I/O, legacy
store access, and history restoration; the render thread polls its result.
Only one vault job runs per process, including an explicitly requested keychain
import that is still waiting for the operating system.

An existing vault opens with the login password. A resumed session without a
password remains chat-locked until the player opens **Unlock chat**. **Lock chat**
flushes encrypted state, disconnects chat, and drops the machine and unlocked
store. Sign-out, account switch, and process exit likewise drop the unlocked
keys. Key material and password buffers use zeroization; this is not a promise
that every temporary compiler copy, OS swap page, or transport buffer is wiped.

An accepted password change first tries the new password, then the old
password and rewraps the same data key. It works whether the gateway rotates or
retains the session token. Local manual rewrapping uses the original vault
password and two matching copies of the new password. History and device keys
are not regenerated. Backups made before a password change still require their
old password.

An administrator's password reset does **not** recover the local vault. Enter
the original vault password and optionally rewrap with the new account
password. Wrong passwords, damaged ciphertext, unreadable files, and unknown
versions leave the vault closed; they never become “no keys”. Chat can stay
locked while the player uses the rest of the application. There is no recovery
backdoor. If the original password is lost, preserve a backup of the whole
account vault folder before manually moving it aside to create a new device;
the old history cannot be promised recoverable.

## The account key

The owner, 30.10.2026: the password opens the account's master key once and
is dropped; it is never kept for anything else. For a Baylee Central account
the chat's device keys live in Central's encrypted archive
(`central-chat-sync.md`); a local password vault is only the source of a
one-time import. Before this change the client kept the password for 120
seconds after a Central sign-in to open that vault at the first gateway
admission; that hand-off is gone.

Instead a vault can carry a second way in: its data key wrapped under a key
derived from the account's master key (`central-crypto`'s local vault key,
HKDF-SHA-512 over the master key, the account context and the vault's 32-byte
scope), in `account-key.wrap` beside `password.vault`. Version 1 is exactly 80
bytes: `BAYAKW01`, a 24-byte nonce, and the XChaCha20-Poly1305 ciphertext and
tag of the data key, bound to the scope. The vault file itself is never
changed by it, so a gateway's own account (no master key) opens the vault
with its password exactly as before.

- **At a Central sign-in with the password**, in the sign-in's own thread
  and before the password is dropped, every local vault that password opens
  and that has no wrapper yet gains one (`attach_account_keys`). The scope
  comes from the vault's folder name, so no gateway or tag is needed. At most
  eight vaults are tried per sign-in (one Argon2id run each); a vault another
  process holds is skipped.
- **At a Central chat start** the import tries the account's key first,
  through the account worker (which keeps the master key); the password is
  used only where the player typed it into the existing unlock, and that
  import then writes the wrapper too.
- A wrapper of another account or scope, or of a vault made again since
  (another data key), fails authentication and releases nothing.

The vault file stays password-wrapped: removing that wrapper (or the file)
once its keys are confirmed in Central's archive would end the password's
last offline-guessable copy on the device, and is left to a later decision,
since it deletes or rewrites a player's file.

## Creation, import, and durable storage

Missing vaults require an explicit choice. **Create new encrypted vault** has a
second confirmation because it creates a new E2E device. Creation and imports
also require a matching repeat of the actual chosen wrapping password, using
the optional new-password field when supplied and otherwise the first field. **Import legacy key
file** and **Import from system keychain** read their source only after the
player selects them. The latter may show an OS access prompt, on its worker.
There is no automatic OS-store availability probe or keychain read.

Migration preserves legacy sources byte-for-byte and copies ciphertext into
the new scoped chat folder. Existing state must authenticate under the imported
key. An interrupted import can be resumed with the same device and password;
conflicting destination bytes are refused. The legacy `device.key`, OS entry,
and old chat directory are never deleted. In particular, an old plaintext key
file remains a security exposure until its owner explicitly removes it after
checking the migration and backups. Revoking a device archives its encrypted
vault under a UUIDv7 name; it cannot silently register that revoked device on
the next launch.

A held OS file lock excludes another process for the unlocked account's
lifetime. Unix directories/files are 0700/0600, owned by the current effective
user, with symlink and hardlink checks. The configured root is canonicalized
(to accept macOS `/tmp` and `/var` aliases); vault subpaths must not be symlinks.
Windows directories have a protected DACL granting only the current user and
LocalSystem, inherited by files; unexpected owners, ACLs and reparse points
are rejected. Writes use a new unique file, sync, atomic replacement, and
Unix directory sync or Windows write-through replacement. No plaintext key is
written to a temporary file. Inputs are bounded before KDF/file decoding.

This defends against copied encrypted files, accidental cross-account access,
and ordinary other-user access. It does not defend a compromised unlocked
process, an administrator controlling the machine, a malicious same-user
process racing filesystem operations, or rollback to an older valid backup.
Password strength still determines resistance to offline guessing.

## Stores outside this change

The account/gateway session and guest bearer-token persistence in client
settings is unchanged; those tokens are not E2E keys. `baylee-seat`'s LLM API
keys are also a separate system: environment values, its explicit server file
store, or its OS credential store. It has no account-login password in standalone
CLI operation. Desktop `seatpanel::poll` does not query it during normal startup;
opening Settings can request key metadata through the separate `baylee-seat`
process, and starting a configured LLM seat can read its key there. Those paths
remain OS-store consumers. Provider CLI authentication (for example Codex's own
login) belongs to that provider CLI and is not migrated by this feature.

The new UI uses existing fonts and UI tokens and adds no card art, symbols,
audio, or external assets. The Wizards Fan Content Policy FAQ, checked at
[the primary policy page](https://company.wizards.com/en/legal/fancontentpolicy),
says: “You can require a login/sign up to play a private group game.” This
local encryption requirement adds no fee or account-registration requirement.
Scryfall's primary documentation returned an access challenge during this
review; no API, image, attribution, or image treatment is changed. New direct
crypto/OS dependencies are existing lockfile crates, licensed MIT/Apache-2.0
(or the compatible rustix options); no new shipped asset licence is introduced.

## Integration verification (18 October 2026)

An isolated registered account on the local gateway was exercised through the
native client's dev-control harness, with the normal password store enabled:
explicit creation and confirmation, manual lock, rejected wrong password,
correct unlock, process restart and automatic unlock from a fresh login.
The encrypted record was 232 bytes; no plaintext `device.key` was created.
A wrong password left the file byte-identical. Manual password changes in both
directions changed the wrapper while preserving the encrypted device payload
byte-for-byte. Rendered unlock, failure and restored-chat screens were inspected.
The final native client run passed 2094 tests; the integrated core run passed
2150 tests. Workspace all-targets Clippy with dev-control, Wasm and formatting
checks passed. A 1280 × 720 run also verified scrolling, Escape dismissal, and
that an acknowledged chat notice stays closed after another unlock.
Client/core regression tests also cover idle-tree retention, focus, account
switch cancellation, migration, password reset and preferences changed during
an unlock job. Windows ACL code and tests were cross-compiled, but Windows
runtime verification remains unavailable on this Mac. Dependency license, ban
and source checks passed. The subsequent Windows/Linux integration review
replaced the unmaintained Linux `zbus_names`, `zvariant` and `zvariant_derive`
chain (2026-0370/0371/0372) with the shared
[native credential adapter](native-credential-store.md). Its isolated real
Secret Service fixture verifies legacy lookup, read/write/delete and refusal
of ambiguous entries; those advisories are no longer present in the graph.
