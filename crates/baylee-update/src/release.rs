//! What GitHub's releases API answers, and which of it is ours to install.
//!
//! `GET /repos/AceVik/baylee/releases` lists releases newest first, each
//! with its assets. The release workflow (`.github/workflows/release.yml`)
//! uploads, per desktop target, `baylee-client-<version>-<triple>.<ext>`
//! with a `.sha256` and a `.sig` beside it; this module finds the three for
//! one target. Only the fields read here are declared, so a field GitHub
//! adds or drops elsewhere in the answer changes nothing.

use crate::version;
use semver::Version;
use serde::Deserialize;

/// One release, as the API lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Release {
    /// `v0.1.0-beta.2`.
    pub tag_name: String,
    /// A draft is not published, and never offered.
    #[serde(default)]
    pub draft: bool,
    /// GitHub's own pre-release flag. Offered only to a pre-release build,
    /// like a pre-release version ([`version::is_offered`]).
    #[serde(default)]
    pub prerelease: bool,
    /// The release's page, with its notes: where a notice links to.
    pub html_url: String,
    /// Every file uploaded to it.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// One uploaded file.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Asset {
    /// The file name, which is how an archive is found for a target.
    pub name: String,
    /// Its size in bytes, as uploaded.
    pub size: u64,
    /// Where to download it. Redirects to GitHub's object storage.
    pub browser_download_url: String,
}

/// Reads the API's answer.
///
/// # Errors
///
/// The JSON error when the body is not a list of releases.
pub fn parse(body: &str) -> Result<Vec<Release>, serde_json::Error> {
    serde_json::from_str(body)
}

/// The archive the release workflow packs for `target`, by name.
///
/// `.zip` for Windows and macOS, `.tar.gz` for everything else, as
/// `scripts/package-client.sh` writes them.
#[must_use]
pub fn archive_name(version: &str, target: &str) -> String {
    let ext = if target.contains("-windows-") || target.contains("-apple-") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("baylee-client-{version}-{target}.{ext}")
}

/// An update this build is offered, and what it would install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The release's version.
    pub version: Version,
    /// Its tag, `v` included.
    pub tag: String,
    /// The release page, with the notes.
    pub page: String,
    /// The three files for this build's target, as far as the release has
    /// them.
    pub files: Option<Files>,
}

/// The archive for one target and the two files that vouch for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Files {
    /// The archive itself.
    pub archive: Asset,
    /// Its Ed25519 signature ([`crate::sign`]). Without one the archive is
    /// never installed.
    pub signature: Option<Asset>,
    /// Its SHA-256, `sha256sum`'s format.
    pub checksum: Option<Asset>,
}

/// The newest release a build at `current` for `target` is offered, if any.
///
/// Drafts are skipped, and so is a release whose tag is not a version. A
/// release counts as a pre-release when either its version has a
/// pre-release part or GitHub flags it as one. The newest is picked by
/// version, not by list order: the API sorts by creation date, and a patch
/// release for an older line can be created after a newer one.
#[must_use]
pub fn newest(releases: &[Release], current: &Version, target: &str) -> Option<Offer> {
    releases
        .iter()
        .filter(|release| !release.draft)
        .filter(|release| !(release.prerelease && current.pre.is_empty()))
        .filter_map(|release| Some((version::of_tag(&release.tag_name)?, release)))
        .filter(|(candidate, _)| version::is_offered(candidate, current))
        .max_by(|(a, _), (b, _)| a.cmp_precedence(b))
        .map(|(version, release)| {
            let files = files_for(release, &version, target);
            Offer {
                version,
                tag: release.tag_name.clone(),
                page: release.html_url.clone(),
                files,
            }
        })
}

fn files_for(release: &Release, version: &Version, target: &str) -> Option<Files> {
    let name = archive_name(&version.to_string(), target);
    let find = |wanted: &str| release.assets.iter().find(|a| a.name == wanted).cloned();
    Some(Files {
        archive: find(&name)?,
        signature: find(&format!("{name}.sig")),
        checksum: find(&format!("{name}.sha256")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The API's real answer for this repository on 27.09.2026 (two
    /// pre-releases, ten assets each, no `.sig` yet), trimmed to the fields
    /// a release and an asset carry besides their authors and notes.
    const RECORDED: &str = include_str!("../tests/fixtures/releases.json");

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn release(tag: &str, prerelease: bool, draft: bool, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            draft,
            prerelease,
            html_url: format!("https://example.test/{tag}"),
            assets: assets
                .iter()
                .map(|name| Asset {
                    name: (*name).into(),
                    size: 1,
                    browser_download_url: format!("https://example.test/{name}"),
                })
                .collect(),
        }
    }

    #[test]
    fn the_recorded_answer_parses() {
        let releases = parse(RECORDED).unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v0.1.0-beta.2");
        assert!(releases[0].prerelease);
        assert!(!releases[0].draft);
        assert_eq!(releases[0].assets.len(), 10);
        assert_eq!(
            releases[0].html_url,
            "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.2"
        );
    }

    /// A beta.1 build is offered beta.2, with the archive for its own
    /// target and the checksum — and no signature, because none was
    /// published before #326. That release is therefore link-only.
    #[test]
    fn a_beta_one_build_is_offered_beta_two_from_the_recorded_answer() {
        let releases = parse(RECORDED).unwrap();
        let offer = newest(&releases, &v("0.1.0-beta.1"), "x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(offer.version, v("0.1.0-beta.2"));
        assert_eq!(offer.tag, "v0.1.0-beta.2");
        let files = offer.files.unwrap();
        assert_eq!(
            files.archive.name,
            "baylee-client-0.1.0-beta.2-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(files.archive.size, 119_590_682);
        assert!(
            files
                .archive
                .browser_download_url
                .starts_with("https://github.com/AceVik/baylee/releases/download/v0.1.0-beta.2/")
        );
        assert_eq!(
            files.checksum.unwrap().name,
            "baylee-client-0.1.0-beta.2-x86_64-unknown-linux-gnu.tar.gz.sha256"
        );
        assert_eq!(files.signature, None);
    }

    #[test]
    fn the_newest_build_is_offered_nothing_from_the_recorded_answer() {
        let releases = parse(RECORDED).unwrap();
        assert_eq!(
            newest(&releases, &v("0.1.0-beta.2"), "aarch64-apple-darwin"),
            None
        );
    }

    #[test]
    fn every_published_target_finds_its_own_archive() {
        let releases = parse(RECORDED).unwrap();
        for (target, ext) in [
            ("x86_64-unknown-linux-gnu", "tar.gz"),
            ("aarch64-unknown-linux-gnu", "tar.gz"),
            ("x86_64-pc-windows-msvc", "zip"),
            ("aarch64-pc-windows-msvc", "zip"),
            ("aarch64-apple-darwin", "zip"),
        ] {
            let files = newest(&releases, &v("0.1.0-beta.1"), target)
                .unwrap()
                .files
                .unwrap_or_else(|| panic!("no archive for {target}"));
            assert_eq!(
                files.archive.name,
                format!("baylee-client-0.1.0-beta.2-{target}.{ext}")
            );
        }
    }

    /// A target no release was built for (Intel macOS) is still told about
    /// the release, with no files: a notice with a link, nothing to install.
    #[test]
    fn a_target_without_an_archive_gets_the_page_only() {
        let releases = parse(RECORDED).unwrap();
        let offer = newest(&releases, &v("0.1.0-beta.1"), "x86_64-apple-darwin").unwrap();
        assert_eq!(offer.files, None);
        assert!(offer.page.ends_with("/v0.1.0-beta.2"));
    }

    #[test]
    fn the_archive_name_follows_package_client_sh() {
        assert_eq!(
            archive_name("0.1.0-beta.3", "aarch64-apple-darwin"),
            "baylee-client-0.1.0-beta.3-aarch64-apple-darwin.zip"
        );
        assert_eq!(
            archive_name("0.1.0-beta.3", "x86_64-pc-windows-msvc"),
            "baylee-client-0.1.0-beta.3-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            archive_name("0.1.0-beta.3", "aarch64-unknown-linux-gnu"),
            "baylee-client-0.1.0-beta.3-aarch64-unknown-linux-gnu.tar.gz"
        );
    }

    #[test]
    fn the_signature_and_checksum_are_found_beside_the_archive() {
        let t = "x86_64-pc-windows-msvc";
        let name = archive_name("0.1.0-beta.3", t);
        let releases = [release(
            "v0.1.0-beta.3",
            true,
            false,
            &[
                &name,
                &format!("{name}.sig"),
                &format!("{name}.sha256"),
                // Another target's files are not mistaken for this one's.
                "baylee-client-0.1.0-beta.3-aarch64-pc-windows-msvc.zip.sig",
            ],
        )];
        let files = newest(&releases, &v("0.1.0-beta.2"), t)
            .unwrap()
            .files
            .unwrap();
        assert_eq!(files.signature.unwrap().name, format!("{name}.sig"));
        assert_eq!(files.checksum.unwrap().name, format!("{name}.sha256"));
    }

    #[test]
    fn drafts_are_never_offered() {
        let releases = [
            release("v0.1.0-beta.4", true, true, &[]),
            release("v0.1.0-beta.3", true, false, &[]),
        ];
        let offer = newest(&releases, &v("0.1.0-beta.2"), "t").unwrap();
        assert_eq!(offer.version, v("0.1.0-beta.3"));
    }

    /// The newest by version, whatever order the list is in: beta.10 after
    /// beta.9 even when the API lists beta.9 first.
    #[test]
    fn the_newest_is_picked_by_version_not_by_list_order() {
        let releases = [
            release("v0.1.0-beta.9", true, false, &[]),
            release("v0.1.0-beta.10", true, false, &[]),
            release("v0.1.0-beta.3", true, false, &[]),
        ];
        let offer = newest(&releases, &v("0.1.0-beta.2"), "t").unwrap();
        assert_eq!(offer.version, v("0.1.0-beta.10"));
    }

    /// A stable build skips a release GitHub flags as a pre-release even
    /// when its version has no pre-release part, and one whose version has
    /// one even when GitHub does not flag it.
    #[test]
    fn a_stable_build_skips_both_kinds_of_pre_release() {
        let releases = [
            release("v0.4.0", true, false, &[]),
            release("v0.3.0-rc.1", false, false, &[]),
            release("v0.2.1", false, false, &[]),
        ];
        let offer = newest(&releases, &v("0.2.0"), "t").unwrap();
        assert_eq!(offer.version, v("0.2.1"));
    }

    #[test]
    fn a_tag_that_is_no_version_is_skipped() {
        let releases = [
            release("nightly", true, false, &[]),
            release("v0.1.0-beta.3", true, false, &[]),
        ];
        let offer = newest(&releases, &v("0.1.0-beta.2"), "t").unwrap();
        assert_eq!(offer.tag, "v0.1.0-beta.3");
    }

    #[test]
    fn a_body_that_is_not_a_list_is_an_error() {
        assert!(parse(r#"{"message":"API rate limit exceeded"}"#).is_err());
        assert!(parse("").is_err());
    }
}
