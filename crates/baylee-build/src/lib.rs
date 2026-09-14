//! What this binary is, and where its source lives.
//!
//! Every crate in the workspace can ask the same six questions and get the
//! same answers, because they are stamped in at compile time by one build
//! script rather than derived at run time by each caller. That matters for
//! more than a splash line: the AGPL's §13 obliges a program that users
//! interact with over a network to offer them its Corresponding Source, and
//! an offer is only good if it names a *version*. "The source is on GitHub"
//! is not an offer; "this is 0.1.0+build.42, commit 3f9a1c7e21, and here is
//! where that commit lives" is.
//!
//! # Why a crate and not a `build.rs` per binary
//!
//! Three binaries would otherwise carry three copies of the same script and
//! drift, and the client would carry a fourth that had to work on
//! `wasm32-unknown-unknown`. There is nothing target-specific here — a build
//! script runs on the host and leaves behind string constants — so one crate
//! serves the native binaries and the browser build alike, and it has no
//! dependencies at all so that adding it to the client costs the wasm bundle
//! nothing but the bytes of the strings themselves.

#![forbid(unsafe_code)]

/// The workspace version, from Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the Corresponding Source lives. See the module doc.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The full commit this was built from, or `unknown`.
pub const COMMIT: &str = env!("BAYLEE_COMMIT");

/// The same commit, abbreviated to ten characters.
///
/// Ten rather than the seven git shows by default: seven collides in a
/// repository of this size sooner than people expect, and the number is read
/// by a person comparing two builds, which is exactly the case a collision
/// makes confusing.
pub const COMMIT_SHORT: &str = env!("BAYLEE_COMMIT_SHORT");

/// The branch it was built from, or `HEAD` when detached, or `unknown`.
pub const BRANCH: &str = env!("BAYLEE_BRANCH");

/// The build number. See [`build.rs`'s own note][crate] on why it is not a
/// commit count: the history is squashed from time to time and a number that
/// walks backwards is worse than none, so this is CI's run number where
/// there is one.
pub const BUILD_NUMBER: &str = env!("BAYLEE_BUILD_NUMBER");

/// When it was built, ISO-8601 UTC, or `unknown`.
pub const BUILT_AT: &str = env!("BAYLEE_BUILT_AT");

/// The target triple it was built for.
pub const TARGET: &str = env!("BAYLEE_TARGET");

/// `debug` or `release`.
pub const PROFILE: &str = env!("BAYLEE_PROFILE");

/// Whether the working tree had uncommitted changes.
///
/// A build made from an edited tree is not the commit it names, and a
/// version string that hid that would be the one lie in this module. Tracked
/// files only — an untracked scratch file does not change what was compiled.
pub const DIRTY: bool = matches!(env!("BAYLEE_DIRTY").as_bytes(), b"1");

/// The one-line version, the way a person reads it.
///
/// `0.1.0+build.42 (3f9a1c7e21)`, with `-dirty` appended when it is. Const
/// because a splash line should not allocate, and because a caller that
/// wants the pieces separately already has them above.
#[must_use]
pub const fn short() -> &'static str {
    // `const_format!` would be a dependency; the pieces are concatenated by
    // `concat!` instead, which is a macro rather than a crate.
    if DIRTY {
        concat!(
            env!("CARGO_PKG_VERSION"),
            "+build.",
            env!("BAYLEE_BUILD_NUMBER"),
            " (",
            env!("BAYLEE_COMMIT_SHORT"),
            "-dirty)"
        )
    } else {
        concat!(
            env!("CARGO_PKG_VERSION"),
            "+build.",
            env!("BAYLEE_BUILD_NUMBER"),
            " (",
            env!("BAYLEE_COMMIT_SHORT"),
            ")"
        )
    }
}

/// Everything, as the lines an "about" panel or a `--version` prints.
#[must_use]
pub fn lines() -> Vec<(&'static str, &'static str)> {
    vec![
        ("version", short()),
        ("commit", COMMIT),
        ("branch", BRANCH),
        ("built", BUILT_AT),
        ("target", TARGET),
        ("profile", PROFILE),
        ("source", REPOSITORY),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The version is the one field that is true even with no git, no
    /// network and no CI, because it comes from Cargo rather than from a
    /// process that might not be there.
    #[test]
    fn the_version_is_always_answered() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'), "{VERSION} is not a version");
    }

    /// Every probe answers *something*. The point of the build script's
    /// fallbacks is that a build from a tarball still produces a binary that
    /// can say what it is, and a test that only ever ran in a git checkout
    /// would never see the case those fallbacks exist for — so this asserts
    /// the shape rather than the value.
    #[test]
    fn nothing_is_ever_an_empty_string() {
        for (name, value) in lines() {
            assert!(!value.is_empty(), "{name} came back empty");
        }
    }

    /// The source offer is a URL, because §13 is satisfied by a place a
    /// person can go and not by a word.
    #[test]
    fn the_source_offer_is_reachable_prose() {
        assert!(
            REPOSITORY.starts_with("https://"),
            "the AGPL source offer is {REPOSITORY}, which is not somewhere a user can go"
        );
    }

    /// `short()` names the version and the commit in one line, which is what
    /// makes it an identification rather than a label — two builds of the
    /// same version from different commits must not read alike.
    #[test]
    fn the_short_line_names_both_halves() {
        let line = short();
        assert!(
            line.starts_with(VERSION),
            "{line} does not open with {VERSION}"
        );
        assert!(
            line.contains(COMMIT_SHORT),
            "{line} does not name the commit it was built from"
        );
        assert_eq!(line.contains("-dirty"), DIRTY);
    }
}
