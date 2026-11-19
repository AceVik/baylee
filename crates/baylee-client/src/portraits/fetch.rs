//! The worker that fetches, keeps and decodes pictures: one thread, one
//! picture at a time, never the render thread.
//!
//! - **Where from.** Only the picture origin the main thread hands it
//!   ([`super::picture_origin`]): `https://`, or `http://` on a numeric
//!   loopback address in a development build, with no path, user or query
//!   ([`origin_ok`]). The path is Central's own (`picture_path`). No
//!   redirect is followed, no proxy is used, no cookie or bearer is sent.
//! - **How much.** A still at most [`MAX_STILL_BYTES`], a sequence at most
//!   `Bounds::PICTURE.max_bytes`, both checked against `Content-Length`
//!   before the body and against the body as it is read; the answer must say
//!   the type the file is.
//! - **Kept.** The raw bytes go to the cache directory under the key
//!   (`portrait::file_name`), and a file there is read back through the
//!   same bounded decode as a download. The directory is held to
//!   [`DISK_CAP_BYTES`], oldest first.
//! - **Decoded.** A still is a PNG of exactly 256 px square, read by the
//!   `image` crate with limits; a moving picture is decoded by the child
//!   process (`helper`). Both are kept at `KEPT_SIDE`.

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use baylee_account::sdk::{PictureFile, picture_path};
use baylee_client_core::portrait::{
    DISK_CAP_BYTES, File, Kept, MAX_STILL_BYTES, PictureKey, SERVED_SIDE, file_name, halve,
    over_cap, parse_file_name,
};

use super::helper::{self, Frames};

/// Jobs waiting for the worker, at most.
pub(super) const QUEUE: usize = 8;

/// What the worker is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Job {
    /// The still of `key`, from `origin` unless the disk has it.
    Still { origin: String, key: PictureKey },
    /// The frames of `key`, which moves.
    Moving { origin: String, key: PictureKey },
    /// `key`'s files are no one's any more: removed from the disk.
    Forget { key: PictureKey },
}

/// What the worker answers.
#[derive(Debug)]
pub(super) enum Done {
    /// A still at `KEPT_SIDE`, RGBA.
    Still { key: PictureKey, rgba: Vec<u8> },
    /// A moving picture's frames.
    Moving { key: PictureKey, frames: Frames },
    /// Nothing to show for `key`'s `file`; `again` when it may come later
    /// (no answer, or Central busy), else for this session.
    Failed {
        key: PictureKey,
        file: File,
        again: bool,
    },
}

/// Why a download gave nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Miss {
    /// Worth asking again later: no answer, `429`, `5xx`.
    Later,
    /// Not this picture, this session: `404`, a wrong type or size, a
    /// refused decode.
    Never,
}

/// The worker's two ends, held by the main thread.
pub(super) struct Courier {
    send: mpsc::SyncSender<Job>,
    receive: Mutex<mpsc::Receiver<Done>>,
}

impl Courier {
    /// Starts the worker, keeping its files in `dir` (none: nothing kept).
    pub(super) fn start(dir: Option<PathBuf>) -> Option<Self> {
        let (send, jobs) = mpsc::sync_channel::<Job>(QUEUE);
        let (done, receive) = mpsc::channel::<Done>();
        std::thread::Builder::new()
            .name("baylee-portraits".into())
            .spawn(move || {
                let shelf = dir.map(Shelf::open);
                while let Ok(job) = jobs.recv() {
                    if let Some(answer) = work(shelf.as_ref(), job)
                        && done.send(answer).is_err()
                    {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self {
            send,
            receive: Mutex::new(receive),
        })
    }

    /// Hands the worker a job; `false` when its queue is full.
    pub(super) fn ask(&self, job: Job) -> bool {
        self.send.try_send(job).is_ok()
    }

    /// What the worker has finished since the last look.
    pub(super) fn take(&self) -> Vec<Done> {
        self.receive
            .lock()
            .map(|receive| receive.try_iter().collect())
            .unwrap_or_default()
    }
}

fn work(shelf: Option<&Shelf>, job: Job) -> Option<Done> {
    match job {
        Job::Forget { key } => {
            if let Some(shelf) = shelf {
                shelf.remove(&key);
            }
            None
        }
        Job::Still { origin, key } => Some(
            match fetch(shelf, &origin, &key, File::Still).and_then(|bytes| {
                still(&bytes).ok_or(Miss::Never).inspect_err(|_| {
                    if let Some(shelf) = shelf {
                        shelf.remove_file(&key, File::Still);
                    }
                })
            }) {
                Ok(rgba) => Done::Still { key, rgba },
                Err(miss) => Done::Failed {
                    key,
                    file: File::Still,
                    again: miss == Miss::Later,
                },
            },
        ),
        Job::Moving { origin, key } => Some(
            match fetch(shelf, &origin, &key, File::Moving).and_then(|bytes| {
                helper::command()
                    .map_err(|_| Miss::Never)
                    .and_then(|command| {
                        helper::decode(command, &bytes, helper::DEADLINE).map_err(|_| Miss::Never)
                    })
                    .inspect_err(|_| {
                        if let Some(shelf) = shelf {
                            shelf.remove_file(&key, File::Moving);
                        }
                    })
            }) {
                Ok(frames) => Done::Moving { key, frames },
                Err(miss) => Done::Failed {
                    key,
                    file: File::Moving,
                    again: miss == Miss::Later,
                },
            },
        ),
    }
}

/// The file's bytes, from the disk or downloaded (and then kept).
fn fetch(
    shelf: Option<&Shelf>,
    origin: &str,
    key: &PictureKey,
    file: File,
) -> Result<Vec<u8>, Miss> {
    if let Some(bytes) = shelf.and_then(|shelf| shelf.read(key, file)) {
        return Ok(bytes);
    }
    let bytes = download(origin, key, file)?;
    if let Some(shelf) = shelf {
        shelf.write(key, file, &bytes);
    }
    Ok(bytes)
}

/// The most bytes a file may have.
pub(super) fn limit(file: File) -> usize {
    match file {
        File::Still => MAX_STILL_BYTES,
        File::Moving => baylee_avif::Bounds::PICTURE.max_bytes,
    }
}

/// The type an answer must say for a file.
const fn kind(file: File) -> &'static str {
    match file {
        File::Still => "image/png",
        File::Moving => "image/avif",
    }
}

/// Whether `origin` is one pictures may come from: `https://host[:port]`,
/// or `http://` on a numeric loopback address with a port; nothing else in
/// it.
#[must_use]
pub(super) fn origin_ok(origin: &str) -> bool {
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    let plain = url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.origin().ascii_serialization() == origin;
    let place = match (url.scheme(), url.host()) {
        ("https", Some(url::Host::Domain(_))) => true,
        ("http", Some(url::Host::Ipv4(ip))) => ip.is_loopback() && url.port().is_some(),
        ("http", Some(url::Host::Ipv6(ip))) => ip.is_loopback() && url.port().is_some(),
        _ => false,
    };
    plain && place
}

/// The URL of `key`'s `file` under `origin`.
pub(super) fn url(origin: &str, key: &PictureKey, file: File) -> String {
    let which = match file {
        File::Still => PictureFile::Png,
        File::Moving => PictureFile::Sequence,
    };
    format!("{origin}{}", picture_path(key, which))
}

/// The agent pictures are fetched with: certificates checked, no redirect,
/// no proxy, statuses as answers, short timeouts; `https_only` unless the
/// origin is loopback.
fn agent(https_only: bool) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(crate::transport::tls(false))
        .https_only(https_only)
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .user_agent("baylee-client")
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(20)))
        .timeout_recv_body(Some(Duration::from_secs(10)))
        .build()
        .new_agent()
}

/// Downloads `key`'s `file` from `origin` within its bounds.
pub(super) fn download(origin: &str, key: &PictureKey, file: File) -> Result<Vec<u8>, Miss> {
    if !origin_ok(origin) {
        return Err(Miss::Never);
    }
    let agent = agent(origin.starts_with("https://"));
    let response = agent
        .get(url(origin, key, file))
        .header("Accept", kind(file))
        .call()
        .map_err(|_| Miss::Later)?;
    read_answer(response, file)
}

/// An answer's body, when it is the file within its bounds.
pub(super) fn read_answer(
    mut response: ureq::http::Response<ureq::Body>,
    file: File,
) -> Result<Vec<u8>, Miss> {
    match response.status().as_u16() {
        200 => {}
        429 | 500..=599 => return Err(Miss::Later),
        _ => return Err(Miss::Never),
    }
    let headers = response.headers();
    let typed = headers
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(kind(file)));
    let limit = limit(file);
    let length = headers
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .map(str::parse::<u64>);
    if !typed || matches!(length, Some(Err(_))) || matches!(length, Some(Ok(n)) if n > limit as u64)
    {
        return Err(Miss::Never);
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Miss::Later)?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(Miss::Never);
    }
    Ok(bytes)
}

/// A still's pixels at `KEPT_SIDE`: a PNG of exactly 256 px square, read
/// with limits on its size and on what decoding it may allocate.
#[must_use]
pub(super) fn still(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() > MAX_STILL_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(SERVED_SIDE);
    limits.max_image_height = Some(SERVED_SIDE);
    // Four 16-bit channels of 256 × 256 and the decoder's own rows.
    limits.max_alloc = Some(4 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    if decoded.width() != SERVED_SIDE || decoded.height() != SERVED_SIDE {
        return None;
    }
    halve(decoded.to_rgba8().as_raw(), SERVED_SIDE)
}

/// The pictures' directory in the cache.
pub(super) struct Shelf {
    pub(super) dir: PathBuf,
}

impl Shelf {
    pub(super) fn open(dir: PathBuf) -> Self {
        let shelf = Self { dir };
        shelf.trim();
        shelf
    }

    fn path(&self, key: &PictureKey, file: File) -> PathBuf {
        self.dir.join(file_name(key, file))
    }

    pub(super) fn read(&self, key: &PictureKey, file: File) -> Option<Vec<u8>> {
        let path = self.path(key, file);
        let meta = std::fs::symlink_metadata(&path).ok()?;
        if !meta.is_file() || meta.len() == 0 || meta.len() > limit(file) as u64 {
            return None;
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .ok()?
            .take(limit(file) as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() <= limit(file)).then_some(bytes)
    }

    pub(super) fn write(&self, key: &PictureKey, file: File, bytes: &[u8]) {
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        let path = self.path(key, file);
        let partial = path.with_extension("part");
        if std::fs::write(&partial, bytes).is_ok() && std::fs::rename(&partial, &path).is_err() {
            let _ = std::fs::remove_file(&partial);
        }
        self.trim();
    }

    pub(super) fn remove_file(&self, key: &PictureKey, file: File) {
        let _ = std::fs::remove_file(self.path(key, file));
    }

    pub(super) fn remove(&self, key: &PictureKey) {
        self.remove_file(key, File::Still);
        self.remove_file(key, File::Moving);
    }

    /// Holds the directory to [`DISK_CAP_BYTES`]; only the cache's own
    /// files count and only they are removed.
    pub(super) fn trim(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut kept = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if parse_file_name(name).is_none() || !meta.is_file() {
                continue;
            }
            let written = meta
                .modified()
                .ok()
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |at| u64::try_from(at.as_millis()).unwrap_or(u64::MAX));
            kept.push(Kept {
                name: name.to_owned(),
                bytes: meta.len(),
                written,
            });
        }
        for name in over_cap(&kept, DISK_CAP_BYTES) {
            let _ = std::fs::remove_file(self.dir.join(name));
        }
    }
}

/// The pictures' directory under the client's cache.
#[must_use]
pub(super) fn cache_dir() -> Option<PathBuf> {
    crate::artreader::cache_home().map(|home| home.join("pictures"))
}
