//! The desktop client's updater (#326), without the client.
//!
//! Everything here is plain Rust with no renderer, so every decision the
//! updater makes is tested on whichever machine runs the tests, for every
//! system it serves:
//!
//! - [`version`]: which release is newer (semver precedence, pre-releases
//!   only for a pre-release build).
//! - [`release`]: GitHub's releases API, and which asset is this target's.
//! - [`sign`]: the Ed25519 signature over each archive, and the keys the
//!   client trusts.
//! - [`archive`]: unpacking a `.zip` or `.tar.gz` without leaving the
//!   staging directory, symlinks and executable bits kept.
//! - [`plan`]: which renames replace an installation, per system, as a pure
//!   function over names.
//! - [`apply`]: doing those renames under a journal, so a crash half way
//!   leaves a client that starts (the old one or the new one), and the next
//!   start finishes or rolls back.
//! - [`check`]: the HTTP half: asking GitHub, downloading, verifying,
//!   staging.
//! - [`service`]: the thread that checks at start and every six hours, and
//!   never with automatic checks off.
//!
//! The client (`baylee-client/src/update/`) runs [`check`] on a thread of
//! its own and [`apply`] after its window has closed. `docs/client.md`
//! §"Updating" is the design; `docs/releasing.md` §"Signing" the other end.

/// The version type every function here takes.
pub use semver::Version;

/// The public key type the signature checks take.
pub use ed25519_dalek::VerifyingKey;

pub mod apply;
pub mod archive;
pub mod check;
pub mod plan;
pub mod release;
pub mod service;
pub mod sign;
pub mod version;
