# Native credential storage

`baylee-keyring` is the shared adapter used by the legacy chat-key importer and
the language-model seat. Normal account chat storage uses Central's encrypted
archive; this adapter does not change that design or introduce another copy of
the account password.

Windows keeps `windows-native-keyring-store` 1.1.0 and macOS keeps
`apple-native-keyring-store` 1.0.2 through `keyring-core` 1.0.1, with the same
provider defaults and credential names as before. The adapter builds entries
from its own provider instance rather than replacing a process-global store.

Linux uses the safe Rust API of GNOME `libsecret` 0.9.0 with GLib 0.22.10. The
former keyring 4.2 default Linux provider pulled in the retired standalone
zvariant/zbus_names graph (RUSTSEC-2026-0370, -0371 and -0372). Those dependencies
are removed, not suppressed in `deny.toml`.

| Platform | Direct provider dependencies | Library boundary |
| --- | --- | --- |
| Windows | keyring-core 1.0.1, windows-native-keyring-store 1.1.0 | Native Credential Manager, unchanged defaults |
| macOS | keyring-core 1.0.1, apple-native-keyring-store 1.0.2 (`keychain`) | Native Keychain, unchanged defaults |
| Linux | libsecret 0.9.0, GLib 0.22.10; existing futures-util and zeroize | libsecret-sys 0.9.0 and gio/GLib 0.22 safe bindings over system libraries |

Each Linux operation opens its own Secret Service connection and requires the
protocol's encrypted `dh-ietf1024-sha256-aes128-cbc-pkcs7` session before any
secret transfer. The platform library supplies cryptography and randomness;
there is no application cryptography or plaintext-session fallback here. This
is the desktop Secret Service protocol, not the network account protocol or a
substitute for TLS. Local OS/session-bus protections remain part of its trust
boundary.

Compatibility and failure behavior:

- Lookups use the existing exact `service` and `username` attributes across
  collections. No additional schema-name, application-ID or target restriction
  hides older entries. New entries use the provider's default collection.
- One match is read, updated or removed. Multiple matches fail closed and are
  never arbitrarily selected or bulk deleted. A presence check reads metadata
  without requesting the secret.
- Every Linux operation has one 30-second deadline, including unlocking prompts.
  Timeout cancels the local request; a write's remote outcome can still be
  indeterminate, so callers must not interpret timeout as proof of no mutation.
- Errors are fixed redacted messages. Provider payloads and invalid secret bytes
  are never included in diagnostics. Temporary decoded Linux secret bytes use
  `Zeroizing`; the caller must protect the returned string.
- Missing/unavailable providers do not trigger an automatic file fallback.

The Linux build needs `libsecret-1-dev`; Debian/Ubuntu installations need
`libsecret-1-0` and a desktop Secret Service provider for credential operations.
The client packages declare the library dependency. GNOME libsecret is a
dynamically linked system library under LGPL-2.1-or-later; its Rust bindings are
MIT licensed. The system package supplies the native library's notices/source
offer; it is not statically copied into Baylee.

`scripts/test-keyring-linux.sh` creates a separate D-Bus session, temporary HOME
and data/runtime directories, and an owned GNOME Keyring process. It checks
legacy attribute discovery, Unicode/update/delete, no-secret presence lookup,
and refusal to read/overwrite/delete ambiguous matches. It never accesses the
user's session bus or keyring. The normal unit suite performs no OS-store I/O;
the real fixture test is ignored unless explicitly selected by this script.
CI runs the fixture on Linux in test partition 1 and always cleans up its daemon
and temporary files.

Reviewed primary sources:

- [libsecret Rust API](https://docs.rs/libsecret/0.9.0/libsecret/)
- [GNOME libsecret](https://gitlab.gnome.org/GNOME/libsecret)
- [Native DH randomness](https://github.com/GNOME/libsecret/blob/master/egg/egg-dh-libgcrypt.c)
- [Secret Service encryption protocol](https://specifications.freedesktop.org/secret-service/latest/ch07s03.html)
- [zvariant maintenance advisory](https://rustsec.org/advisories/RUSTSEC-2026-0370.html)
