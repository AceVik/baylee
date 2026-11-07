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
//! - a picture is filed under what is drawn and not where it came from
//!   ([`key`]), so the CDN's copy is the one a draw reads under the mirror's
//!   address, and the other way round;
//! - it asks only where card art comes from ([`allowed`]): the Scryfall CDNs
//!   and the gateway mirror the client was told to use. It follows no
//!   redirect, since a redirect could lead anywhere else;
//! - it shows the signed-in session to the gateway's mirror, which serves
//!   nobody else (#273), and to nothing but the mirror ([`authorization`]):
//!   never to a Scryfall host, and never in a URL, where it would land in logs.
//!
//! The background fill stores through [`fill_one`], which is this file's
//! allowlist, download, limits and `.part` under another door, so what it
//! files is exactly what [`ArtReader`] reads. A draw has nothing in front of
//! it: it only tells the fill it is on its way (`crate::cachefill`).
//!
//! A browser keeps bevy's fetch reader (`crates/baylee-client/Cargo.toml`),
//! because the browser caches.

use crate::cachefill::courier::{self, Courier};
use baylee_client_core::cachefill::{Answer, retry_after};
use baylee_client_core::images::{self, SCRYFALL_BACKS_CDN, SCRYFALL_CDN};
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetReaderFuture, AssetSourceBuilder, PathStream, Reader,
    VecReader,
};
use bevy::prelude::*;
use bevy::tasks::ConditionalSendFuture;
use std::fmt::Write as _;
use std::future::ready;
use std::io::{self, Read as _, Write as _};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

/// The largest picture fetched. Scryfall's largest, `png`, is under 2 MB.
const LARGEST: u64 = 16 * 1024 * 1024;

/// The longest side a card picture may say it has. Scryfall's largest is
/// 745 × 1040, and a texture past what the GPU takes ends the client.
const LONGEST_SIDE: u32 = 2048;

/// Whether `bytes` say they are a picture of a card's size, by their header
/// alone, so nothing is decoded and the fill stays free of that (`fill_one`).
///
/// A gateway's mirror answers what it likes, and what is kept here is drawn
/// at every other table too ([`key`]), so a picture is judged before it is
/// kept and again when it is read.
fn picture_fits(bytes: &[u8]) -> bool {
    image::ImageReader::new(io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .is_some_and(|(width, height)| {
            (1..=LONGEST_SIDE).contains(&width) && (1..=LONGEST_SIDE).contains(&height)
        })
}

/// The base a request may go to besides the Scryfall CDNs, asked for on every
/// request because signing in changes it.
type Base = Arc<dyn Fn() -> Arc<str> + Send + Sync>;

/// The session the mirror is shown, asked for on every request for the same
/// reason.
type Session = Arc<dyn Fn() -> Option<Arc<str>> + Send + Sync>;

/// The signed-in session's token, which the lobby keeps here as it signs in
/// and out ([`use_session`]).
static SESSION: std::sync::RwLock<Option<Arc<str>>> = std::sync::RwLock::new(None);

/// Keeps the token the gateway's mirror is shown, or forgets it.
pub fn use_session(token: Option<&str>) {
    if let Ok(mut session) = SESSION.write()
        && session.as_deref() != token
    {
        *session = token.map(Into::into);
    }
}

/// The token in force.
fn session() -> Option<Arc<str>> {
    SESSION.read().ok().and_then(|session| session.clone())
}

/// Registers [`ArtReader`] as the `http` and `https` asset sources.
///
/// It has to be added before `AssetPlugin`, which builds its sources from
/// whatever has been registered by then.
pub struct ArtReaderPlugin {
    /// Where pictures are cached. `None` caches nothing.
    pub cache: Option<PathBuf>,
    /// The mirror in force ([`images::art_base`] in the app).
    pub base: Base,
    /// The session the mirror is shown ([`use_session`] in the app).
    pub session: Session,
    /// Who is told that a wanted picture is on its way
    /// ([`courier::shared`] in the app).
    pub courier: Arc<Courier>,
}

impl Default for ArtReaderPlugin {
    fn default() -> Self {
        Self {
            cache: art_cache(),
            base: Arc::new(images::art_base),
            session: Arc::new(session),
            courier: courier::shared(),
        }
    }
}

/// Where card art from Scryfall is cached on this device, if anywhere: what
/// every gateway may draw, because only Scryfall's own answers are filed here.
/// A gateway's mirror has a shelf of its own ([`mirror_shelf`]).
#[must_use]
pub fn art_cache() -> Option<PathBuf> {
    cache_home().map(|home| home.join("art").join("shelf"))
}

/// Where every gateway's mirror keeps its pictures, below `cache`.
pub(crate) fn mirror_shelves(cache: &Path) -> PathBuf {
    cache.join("mirrors")
}

/// Where the mirror at `base` keeps its pictures. A gateway may answer any
/// valid picture for any printing, so what it says is drawn at that gateway
/// and no other (owner, 07.11.2026: duplicate downloads are accepted).
fn mirror_shelf(cache: &Path, base: &str) -> PathBuf {
    mirror_shelves(cache).join(baylee_client_core::cardquery::gateway_tag(base))
}

/// Whether `url` is Scryfall's own, whatever the art base in force is.
fn from_scryfall(url: &str) -> bool {
    [images::SCRYFALL_CDN, images::SCRYFALL_BACKS_CDN]
        .into_iter()
        .any(|root| {
            url.strip_prefix(root.trim_end_matches('/'))
                .is_some_and(|rest| rest.starts_with('/'))
        })
}

/// Where a picture fetched from `url` is filed.
fn file_of(cache: &Path, url: &str, base: &str) -> PathBuf {
    if from_scryfall(url) {
        cache.join(key(url, base))
    } else {
        mirror_shelf(cache, base).join(key(url, base))
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
                session: self.session.clone(),
                courier: self.courier.clone(),
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
    /// The session the mirror is shown.
    pub session: Session,
    /// Who is told that a wanted picture is on its way.
    pub courier: Arc<Courier>,
}

impl AssetReader for ArtReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<VecReader, AssetReaderError> {
        let Some(url) = path
            .to_str()
            .map(|rest| format!("{}://{}", self.scheme, rest.replace(MAIN_SEPARATOR, "/")))
        else {
            return Err(AssetReaderError::NotFound(path.to_path_buf()));
        };
        let base = (self.base)();
        allowed(&url, &base)
            .map_err(|refusal| AssetReaderError::Io(Arc::new(io::Error::other(refusal))))?;
        let shown = authorization(&url, &base, (self.session)().as_deref());
        // Scryfall's copy is good at any gateway, so it comes first.
        let scryfall = self.cache.as_ref().map(|dir| dir.join(key(&url, &base)));
        let file = self.cache.as_ref().map(|dir| file_of(dir, &url, &base));
        let asked = path.to_path_buf();
        let courier = self.courier.clone();
        // Off the asset executor: the download and the cache are both
        // blocking, and a table asks for dozens of pictures at once.
        let bytes = blocking::unblock(move || {
            let held = scryfall.as_deref().and_then(cached);
            match held {
                Some(bytes) => Ok(bytes),
                None => fetch(&url, shown.as_deref(), file.as_deref(), asked, &courier),
            }
        })
        .await?;
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

/// The `Authorization` a request for `url` carries: the session, for the
/// gateway's mirror at `base` and for nothing else.
///
/// Never for a Scryfall host, even one that is `base` (a gateway without a
/// mirror leaves the CDN as the base): the session is the player's key to
/// their account, and a third party must never be handed it.
#[must_use]
pub fn authorization(url: &str, base: &str, session: Option<&str>) -> Option<String> {
    let session = session?;
    let host = url.split_once("://").map_or("", |(_, rest)| {
        rest.split(['/', '?', '#']).next().unwrap_or(rest)
    });
    let name = host.rsplit_once('@').map_or(host, |(_, name)| name);
    let name = name.split(':').next().unwrap_or(name).to_ascii_lowercase();
    if name == "scryfall.io" || name.ends_with(".scryfall.io") {
        return None;
    }
    let under = url
        .strip_prefix(base.trim_end_matches('/'))
        .is_some_and(|rest| rest.starts_with('/'));
    under.then(|| format!("Bearer {session}"))
}

/// The per-user cache directory, `…/baylee`.
///
/// `$XDG_CACHE_HOME` when it is set and absolute (the XDG spec ignores a
/// relative one), on every desktop, as `settings` reads `$XDG_CONFIG_HOME`;
/// else `~/Library/Caches` on macOS and iOS (whose `HOME` is the app's
/// container), `%LOCALAPPDATA%` on Windows, `~/.cache` elsewhere, and on
/// Android the app's own cache directory. Every answer but Android's is
/// `baylee_client_core::userdirs::user_dir`, where each system's is tested.
#[must_use]
pub fn cache_home() -> Option<PathBuf> {
    use baylee_client_core::userdirs::{Kind, Os, real_env, user_dir};
    user_dir(Kind::Cache, Os::current(), &real_env).or_else(android_cache)
}

/// Android's cache directory: `getCacheDir()` is `cache` beside
/// `getFilesDir()`, which is the one path a NativeActivity is handed.
#[cfg(target_os = "android")]
fn android_cache() -> Option<PathBuf> {
    bevy::android::ANDROID_APP
        .get()
        .and_then(|app| app.internal_data_path())
        .and_then(|files| Some(files.parent()?.join("cache").join("baylee")))
}

/// Only Android is handed its cache directory rather than reading it from
/// the environment.
#[cfg(not(target_os = "android"))]
const fn android_cache() -> Option<PathBuf> {
    None
}

/// The one client every picture is fetched with.
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    // The system's trust store, with verification enabled in every build
    // profile (`crate::transport`).
    ureq::Agent::config_builder()
        .tls_config(crate::transport::tls(true))
        .user_agent(crate::cardtext::scryfall::AGENT)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        // A refusal has to arrive as an answer: `Retry-After` is one of its
        // headers, and the error ureq makes of it keeps only the status.
        .http_status_as_error(false)
        .build()
        .new_agent()
});

/// The picture at `url`, for a draw: from `file` when the cache has it, else
/// downloaded at once, showing `shown` as its `Authorization` when there is
/// one, and then cached. Blocking.
fn fetch(
    url: &str,
    shown: Option<&str>,
    file: Option<&Path>,
    asked: PathBuf,
    courier: &Courier,
) -> Result<Vec<u8>, AssetReaderError> {
    if let Some(bytes) = file.and_then(cached) {
        return Ok(bytes);
    }
    let bytes = {
        // Nothing waits here: the fill is told, and gives way.
        let _on_its_way = courier.wanted();
        download(url, shown, &|| false).map_err(|failed| failed.into_asset_error(url, asked))?
    };
    if let Some(file) = file
        && let Err(err) = store(file, &bytes)
    {
        warn_once("write", file, &err);
    }
    Ok(bytes)
}

/// The picture in `file`, if the cache holds it. A cache that cannot be read
/// is one warning and no picture.
fn cached(file: &Path) -> Option<Vec<u8>> {
    match std::fs::read(file) {
        Ok(bytes) if picture_fits(&bytes) => Some(bytes),
        Ok(_) => None,
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            warn_once("read", file, &err);
            None
        }
    }
}

/// One picture for the background fill, into the cache and nowhere else: no
/// bytes come back, nothing is decoded and no texture is made.
///
/// `url` lies under `base`, which is Scryfall's CDN in the app, so no session
/// is shown: the fill signs in to nothing. It is checked as a draw's is and
/// goes through [`download`], [`key`] and [`store`] as a draw's does.
/// `give_way` is asked between two reads of the body: a wanted picture stops
/// this one, and it is fetched again later.
pub(crate) fn fill_one(url: &str, base: &str, cache: &Path, give_way: &dyn Fn() -> bool) -> Answer {
    // A plan names nothing else, so this is a bug, and the fill stops on it.
    if allowed(url, base).is_err() {
        return Answer::Refused(0);
    }
    let file = cache.join(key(url, base));
    if std::fs::metadata(&file).is_ok_and(|meta| meta.len() > 0) {
        return Answer::Already;
    }
    match download(url, None, give_way) {
        Ok(bytes) => match store(&file, &bytes) {
            Ok(()) => Answer::Stored(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
            Err(err) => {
                warn_once("write", &file, &err);
                Answer::CannotWrite
            }
        },
        Err(failed) => failed.answer(),
    }
}

/// How a request ended without a picture.
#[derive(Debug)]
enum Failed {
    /// A status other than a success (a redirect among them: with none
    /// followed, it arrives as an answer, and its body is not the picture),
    /// and what `Retry-After` said.
    Status(u16, Option<Duration>),
    /// No usable answer.
    Transport(String),
    /// It stopped for a wanted picture.
    GaveWay,
}

impl Failed {
    /// As the fill counts it.
    fn answer(&self) -> Answer {
        match *self {
            Self::Status(404, _) => Answer::Missing,
            Self::Status(429 | 503, asked) => Answer::Busy(asked),
            Self::Status(code @ 500..=599, asked) => Answer::Server(code, asked),
            Self::Status(code, _) => Answer::Refused(code),
            Self::Transport(_) => Answer::Unreachable,
            Self::GaveWay => Answer::GaveWay,
        }
    }

    /// The error bevy's own reader gave for the same answer.
    fn into_asset_error(self, url: &str, asked: PathBuf) -> AssetReaderError {
        match self {
            Self::Status(404, _) => AssetReaderError::NotFound(asked),
            Self::Status(code, _) => AssetReaderError::HttpError(code),
            Self::Transport(said) => {
                AssetReaderError::Io(Arc::new(io::Error::other(format!("{url}: {said}"))))
            }
            Self::GaveWay => {
                AssetReaderError::Io(Arc::new(io::Error::other(format!("{url}: gave way"))))
            }
        }
    }
}

/// `url`'s body. Only a success is a picture. `give_way` is asked between two
/// reads, and a draw's says no.
fn download(
    url: &str,
    shown: Option<&str>,
    give_way: &dyn Fn() -> bool,
) -> Result<Vec<u8>, Failed> {
    let request = AGENT.get(url);
    let request = match shown {
        Some(value) => request.header("Authorization", value),
        None => request,
    };
    let mut response = request
        .call()
        .map_err(|err| Failed::Transport(err.to_string()))?;
    if !response.status().is_success() {
        let unix_now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |now| now.as_secs());
        let asked = response
            .headers()
            .get("Retry-After")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| retry_after(value, unix_now));
        return Err(Failed::Status(response.status().as_u16(), asked));
    }
    let mut body = response.body_mut().with_config().limit(LARGEST).reader();
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        if give_way() {
            return Err(Failed::GaveWay);
        }
        match body.read(&mut chunk) {
            Ok(0) if picture_fits(&bytes) => return Ok(bytes),
            Ok(0) => return Err(Failed::Transport("not a card's picture".into())),
            Ok(read) => bytes.extend_from_slice(&chunk[..read]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(Failed::Transport(err.to_string())),
        }
    }
}

/// The cache file name for `url`, a picture under `base`: the SHA-256 of what
/// is drawn ([`images::art_path`]), so any path makes a safe name, no two
/// pictures share one, and neither the CDN nor a mirror has a copy of its own.
///
/// Until 04.11.2026 the hash was of the whole URL. Those files can no more be
/// told from a URL than be turned into one, so they are not read: a picture
/// is fetched again, and `.jpg` on the new names is what lets a listing clear
/// the old ones away (`cachefill::worker::list`).
pub(crate) fn key(url: &str, base: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut name = String::with_capacity(68);
    for byte in Sha256::digest(images::art_path(url, base).as_bytes()) {
        let _ = write!(name, "{byte:02x}");
    }
    name.push_str(".jpg");
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
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;

    /// The picture every test server hands out: a real one, one pixel of PNG,
    /// because anything else is not kept ([`picture_fits`]).
    const PICTURE: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89\0\0\0\rIDATx\x9cc\0\x01\0\0\x05\0\x01\r\n-\xb4\0\0\0\0IEND\xaeB`\x82";

    /// What a mirror says it is sent is judged by its header: a picture of a
    /// card's size passes, and bytes that are none, or a picture of a size
    /// the GPU refuses, are neither kept nor handed to the decoder.
    #[test]
    fn only_a_picture_of_a_card_s_size_is_kept() {
        let png = |width, height| {
            let mut out = io::Cursor::new(Vec::new());
            image::DynamicImage::ImageLuma8(image::GrayImage::new(width, height))
                .write_to(&mut out, image::ImageFormat::Png)
                .expect("a png");
            out.into_inner()
        };
        let jpeg = |width, height| {
            let mut out = io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height))
                .write_to(&mut out, image::ImageFormat::Jpeg)
                .expect("a jpeg");
            out.into_inner()
        };
        assert!(picture_fits(PICTURE));
        assert!(picture_fits(&png(488, 680)));
        assert!(picture_fits(&jpeg(488, 680)));
        assert!(!picture_fits(&png(65_000, 1)));
        assert!(!picture_fits(&jpeg(4000, 1)));
        assert!(!picture_fits(b"<html>not a picture</html>"));
        assert!(!picture_fits(b""));
    }

    /// A cache that already holds something else is a miss, so a bad file
    /// costs a download and never a load.
    #[test]
    fn a_cached_file_that_is_no_picture_is_not_read() {
        let dir = scratch("no-picture");
        let file = dir.join("kept.jpg");
        store(&file, b"<html>not a picture</html>").expect("stored");
        assert!(cached(&file).is_none());
        store(&file, PICTURE).expect("stored");
        assert_eq!(cached(&file).as_deref(), Some(PICTURE));
    }

    /// A web server on loopback that answers every request with `head` and
    /// [`PICTURE`], counting requests. Returns its art base (`…/art`).
    fn serve(head: String) -> (String, Arc<AtomicUsize>) {
        let (base, served, _) = listen(head);
        (base, served)
    }

    /// The same, keeping the head of every request it was sent.
    fn serve_seen(head: String) -> (String, Arc<Mutex<Vec<String>>>) {
        let (base, _, seen) = listen(head);
        (base, seen)
    }

    /// [`serve`] and [`serve_seen`] in one.
    fn listen(head: String) -> (String, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let base = format!("http://{}/art", listener.local_addr().expect("bound"));
        let served = Arc::new(AtomicUsize::new(0));
        let count = served.clone();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let heads = seen.clone();
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
                if let Ok(mut heads) = heads.lock() {
                    heads.push(String::from_utf8_lossy(&request).to_ascii_lowercase());
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
        (base, served, seen)
    }

    /// The `http` reader with `base` as the mirror in force, signed out.
    fn reader(base: &str, cache: Option<PathBuf>) -> ArtReader {
        let base: Arc<str> = base.into();
        ArtReader {
            scheme: "http",
            cache,
            base: Arc::new(move || base.clone()),
            session: Arc::new(|| None),
            courier: Arc::new(Courier::new(0.0)),
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
            std::fs::read(file_of(&dir.join("art"), &url, &base)).expect("the cache file"),
            PICTURE
        );
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }

    /// A gateway may answer any valid picture for a printing, so what it
    /// answered is drawn at that gateway and at no other, and what Scryfall
    /// holds is nobody's to replace.
    #[test]
    fn what_one_mirror_said_is_not_drawn_at_another() {
        let cache = Path::new("/cache/art/shelf");
        let path = "/normal/front/a/b/two.jpg";
        let at = |base: &str| file_of(cache, &format!("{base}{path}"), base);
        let (evil, good) = ("https://evil.example/art", "https://good.example/art");
        assert_ne!(at(evil), at(good));
        assert!(
            at(evil).starts_with(mirror_shelves(cache)),
            "{:?}",
            at(evil)
        );

        let scryfall = format!("{}{path}", images::SCRYFALL_CDN);
        assert_eq!(
            file_of(cache, &scryfall, evil),
            cache.join(key(&scryfall, evil)),
            "Scryfall's own picture lies on Scryfall's shelf, whatever the mirror"
        );
        assert_eq!(
            file_of(cache, &scryfall, evil),
            file_of(cache, &scryfall, good)
        );
        assert_ne!(at(evil).parent(), Some(cache), "no mirror writes there");
    }

    /// A picture Scryfall gave is good at any gateway, so a mirror is not asked
    /// for what is already held.
    #[test]
    fn a_mirror_is_not_asked_for_what_scryfall_gave() {
        let dir = scratch("scryfall_first");
        let (base, served) = serve("200 OK".into());
        let shelf = dir.join("art").join("scryfall");
        let art = reader(&base, Some(shelf.clone()));
        let url = format!("{base}/normal/front/a/b/two.jpg");
        store(&shelf.join(key(&url, &base)), PICTURE).expect("filed");
        assert_eq!(read(&art, &url).expect("from the shelf"), PICTURE);
        assert_eq!(served.load(Ordering::SeqCst), 0);
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
        assert!(!dir.join("art").join(key(&url, &base)).exists());
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

    /// The session goes to the gateway's mirror and nowhere else (#273). A
    /// Scryfall host is never shown it, not even when it is the base, which is
    /// where a gateway without a mirror leaves it; nor is a host that only
    /// begins like the mirror, or the mirror's host outside its art.
    #[test]
    fn only_the_gateway_s_mirror_is_shown_the_session() {
        let mirror = "http://127.0.0.1:28766/art";
        let session = Some("tok");
        assert_eq!(
            authorization(
                "http://127.0.0.1:28766/art/normal/front/a/b/x.jpg",
                mirror,
                session
            ),
            Some("Bearer tok".to_string())
        );
        for base in [mirror, SCRYFALL_CDN, SCRYFALL_BACKS_CDN] {
            for url in [
                "https://cards.scryfall.io/normal/front/a/b/x.jpg",
                "https://backs.scryfall.io/normal/0/a/x.jpg",
                "https://CARDS.Scryfall.IO/normal/front/a/b/x.jpg",
                "https://api.scryfall.io/cards/x",
                "https://cards.scryfall.io:443/normal/front/a/b/x.jpg",
            ] {
                assert_eq!(
                    authorization(url, base, session),
                    None,
                    "{url} under {base}"
                );
            }
        }
        for url in [
            "http://127.0.0.1:28766/artifacts/x.jpg",
            "http://127.0.0.1:28766/auth/config",
            "http://127.0.0.1:28767/art/normal/front/a/b/x.jpg",
            "https://evil.example/art/normal/front/a/b/x.jpg",
        ] {
            assert_eq!(authorization(url, mirror, session), None, "{url}");
        }
        assert_eq!(
            authorization(
                "http://127.0.0.1:28766/art/normal/front/a/b/x.jpg",
                mirror,
                None
            ),
            None,
            "signed out, nothing is shown"
        );

        // And on the wire: the mirror is sent the header, and signed out it
        // is not.
        let (mirror, seen) = serve_seen("200 OK".into());
        let mut art = reader(&mirror, None);
        art.session = Arc::new(|| Some("tok".into()));
        read(&art, &format!("{mirror}/normal/front/a/b/x.jpg")).expect("a picture");
        let signed_out = reader(&mirror, None);
        read(&signed_out, &format!("{mirror}/normal/front/a/b/y.jpg")).expect("a picture");
        let heads = seen.lock().expect("the heads").clone();
        assert_eq!(heads.len(), 2);
        assert!(
            heads[0].contains("\r\nauthorization: bearer tok\r\n"),
            "{}",
            heads[0]
        );
        assert!(!heads[1].contains("authorization"), "{}", heads[1]);
    }

    /// A redirect is not followed. The mirror is allowed, and the place it
    /// points to is not: asked with the session, as a mirror is (#273), so a
    /// followed redirect would have carried the session there too.
    #[test]
    fn a_redirect_is_not_followed() {
        let (elsewhere, followed) = serve("200 OK".into());
        let (mirror, _) = serve(format!("302 Found\r\nLocation: {elsewhere}/x.jpg"));
        let mut art = reader(&mirror, None);
        art.session = Arc::new(|| Some("tok".into()));
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
        let file = dir
            .join("art")
            .join(key("http://mirror/one.jpg", "http://mirror"));
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
