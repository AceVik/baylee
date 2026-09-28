//! Asking GitHub for a newer release, and staging it.
//!
//! One request per check to the releases API, unauthenticated: 60 an hour
//! per address is GitHub's limit, and a client checks at start and every six
//! hours. The answer's `ETag` is sent back as `If-None-Match`, so an
//! unchanged list costs a `304` that GitHub does not count against it. When
//! GitHub says the limit is spent (`403`/`429` with `x-ratelimit-remaining:
//! 0`, or a `retry-after`), no request is made before the time it names.
//!
//! Staging an offer ([`Checker::run`]) downloads the archive for this build's
//! target into `<base>/.baylee-update/`, hashing it on the way; checks its
//! `.sha256`; verifies its `.sig` against the trusted keys; and only then
//! unpacks it, strips its versioned top folder into `new/`, and records it
//! as staged. An archive whose signature does not verify is deleted and
//! never unpacked.

use crate::apply::{self, Install, Staged, Unplaceable};
use crate::archive::{self, Format};
use crate::plan::NEW;
use crate::release::{self, Offer, Release};
use crate::sign::{self, VerifyingKey};
use semver::Version;
use sha2::{Digest as _, Sha256};
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The releases of this repository, newest first, ten at a time.
pub const GITHUB_RELEASES: &str = "https://api.github.com/repos/AceVik/baylee/releases?per_page=10";

/// The largest `.sig` or `.sha256` read: both are one short line.
const SMALL_FILE: u64 = 4096;

/// What one check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No release is newer than this build.
    UpToDate,
    /// A newer release is unpacked, verified, and installs when the client
    /// closes.
    Staged {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
    },
    /// A newer release exists and will not install itself; the notice
    /// links to its page.
    Available {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
        /// Why it is a link and not an install.
        why: Manual,
    },
    /// The check itself failed (offline, rate limited). Said nowhere but in
    /// the log: the next check tries again.
    Unknown(String),
}

/// Why an update is only linked to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Manual {
    /// The player switched automatic updates off.
    Off,
    /// A development build: it checks, and never replaces itself.
    DevBuild,
    /// The release has no archive for this target.
    NoArchive,
    /// The release's archive has no signature.
    Unsigned,
    /// The signature or the checksum did not verify.
    NotOurs(String),
    /// The installation cannot be replaced by this user.
    NotWritable(String),
    /// There is no installation to replace (a bare binary on macOS, or an
    /// app macOS runs from a read-only copy).
    Unplaceable(Unplaceable),
    /// Installing this version failed before; it is not tried again.
    FailedBefore(String),
    /// The download or the unpacking failed.
    Download(String),
}

/// What the client tells a check about itself.
#[derive(Clone, Debug)]
pub struct Context {
    /// This build's version.
    pub current: Version,
    /// This build's target triple.
    pub target: String,
    /// Where it is installed, or why it cannot be replaced.
    pub install: Result<Install, Unplaceable>,
    /// The keys an archive may be signed with.
    pub keys: Vec<VerifyingKey>,
    /// Whether it may install an update at all: a release build of a clean
    /// commit, with automatic updates on. [`Manual::DevBuild`] or
    /// [`Manual::Off`] when not.
    pub installs: Result<(), Manual>,
}

/// Why the releases could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    /// GitHub's limit is spent until this many seconds from now.
    RateLimited(u64),
    /// Any other answer than `200` or `304`.
    Status(u16),
    /// No answer at all.
    Network(String),
    /// An answer that is not a list of releases.
    Body(String),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited(secs) => write!(f, "rate limited for {secs} s"),
            Self::Status(code) => write!(f, "HTTP {code}"),
            Self::Network(err) | Self::Body(err) => f.write_str(err),
        }
    }
}

/// Asks for releases and stages updates. Keeps the last answer and its
/// `ETag` between checks.
pub struct Checker {
    agent: ureq::Agent,
    api: String,
    user_agent: String,
    etag: Option<String>,
    cached: Vec<Release>,
    not_before: Option<SystemTime>,
    requests: u64,
}

impl Checker {
    /// A checker asking `api` (normally [`GITHUB_RELEASES`]) as
    /// `Baylee/<version>`.
    #[must_use]
    pub fn new(api: impl Into<String>, version: &str) -> Self {
        use ureq::tls::{RootCerts, TlsConfig};
        let agent = ureq::Agent::config_builder()
            // Always verified, in a debug build too: nothing about GitHub
            // needs a development exception, and the signature is checked
            // anyway.
            .tls_config(TlsConfig::builder().root_certs(RootCerts::WebPki).build())
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            // A download of 150 MB over a slow line: no global timeout, but
            // one on silence.
            .timeout_recv_body(Some(Duration::from_secs(60)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .build()
            .new_agent();
        Self {
            agent,
            api: api.into(),
            user_agent: format!("Baylee/{version}"),
            etag: None,
            cached: Vec::new(),
            not_before: None,
            requests: 0,
        }
    }

    /// How many requests this checker has made, every one counted: what
    /// "no request at all" is measured by.
    #[must_use]
    pub fn requests(&self) -> u64 {
        self.requests
    }

    fn get(&mut self, url: &str) -> Result<ureq::http::Response<ureq::Body>, CheckError> {
        self.requests += 1;
        self.agent
            .get(url)
            .header("User-Agent", &self.user_agent)
            .call()
            .map_err(|err| CheckError::Network(err.to_string()))
    }

    /// The releases, fresh or, when GitHub says nothing changed, the last
    /// ones it sent.
    ///
    /// # Errors
    ///
    /// [`CheckError`].
    pub fn releases(&mut self) -> Result<Vec<Release>, CheckError> {
        let now = SystemTime::now();
        if let Some(until) = self.not_before
            && now < until
        {
            return Err(CheckError::RateLimited(
                until.duration_since(now).unwrap_or_default().as_secs(),
            ));
        }
        self.requests += 1;
        let mut request = self
            .agent
            .get(&self.api)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(etag) = &self.etag {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request
            .call()
            .map_err(|err| CheckError::Network(err.to_string()))?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        match response.status().as_u16() {
            200 => {
                let etag = header("etag");
                let body = response
                    .body_mut()
                    .with_config()
                    .limit(4 * 1024 * 1024)
                    .read_to_string()
                    .map_err(|err| CheckError::Body(err.to_string()))?;
                let releases =
                    release::parse(&body).map_err(|err| CheckError::Body(err.to_string()))?;
                self.etag = etag;
                self.cached.clone_from(&releases);
                Ok(releases)
            }
            304 => Ok(self.cached.clone()),
            code @ (403 | 429) => {
                let wait = limit_wait(
                    header("retry-after").as_deref(),
                    header("x-ratelimit-remaining").as_deref(),
                    header("x-ratelimit-reset").as_deref(),
                    now,
                );
                match wait {
                    Some(wait) => {
                        self.not_before = Some(now + wait);
                        Err(CheckError::RateLimited(wait.as_secs()))
                    }
                    None => Err(CheckError::Status(code)),
                }
            }
            code => Err(CheckError::Status(code)),
        }
    }

    /// One check: the newest offer for this build, staged if it may be.
    pub fn run(&mut self, context: &Context) -> Outcome {
        let releases = match self.releases() {
            Ok(releases) => releases,
            Err(err) => return Outcome::Unknown(err.to_string()),
        };
        let Some(offer) = release::newest(&releases, &context.current, &context.target) else {
            return Outcome::UpToDate;
        };
        let available = |why| Outcome::Available {
            version: offer.version.to_string(),
            page: offer.page.clone(),
            why,
        };
        if let Err(why) = &context.installs {
            return available(why.clone());
        }
        let install = match &context.install {
            Ok(install) => install,
            Err(why) => return available(Manual::Unplaceable(why.clone())),
        };
        match self.stage(install, &offer, &context.keys) {
            Ok(staged) => Outcome::Staged {
                version: staged.version,
                page: staged.page,
            },
            Err(why) => available(why),
        }
    }

    /// Downloads, verifies and unpacks `offer` into the staging directory.
    ///
    /// # Errors
    ///
    /// Why it was not staged; nothing unverified is left behind.
    pub fn stage(
        &mut self,
        install: &Install,
        offer: &Offer,
        keys: &[VerifyingKey],
    ) -> Result<Staged, Manual> {
        let files = offer.files.as_ref().ok_or(Manual::NoArchive)?;
        let signature = files.signature.as_ref().ok_or(Manual::Unsigned)?;
        let version = offer.version.to_string();
        // One stager per directory; the service is the only caller, so a
        // claim already held means a check is still running.
        let _claim = apply::Claim::take(&install.stage()).map_err(|err| {
            if err.kind() == io::ErrorKind::WouldBlock {
                Manual::Download("another process owns the update directory".into())
            } else {
                Manual::NotWritable(err.to_string())
            }
        })?;
        if install.stage().join("journal.json").exists()
            || install.stage().join("activation.json").exists()
        {
            return Err(Manual::Download(
                "installation recovery must finish first".into(),
            ));
        }
        if let Some(failed) = apply::failed(install)
            && failed.version == version
        {
            return Err(Manual::FailedBefore(failed.reason));
        }
        if let Some(staged) = apply::staged(install)
            && staged.tag == offer.tag
            && staged.asset == files.archive.name
            && staged.size == files.archive.size
        {
            return Ok(staged);
        }
        install
            .writable()
            .map_err(|err| Manual::NotWritable(err.to_string()))?;
        let format = Format::of_name(&files.archive.name).ok_or(Manual::NoArchive)?;
        let stage = install.stage();
        clear_staging(&stage);
        let download = |err: &dyn std::fmt::Display| Manual::Download(err.to_string());

        let archive_path = stage.join(&files.archive.name);
        let digest = self
            .download(
                &files.archive.browser_download_url,
                &archive_path,
                files.archive.size,
            )
            .map_err(|err| download(&err))?;
        let refuse = |why: String| {
            let _ = fs::remove_file(&archive_path);
            Manual::NotOurs(why)
        };
        if let Some(checksum) = &files.checksum {
            let text = self
                .small(&checksum.browser_download_url)
                .map_err(|err| download(&err))?;
            let text = String::from_utf8_lossy(&text);
            sign::checksum_matches(&text, &files.archive.name, &digest)
                .map_err(|err| refuse(err.to_string()))?;
        }
        let sig = self
            .small(&signature.browser_download_url)
            .map_err(|err| download(&err))?;
        let bytes = fs::read(&archive_path).map_err(|err| download(&err))?;
        sign::verify(&bytes, &sig, keys).map_err(|err| refuse(err.to_string()))?;
        drop(bytes);

        let unpacked = stage.join("unpack");
        let expected_root = files
            .archive
            .name
            .strip_suffix(".tar.gz")
            .or_else(|| files.archive.name.strip_suffix(".zip"))
            .ok_or(Manual::NoArchive)?;
        let result = unpack_into_new(
            &archive_path,
            format,
            &unpacked,
            &stage.join(NEW),
            install,
            expected_root,
        );
        let _ = fs::remove_file(&archive_path);
        let _ = fs::remove_dir_all(&unpacked);
        if let Err(err) = result {
            let _ = fs::remove_dir_all(stage.join(NEW));
            return Err(download(&err));
        }
        let staged = Staged {
            version,
            tag: offer.tag.clone(),
            page: offer.page.clone(),
            asset: files.archive.name.clone(),
            size: files.archive.size,
        };
        apply::mark_staged(install, &staged).map_err(|err| download(&err))?;
        Ok(staged)
    }

    /// Streams `url` to `to` through a `.part` file, hashing on the way.
    fn download(&mut self, url: &str, to: &Path, size: u64) -> io::Result<[u8; 32]> {
        let mut response = self
            .get(url)
            .map_err(|err| io::Error::other(err.to_string()))?;
        if response.status().as_u16() != 200 {
            return Err(io::Error::other(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        let part = to.with_extension("part");
        let mut file = fs::File::create(&part)?;
        let mut hash = Sha256::new();
        let mut reader = response.body_mut().as_reader().take(size + 1);
        let mut buf = vec![0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n])?;
            total += n as u64;
        }
        file.sync_all()?;
        drop(file);
        if total != size {
            let _ = fs::remove_file(&part);
            return Err(io::Error::other(format!(
                "{total} bytes where the release lists {size}"
            )));
        }
        fs::rename(&part, to)?;
        Ok(hash.finalize().into())
    }

    fn small(&mut self, url: &str) -> io::Result<Vec<u8>> {
        let mut response = self
            .get(url)
            .map_err(|err| io::Error::other(err.to_string()))?;
        if response.status().as_u16() != 200 {
            return Err(io::Error::other(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        response
            .body_mut()
            .with_config()
            .limit(SMALL_FILE)
            .read_to_vec()
            .map_err(io::Error::other)
    }
}

/// Removes a previous download or unpack, keeping the records `apply`
/// reads (a journal above all).
fn clear_staging(stage: &Path) {
    for name in [NEW, "unpack"] {
        let _ = fs::remove_dir_all(stage.join(name));
    }
    if let Ok(entries) = fs::read_dir(stage) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if Format::of_name(&name).is_some()
                || Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("part"))
                || name == "staged.json"
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

/// Unpacks into `unpacked`, checks it holds one versioned folder with this
/// system's program in it, and moves that folder's contents to `new`.
fn unpack_into_new(
    archive: &Path,
    format: Format,
    unpacked: &Path,
    new: &Path,
    install: &Install,
    expected_root: &str,
) -> io::Result<()> {
    let _ = fs::remove_dir_all(unpacked);
    fs::create_dir_all(unpacked)?;
    archive::unpack(archive, format, unpacked)?;
    let mut tops = fs::read_dir(unpacked)?
        .filter_map(Result::ok)
        .filter(|e| !e.file_name().to_string_lossy().starts_with("__MACOSX"))
        .collect::<Vec<_>>();
    let top = match (tops.pop(), tops.is_empty()) {
        (Some(top), true) if top.path().is_dir() => top.path(),
        _ => return Err(io::Error::other("the archive is not one folder")),
    };
    if top.file_name().and_then(|name| name.to_str()) != Some(expected_root) {
        return Err(io::Error::other(
            "signed archive version/target does not match the offer",
        ));
    }
    if !top.join(install.os.program()).exists() {
        return Err(io::Error::other(format!(
            "the archive has no {}",
            install.os.program()
        )));
    }
    fs::rename(&top, new)
}

/// How long GitHub asks to wait, from a `403`/`429`'s headers. `None` when
/// the answer is not about the rate limit.
fn limit_wait(
    retry_after: Option<&str>,
    remaining: Option<&str>,
    reset: Option<&str>,
    now: SystemTime,
) -> Option<Duration> {
    if let Some(secs) = retry_after.and_then(|v| v.trim().parse::<u64>().ok()) {
        return Some(Duration::from_secs(secs));
    }
    if remaining.map(str::trim) == Some("0") {
        let now = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let reset = reset
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(now + 3600);
        return Some(Duration::from_secs(reset.saturating_sub(now).max(60)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spent_limit_waits_until_its_reset() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert_eq!(
            limit_wait(None, Some("0"), Some("1000600"), now),
            Some(Duration::from_secs(600))
        );
        // A reset already past still waits a minute rather than asking at
        // once and being refused again.
        assert_eq!(
            limit_wait(None, Some("0"), Some("999000"), now),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            limit_wait(Some("120"), Some("5"), None, now),
            Some(Duration::from_secs(120))
        );
        // A 403 that is not about the limit is not waited out.
        assert_eq!(limit_wait(None, Some("12"), Some("1000600"), now), None);
        assert_eq!(limit_wait(None, None, None, now), None);
    }
}
