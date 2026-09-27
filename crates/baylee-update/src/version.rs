//! Which release is newer than this build.
//!
//! Semantic versioning's precedence (semver.org §11), pre-releases included:
//! `0.1.0-beta.10` is newer than `0.1.0-beta.9` because a numeric identifier
//! compares as a number, and `0.1.0` is newer than every `0.1.0-…`. Build
//! metadata (`+build.42`) takes no part in it. The ordering itself is the
//! `semver` crate's; what this module adds is the policy: which releases a
//! build is offered at all.

use semver::Version;
use std::cmp::Ordering;

/// A release tag as a version: `v0.1.0-beta.3` → `0.1.0-beta.3`.
///
/// `None` for a tag that is not a version, which is then never offered: a
/// tag nobody can order is not "newer".
#[must_use]
pub fn of_tag(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

/// Whether a build at `current` is offered `candidate`.
///
/// Only a strictly newer version by precedence. A pre-release only to a
/// build that is itself a pre-release: a player on a stable build asked for
/// stable builds, and a beta is not one. A build on a pre-release is offered
/// the next pre-release and the stable release alike.
#[must_use]
pub fn is_offered(candidate: &Version, current: &Version) -> bool {
    if !candidate.pre.is_empty() && current.pre.is_empty() {
        return false;
    }
    candidate.cmp_precedence(current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn a_tag_loses_its_v() {
        assert_eq!(of_tag("v0.1.0-beta.3"), Some(v("0.1.0-beta.3")));
        assert_eq!(of_tag("0.2.0"), Some(v("0.2.0")));
        assert_eq!(of_tag("nightly"), None);
        assert_eq!(of_tag("v0.1"), None);
    }

    /// The case the owner named: ten after nine, not before it, which a
    /// string comparison gets wrong.
    #[test]
    fn beta_ten_is_newer_than_beta_nine() {
        assert!(is_offered(&v("0.1.0-beta.10"), &v("0.1.0-beta.9")));
        assert!(!is_offered(&v("0.1.0-beta.9"), &v("0.1.0-beta.10")));
    }

    #[test]
    fn precedence_follows_semver_eleven() {
        // The chain semver.org §11 gives, each newer than the one before.
        let chain = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in chain.windows(2) {
            let (older, newer) = (v(pair[0]), v(pair[1]));
            assert!(is_offered(&newer, &older), "{newer} after {older}");
            assert!(!is_offered(&older, &newer), "{older} not after {newer}");
        }
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        assert!(!is_offered(&v("0.1.0-beta.2"), &v("0.1.0-beta.2")));
        assert!(!is_offered(&v("0.2.0"), &v("0.2.0")));
    }

    /// Build metadata is not part of precedence: a rebuild of the same
    /// version is not offered as newer.
    #[test]
    fn build_metadata_does_not_make_a_release_newer() {
        assert!(!is_offered(&v("0.2.0+build.9"), &v("0.2.0+build.1")));
    }

    #[test]
    fn a_stable_build_is_never_offered_a_pre_release() {
        assert!(!is_offered(&v("0.3.0-beta.1"), &v("0.2.0")));
        assert!(is_offered(&v("0.3.0"), &v("0.2.0")));
    }

    #[test]
    fn a_pre_release_build_is_offered_both_kinds() {
        assert!(is_offered(&v("0.1.0-beta.3"), &v("0.1.0-beta.2")));
        assert!(is_offered(&v("0.1.0"), &v("0.1.0-beta.2")));
        assert!(is_offered(&v("0.2.0-alpha.1"), &v("0.1.0-beta.2")));
    }

    #[test]
    fn an_older_stable_is_not_offered_to_a_newer_pre_release() {
        assert!(!is_offered(&v("0.1.0"), &v("0.2.0-beta.1")));
    }
}
