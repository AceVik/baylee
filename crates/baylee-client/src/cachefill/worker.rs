//! The fill's own threads: they ask the [`Courier`] what to do, and do it.
//!
//! A few plain threads that sleep when there is nothing to do, rather than
//! tasks on Bevy's pools: a request blocks for as long as the network likes,
//! and a task doing that would hold a thread the frame's work needs.

use super::courier::Courier;
use crate::artreader;
use baylee_client_core::cachefill::{FILL_SLOTS, Fill, Job, Next, Reach, Source, plan};
use baylee_client_core::images;
use baylee_core::types::{SupertypeSet, TypeSet};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// How old a stray `.part` has to be before it is cleared away: a write in
/// flight lasts milliseconds, and one this old is a crash's leftover.
const STALE_PART: Duration = Duration::from_secs(3_600);

/// What a worker needs, every door that could differ from the app's handed
/// in, so a test runs one against a server on loopback and a pool of its own.
#[derive(Clone)]
pub(crate) struct Worker {
    /// Where it learns what to do.
    pub(crate) courier: Arc<Courier>,
    /// The cache directory.
    pub(crate) cache: PathBuf,
    /// Where the pictures come from: Scryfall's CDN, signed in or not.
    pub(crate) base: Arc<str>,
    /// The cards to fill.
    pub(crate) pool: fn() -> Vec<Source<'static>>,
    /// How much room the disk under a directory has.
    pub(crate) room: fn(&Path) -> Option<u64>,
}

/// Starts the workers, or says there is nowhere to put a picture.
pub(crate) fn start(courier: Arc<Courier>, cache: Option<PathBuf>) {
    let Some(cache) = cache else {
        courier.with(|fill, _| fill.no_cache());
        return;
    };
    let worker = Worker {
        courier,
        cache,
        base: images::SCRYFALL_CDN.into(),
        pool,
        room,
    };
    for at in 0..FILL_SLOTS {
        let worker = worker.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("card-fill-{at}"))
            .spawn(move || {
                loop {
                    worker.step();
                }
            });
        if let Err(err) = spawned {
            bevy::log::warn!("cannot start the card cache's fill: {err}");
        }
    }
}

impl Worker {
    /// One thing the fill wants done.
    pub(crate) fn step(&self) {
        match self.courier.next() {
            Next::Check { epoch, reach } => self.check(epoch, reach),
            Next::Measure => {
                let listing = list(&self.cache);
                let room = (self.room)(&self.cache);
                self.courier
                    .with(|fill, _| fill.measured(listing.bytes, room));
            }
            Next::Fetch(job) => self.fetch(&job),
            Next::Wipe => {
                wipe(&self.cache);
                self.courier.with(Fill::wiped);
            }
            Next::Wait(wait) => self.courier.sleep(wait),
            Next::Idle => self.courier.sleep(Duration::MAX),
        }
    }

    /// Works out what is on disk: the whole of "resuming".
    fn check(&self, epoch: u64, reach: Reach) {
        let pictures = plan(&self.base, &(self.pool)(), reach);
        let listing = list(&self.cache);
        let present: Vec<bool> = pictures
            .iter()
            .map(|picture| {
                listing
                    .names
                    .contains(&artreader::key(&picture.url, &self.base))
            })
            .collect();
        let room = (self.room)(&self.cache);
        self.courier.with(|fill, now| {
            fill.planned(epoch, pictures, &present, listing.bytes, room, now);
        });
    }

    /// Fetches one picture into the cache.
    fn fetch(&self, job: &Job) {
        let courier = &self.courier;
        let answer =
            artreader::fill_one(&job.url, &self.base, &self.cache, &|| courier.giving_way());
        courier.finished(job, answer);
    }
}

/// The pool the fill covers: every card this build plays and has tested
/// (`Coverage::Implemented`), at the printing a row that picks nothing shows.
pub(crate) fn pool() -> Vec<Source<'static>> {
    baylee_cards::all()
        .filter(|card| card.is_implemented())
        .map(|card| Source {
            name: card.name(),
            scryfall_id: card.scryfall_id,
            back: baylee_cards::sides::has_back_image(card.index),
            basic: card.faces.first().is_some_and(|face| {
                face.supertypes.contains(SupertypeSet::BASIC) && face.types.contains(TypeSet::LAND)
            }),
        })
        .collect()
}

/// What a cache directory holds.
#[derive(Debug, Default)]
pub(crate) struct Listing {
    /// The file names with a picture in them. An empty file is none: the
    /// reader treats it as missing too.
    pub(crate) names: HashSet<String>,
    /// How many bytes it holds.
    pub(crate) bytes: u64,
}

/// Lists the cache, and clears away what a crash left behind and what the old
/// key filed, which nothing reads any more and which would only eat the budget
/// (`artreader::key`). A directory that is not there yet is an empty one; one
/// that cannot be read comes out the same, and costs the fill nothing but a
/// look at each picture.
pub(crate) fn list(dir: &Path) -> Listing {
    let mut listing = Listing::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return listing;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_part(&name) {
            let old = meta
                .modified()
                .ok()
                .and_then(|at| now.duration_since(at).ok())
                .is_some_and(|age| age > STALE_PART);
            if old {
                let _ = std::fs::remove_file(entry.path());
            } else {
                listing.bytes += meta.len();
            }
            continue;
        }
        if is_old_key(&name) {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        listing.bytes += meta.len();
        if meta.len() > 0 {
            listing.names.insert(name);
        }
    }
    listing
}

/// Empties the cache: the pictures and any `.part`, nothing else that might
/// have come to lie there.
fn wipe(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let picture = name.strip_suffix(".jpg").is_some_and(is_old_key);
        if (picture || is_old_key(&name) || is_part(&name))
            && let Err(err) = std::fs::remove_file(entry.path())
            && err.kind() != io::ErrorKind::NotFound
        {
            bevy::log::warn!("cannot remove {}: {err}", entry.path().display());
        }
    }
}

/// Whether a name in the cache is a write in flight, or what a crash left of
/// one (`artreader::store`).
fn is_part(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| ext == "part")
}

/// Whether a name is a bare hash: what a picture was filed under before the
/// key stopped depending on the host.
fn is_old_key(name: &str) -> bool {
    name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// How much room the disk under `dir` has left, where the system says.
fn room(dir: &Path) -> Option<u64> {
    let there = dir.ancestors().find(|path| path.exists())?;
    let there = there.canonicalize().ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    disks
        .iter()
        .filter(|disk| there.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(sysinfo::Disk::available_space)
}

#[cfg(test)]
mod tests;
