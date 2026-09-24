//! Card art over HTTP(S), with a disk cache that can cost a download but never
//! a picture (#250).
//!
//! Bevy's own web reader (`bevy_asset` 0.19.1, `io/web.rs`) kept its cache under
//! `.web-asset-cache` in the working directory, and failed a load whose cache
//! write failed after a download that had succeeded. A macOS app started from
//! Finder or the Dock runs in `/`, so every card drew its text face and the log
//! said only `No such file or directory`. A native client registers this reader
//! as its `http` and `https` asset sources instead:
//!
//! - the cache is the per-user cache directory ([`cache_home`]), never the
//!   working directory, and a cache that cannot be read or written costs one
//!   warning per cause and a download, never a load;
//! - a cached picture is written to a `.part` beside its name and renamed into
//!   place ([`store`]), so a crash leaves a stray `.part` and never half a JPEG
//!   that fails to decode on every launch after;
//! - it asks only where card art comes from ([`allowed`]): the Scryfall CDNs
//!   and the gateway mirror the client was told to use. It follows no
//!   redirect, since a redirect could lead anywhere else.
//!
//! A browser keeps bevy's fetch reader (`crates/baylee-client/Cargo.toml`),
//! because the browser caches.

use baylee_client_core::images::{self, SCRYFALL_BACKS_CDN, SCRYFALL_CDN};
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetReaderFuture, AssetSourceBuilder, PathStream, Reader,
    VecReader,
};
use bevy::prelude::*;
use bevy::tasks::ConditionalSendFuture;
use std::fmt::Write as _;
use std::future::ready;
use std::io::{self, Write as _};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

/// The largest picture fetched. Scryfall's largest, `png`, is under 2 MB.
const LARGEST: u64 = 16 * 1024 * 1024;

/// The base a request may go to besides the Scryfall CDNs, asked for on every
/// request because signing in changes it.
type Base = Arc<dyn Fn() -> Arc<str> + Send + Sync>;

/// Registers [`ArtReader`] as the `http` and `https` asset sources.
///
/// It has to be added before `AssetPlugin`, which builds its sources from
/// whatever has been registered by then.
pub struct ArtReaderPlugin {
    /// Where pictures are cached. `None` caches nothing.
    pub cache: Option<PathBuf>,
    /// The mirror in force ([`images::art_base`] in the app).
    pub base: Base,
}

impl Default for ArtReaderPlugin {
    fn default() -> Self {
        Self {
            cache: cache_home().map(|home| home.join("art")),
            base: Arc::new(images::art_base),
        }
    }
}

impl Plugin for ArtReaderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(dir) = &self.cache {
            info!("card art cached in {}", dir.display());
        } else {
            warn!("no cache directory for card art: every picture is downloaded each time");
        }
        for scheme in ["http", "https"] {
            let reader = ArtReader {
                scheme,
                cache: self.cache.clone(),
                base: self.base.clone(),
            };
            app.register_asset_source(
                scheme,
                AssetSourceBuilder::new(move || Box::new(reader.clone())),
            );
        }
    }
}

/// Card art from the web, through the disk cache.
#[derive(Clone)]
pub struct ArtReader {
    /// `http` or `https`. The asset path arrives without it.
    pub scheme: &'static str,
    /// Where pictures are cached. `None` caches nothing.
    pub cache: Option<PathBuf>,
    /// The mirror in force.
    pub base: Base,
}

impl AssetReader for ArtReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<VecReader, AssetReaderError> {
        let Some(url) = path
            .to_str()
            .map(|rest| format!("{}://{}", self.scheme, rest.replace(MAIN_SEPARATOR, "/")))
        else {
            return Err(AssetReaderError::NotFound(path.to_path_buf()));
        };
        allowed(&url, &(self.base)())
            .map_err(|refusal| AssetReaderError::Io(Arc::new(io::Error::other(refusal))))?;
        let cache = self.cache.clone();
        let asked = path.to_path_buf();
        // Off the asset executor: the download and the cache are both
        // blocking, and a table asks for dozens of pictures at once.
        let bytes = blocking::unblock(move || fetch(&url, cache.as_deref(), asked)).await?;
        Ok(VecReader::new(bytes))
    }

    fn read_meta<'a>(&'a self, path: &'a Path) -> impl AssetReaderFuture<Value: Reader + 'a> {
        // `AssetMetaCheck::Never` (`standalone.rs`) never asks, and a host
        // that serves art is not asked for a sibling file it does not have.
        ready(Err::<VecReader, _>(AssetReaderError::NotFound(
            path.to_path_buf(),
        )))
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<PathStream>, AssetReaderError>> {
        ready(Err(AssetReaderError::NotFound(path.to_path_buf())))
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<bool, AssetReaderError>> {
        ready(Ok(false))
    }
}

/// Whether `url` is somewhere card art comes from: under a Scryfall CDN, or
/// under `base`, the gateway mirror the client was told to use.
///
/// # Errors
///
/// A refusal naming the host, for anywhere else and for a path that climbs
/// out with `..`.
pub fn allowed(url: &str, base: &str) -> Result<(), String> {
    let under = |root: &str| {
        url.strip_prefix(root.trim_end_matches('/'))
            .is_some_and(|rest| rest.starts_with('/'))
    };
    let climbs = url.split(['/', '?']).any(|segment| segment == "..");
    if !climbs
        && [SCRYFALL_CDN, SCRYFALL_BACKS_CDN, base]
            .into_iter()
            .any(under)
    {
        return Ok(());
    }
    let host = url
        .split_once("://")
        .map_or(url, |(_, rest)| rest.split('/').next().unwrap_or(rest));
    Err(format!(
        "card art is fetched only from the Scryfall CDN and the gateway's art mirror, \
         not from {host} ({url})"
    ))
}

/// The per-user cache directory, `…/baylee`.
///
/// `$XDG_CACHE_HOME` when it is set and absolute (the XDG spec ignores a
/// relative one), on every desktop, as `settings` reads `$XDG_CONFIG_HOME`;
/// else `~/Library/Caches` on macOS and iOS (whose `HOME` is the app's
/// container), `%LOCALAPPDATA%` on Windows, `~/.cache` elsewhere, and on
/// Android the app's own cache directory.
#[must_use]
pub fn cache_home() -> Option<PathBuf> {
    let set = |key: &str| {
        std::env::var_os(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    if let Some(xdg) = set("XDG_CACHE_HOME").filter(|dir| dir.is_absolute()) {
        return Some(xdg.join("baylee"));
    }
    #[cfg(target_vendor = "apple")]
    let base = set("HOME").map(|home| home.join("Library").join("Caches"));
    #[cfg(windows)]
    let base = set("LOCALAPPDATA");
    // `getCacheDir()` is `cache` beside `getFilesDir()`, which is the one
    // path a NativeActivity is handed.
    #[cfg(target_os = "android")]
    let base = bevy::android::ANDROID_APP
        .get()
        .and_then(|app| app.internal_data_path())
        .and_then(|files| Some(files.parent()?.join("cache")));
    #[cfg(not(any(target_vendor = "apple", windows, target_os = "android")))]
    let base = set("HOME").map(|home| home.join(".cache"));
    base.map(|dir| dir.join("baylee"))
}

/// The one client every picture is fetched with.
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    use ureq::tls::{RootCerts, TlsConfig};
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent(crate::cardtext::scryfall::AGENT)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .build()
        .new_agent()
});

/// The picture at `url`: from the cache when it is there, else downloaded and
/// then cached. Blocking.
fn fetch(url: &str, cache: Option<&Path>, asked: PathBuf) -> Result<Vec<u8>, AssetReaderError> {
    let file = cache.map(|dir| dir.join(key(url)));
    if let Some(file) = &file {
        match std::fs::read(file) {
            Ok(bytes) if !bytes.is_empty() => return Ok(bytes),
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => warn_once("read", file, &err),
        }
    }
    let bytes = download(url, asked)?;
    if let Some(file) = &file
        && let Err(err) = store(file, &bytes)
    {
        warn_once("write", file, &err);
    }
    Ok(bytes)
}

/// `url`'s body, or the error bevy's own reader gave for the same answer.
///
/// Only a success is a picture.
fn download(url: &str, asked: PathBuf) -> Result<Vec<u8>, AssetReaderError> {
    let failed = |err: ureq::Error| {
        AssetReaderError::Io(Arc::new(io::Error::other(format!("{url}: {err}"))))
    };
    match AGENT.get(url).call() {
        // A redirect among them: with none followed, ureq hands it back as
        // an answer, and its body is not the picture.
        Ok(response) if !response.status().is_success() => {
            Err(AssetReaderError::HttpError(response.status().as_u16()))
        }
        Ok(mut response) => response
            .body_mut()
            .with_config()
            .limit(LARGEST)
            .read_to_vec()
            .map_err(failed),
        Err(ureq::Error::StatusCode(404)) => Err(AssetReaderError::NotFound(asked)),
        Err(ureq::Error::StatusCode(code)) => Err(AssetReaderError::HttpError(code)),
        Err(err) => Err(failed(err)),
    }
}

/// The cache file name for `url`: its SHA-256, so any URL makes a safe name
/// and no two share one.
fn key(url: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut name = String::with_capacity(64);
    for byte in Sha256::digest(url.as_bytes()) {
        let _ = write!(name, "{byte:02x}");
    }
    name
}

/// Writes `bytes` to `file` so that no reader ever sees part of them: into a
/// `.part` beside it, synced, then renamed over it.
///
/// # Errors
///
/// Whatever creating the directory, writing or renaming says. A failure
/// leaves `file` as it was and removes the `.part`.
pub fn store(file: &Path, bytes: &[u8]) -> io::Result<()> {
    store_via(file, bytes, |part, bytes| {
        let mut out = std::fs::File::create(part)?;
        out.write_all(bytes)?;
        out.sync_all()
    })
}

/// [`store`] with the write itself handed in, so a test can make it fail
/// halfway through.
fn store_via(
    file: &Path,
    bytes: &[u8],
    write: impl FnOnce(&Path, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    /// Tells apart two writes of one picture in flight at once.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let part = file.with_extension(format!(
        "{}-{}.part",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let stored = write(&part, bytes).and_then(|()| std::fs::rename(&part, file));
    if stored.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    stored
}

/// Logs a cache failure once per operation and kind of error, since a
/// cache that cannot be written fails the same way for every picture.
fn warn_once(doing: &'static str, file: &Path, err: &io::Error) {
    /// What has been said already.
    static SAID: Mutex<Vec<(&str, io::ErrorKind)>> = Mutex::new(Vec::new());
    let cause = (doing, err.kind());
    let first = SAID.lock().is_ok_and(|mut said| {
        let first = !said.contains(&cause);
        if first {
            said.push(cause);
        }
        first
    });
    if first {
        let dir = file.parent().unwrap_or(file);
        warn!(
            "cannot {doing} the card art cache in {} ({err}); pictures still load, \
             and are downloaded again",
            dir.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;

    /// The picture every test server hands out.
    const PICTURE: &[u8] = b"\xff\xd8 a picture \xff\xd9";

    /// A web server on loopback that answers every request with `head` and
    /// [`PICTURE`], counting requests. Returns its art base (`…/art`).
    fn serve(head: String) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let base = format!("http://{}/art", listener.local_addr().expect("bound"));
        let served = Arc::new(AtomicUsize::new(0));
        let count = served.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut chunk = [0_u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => request.extend_from_slice(&chunk[..n]),
                    }
                }
                count.fetch_add(1, Ordering::SeqCst);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    PICTURE.len()
                );
                let _ = stream.write_all(PICTURE);
            }
        });
        (base, served)
    }

    /// The `http` reader with `base` as the mirror in force.
    fn reader(base: &str, cache: Option<PathBuf>) -> ArtReader {
        let base: Arc<str> = base.into();
        ArtReader {
            scheme: "http",
            cache,
            base: Arc::new(move || base.clone()),
        }
    }

    /// `url` read the way the asset server reads it.
    fn read(reader: &ArtReader, url: &str) -> Result<Vec<u8>, AssetReaderError> {
        let path = Path::new(url.strip_prefix("http://").expect("an http url"));
        bevy::tasks::block_on(async {
            let mut bytes = Vec::new();
            reader.read(path).await?.read_to_end(&mut bytes).await?;
            Ok(bytes)
        })
    }

    /// A directory of this test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baylee-art-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("made");
        dir
    }

    /// The owner's report (#250): a cache that may not be written, as bevy's
    /// was in `/`, costs a download and not the picture.
    #[cfg(unix)]
    #[test]
    fn a_picture_loads_when_its_cache_cannot_be_written() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("locked");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("locked");
        let cache = dir.join("art");
        assert!(
            std::fs::create_dir(&cache).is_err(),
            "{} was writable, so this proves nothing (running as root?)",
            dir.display()
        );
        let (base, served) = serve("200 OK".into());
        let art = reader(&base, Some(cache.clone()));

        let url = format!("{base}/normal/front/a/b/one.jpg");
        assert_eq!(read(&art, &url).expect("the picture loads"), PICTURE);
        assert_eq!(read(&art, &url).expect("and again"), PICTURE);
        assert_eq!(served.load(Ordering::SeqCst), 2, "downloaded both times");
        assert!(!cache.exists());

        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("unlocked");
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }

    /// A picture that was cached is read back from disk, not asked for again.
    #[test]
    fn a_cached_picture_is_read_back_without_asking_again() {
        let dir = scratch("cached");
        let (base, served) = serve("200 OK".into());
        let art = reader(&base, Some(dir.join("art")));

        let url = format!("{base}/normal/front/a/b/two.jpg");
        assert_eq!(read(&art, &url).expect("downloaded"), PICTURE);
        assert_eq!(read(&art, &url).expect("cached"), PICTURE);
        assert_eq!(served.load(Ordering::SeqCst), 1, "the second came off disk");
        assert_eq!(
            std::fs::read(dir.join("art").join(key(&url))).expect("the cache file"),
            PICTURE
        );
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }

    /// A missing picture is `NotFound`, as bevy's reader said, and is not
    /// cached.
    #[test]
    fn a_missing_picture_is_not_found_and_not_cached() {
        let dir = scratch("missing");
        let (base, _) = serve("404 Not Found".into());
        let art = reader(&base, Some(dir.join("art")));
        let url = format!("{base}/normal/front/a/b/gone.jpg");
        assert!(matches!(
            read(&art, &url),
            Err(AssetReaderError::NotFound(_))
        ));
        assert!(!dir.join("art").join(key(&url)).exists());
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }

    /// Only where card art comes from is asked, and a refusal names the host
    /// before a request leaves.
    #[test]
    fn only_where_card_art_comes_from_is_asked() {
        let mirror = "http://127.0.0.1:28766/art";
        for url in [
            "https://cards.scryfall.io/normal/front/a/b/x.jpg",
            "https://backs.scryfall.io/normal/0/a/x.jpg",
            "http://127.0.0.1:28766/art/normal/front/a/b/x.jpg",
        ] {
            assert_eq!(allowed(url, mirror), Ok(()), "{url}");
        }
        for (url, host) in [
            ("https://evil.example/x.jpg", "evil.example"),
            (
                "https://cards.scryfall.io.evil.example/x.jpg",
                "cards.scryfall.io.evil.example",
            ),
            (
                "http://cards.scryfall.io/normal/front/a/b/x.jpg",
                "cards.scryfall.io",
            ),
            ("http://127.0.0.1:28766/artifacts/x.jpg", "127.0.0.1:28766"),
            (
                "http://127.0.0.1:28766/art/../auth/config",
                "127.0.0.1:28766",
            ),
            (
                "http://127.0.0.1:28767/art/normal/front/a/b/x.jpg",
                "127.0.0.1:28767",
            ),
        ] {
            let refusal = allowed(url, mirror).expect_err(url);
            assert!(refusal.contains(host), "{url}: {refusal}");
        }

        // Through the reader: a server that is not the mirror is never asked.
        let (elsewhere, served) = serve("200 OK".into());
        let art = reader(mirror, None);
        let refused = read(&art, &format!("{elsewhere}/normal/front/a/b/x.jpg"));
        assert!(
            matches!(&refused, Err(AssetReaderError::Io(err)) if err.to_string().contains("127.0.0.1")),
            "{refused:?}"
        );
        assert_eq!(served.load(Ordering::SeqCst), 0, "no request left");
    }

    /// A redirect is not followed. The mirror is allowed, and the place it
    /// points to is not.
    #[test]
    fn a_redirect_is_not_followed() {
        let (elsewhere, followed) = serve("200 OK".into());
        let (mirror, _) = serve(format!("302 Found\r\nLocation: {elsewhere}/x.jpg"));
        let art = reader(&mirror, None);
        assert!(read(&art, &format!("{mirror}/normal/front/a/b/x.jpg")).is_err());
        assert_eq!(
            followed.load(Ordering::SeqCst),
            0,
            "the redirect was followed"
        );
    }

    /// A write that dies halfway leaves no half picture under the cache name,
    /// and an older whole one survives it.
    #[test]
    fn a_half_written_picture_is_never_the_cached_one() {
        let dir = scratch("atomic");
        let file = dir.join("art").join(key("http://mirror/one.jpg"));
        let halfway = |part: &Path, bytes: &[u8]| {
            std::fs::write(part, &bytes[..bytes.len() / 2])?;
            Err(io::Error::other("the process died here"))
        };

        assert!(store_via(&file, PICTURE, halfway).is_err());
        assert!(!file.exists(), "half a picture was left under its name");

        store(&file, b"older").expect("stored");
        assert!(store_via(&file, PICTURE, halfway).is_err());
        assert_eq!(std::fs::read(&file).expect("still there"), b"older");

        store(&file, PICTURE).expect("stored");
        assert_eq!(std::fs::read(&file).expect("replaced"), PICTURE);
        let left: Vec<_> = std::fs::read_dir(dir.join("art"))
            .expect("listed")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(left.len(), 1, "a `.part` was left behind: {left:?}");
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }
}
