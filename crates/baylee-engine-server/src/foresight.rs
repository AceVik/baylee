//! The house's search at a hosted table, and what the operator bounds it by
//! (`docs/house-ai.md` §"Foresight at a host", `docs/dev/environment.md`).
//!
//! Every level a player picks by name has a foresight tier and, from
//! generation 9, a bound on the work of one searched answer
//! (`AIProfile::foresight`). A game's preset comes from the gateway, which
//! builds each house chair from the level's name (`AIProfile::named`); the
//! protobuf the dev harness reads carries no tier at all
//! (`baylee_gamehost::preset`). What this engine adds is the host's share:
//!
//! - `BAYLEE_FORESIGHT` (`on`, the default, or `off`): whether the house
//!   searches here at all.
//! - `BAYLEE_FORESIGHT_WORK` (percent, `1..=100`, default 100): the share of
//!   each level's bound one search may spend. Below 100 the levels play
//!   weaker than they were measured; never above.
//! - The ceiling, [`CEILING`]: no search here spends more than `expert`'s
//!   bound, whatever generation a preset names.
//! - `BAYLEE_FORESIGHT_SEARCHES` (`1..=256`, default half this machine's
//!   cores, at least one): searches at once among every engine of the agent
//!   that started this one. A search that finds every slot taken is not made;
//!   the level answers as its heuristics do. Nothing waits for a slot.
//! - `BAYLEE_FORESIGHT_WALL_MS` (`100..=60000`, default [`WALL_MS`]): a search
//!   still running after this much wall time is given up and the level's own
//!   answer stands; the engine says so in its log.
//! - `BAYLEE_FORESIGHT_SLOTS`: the directory the agent made for the slots
//!   (`slot-<n>` files, one lock each). An engine the agent did not start
//!   with one searches nothing.
//!
//! One engine plays one game and searches one answer at a time, so the
//! machine-wide count is the bound that matters: [`Slots`] holds it across
//! the agent's engines with a lock on a file (`flock(2)`), which the kernel
//! lets go when a process ends however it ends.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use baylee_core::preset::AIProfile;
use baylee_gamehost::{HostForesight, SearchPermit, SearchPermits, WallGuard};

/// The most work any search may spend at a host: `expert`'s own bound. A
/// level from before generation 9 bounds nothing itself, and no preset a
/// gateway builds names one, but the engine does not take that on trust.
pub const CEILING: u64 = AIProfile::EXPERT_WORK;

/// The wall guard's default: a searched answer that has not finished in this
/// many milliseconds is given up. `expert`'s slowest measured search took
/// 1.3 s of CPU time (`docs/proposals/house-ai-strength.md` §7.10); this
/// leaves a slower core or a busy one more than twice that.
pub const WALL_MS: u32 = 3_000;

/// The most slots an operator may set.
pub const MOST_SEARCHES: usize = 256;

/// The operator's settings for the house's search on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// `BAYLEE_FORESIGHT`: whether the house searches at all.
    pub on: bool,
    /// `BAYLEE_FORESIGHT_WORK`: the share of each level's bound, in percent.
    pub percent: u8,
    /// `BAYLEE_FORESIGHT_SEARCHES`: searches at once on this machine.
    pub searches: usize,
    /// `BAYLEE_FORESIGHT_WALL_MS`.
    pub wall_ms: u32,
    /// `BAYLEE_FORESIGHT_SLOTS`: where the slots are; `None` searches nothing.
    pub slots: Option<PathBuf>,
}

impl Settings {
    /// Reads the settings from this process's environment.
    ///
    /// # Errors
    /// A value outside its range, which stops the launch: a setting that
    /// meant to bound the house must not silently bound it otherwise.
    pub fn from_env() -> Result<Self, String> {
        Self::read(|name| std::env::var_os(name), default_searches())
    }

    /// The reading itself, with the environment and the machine's default
    /// count handed in, so that a test reads it without touching the
    /// process's environment.
    ///
    /// # Errors
    /// As [`Self::from_env`].
    pub fn read(
        env: impl Fn(&str) -> Option<std::ffi::OsString>,
        default_searches: usize,
    ) -> Result<Self, String> {
        let text = |name: &str| -> Result<Option<String>, String> {
            env(name)
                .map(|value| {
                    value
                        .into_string()
                        .map_err(|_| format!("{name} is not UTF-8"))
                })
                .transpose()
        };
        let number = |name: &str, low: u64, high: u64| -> Result<Option<u64>, String> {
            let Some(value) = text(name)? else {
                return Ok(None);
            };
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!(
                    "{name} must be a whole number from {low} through {high}"
                ));
            }
            value
                .parse::<u64>()
                .ok()
                .filter(|n| (low..=high).contains(n))
                .map(Some)
                .ok_or_else(|| format!("{name} must be a whole number from {low} through {high}"))
        };
        let on = match text("BAYLEE_FORESIGHT")?.as_deref() {
            None | Some("on") => true,
            Some("off") => false,
            Some(_) => return Err("BAYLEE_FORESIGHT must be on or off".to_string()),
        };
        let percent = number("BAYLEE_FORESIGHT_WORK", 1, 100)?.map_or(100, |n| n as u8);
        let searches = number("BAYLEE_FORESIGHT_SEARCHES", 1, MOST_SEARCHES as u64)?
            .map_or(default_searches.clamp(1, MOST_SEARCHES), |n| n as usize);
        let wall_ms =
            number("BAYLEE_FORESIGHT_WALL_MS", 100, 60_000)?.map_or(WALL_MS, |n| n as u32);
        let slots = match env("BAYLEE_FORESIGHT_SLOTS") {
            None => None,
            Some(path) if path.is_empty() => None,
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.is_absolute() {
                    return Err("BAYLEE_FORESIGHT_SLOTS must be an absolute path".to_string());
                }
                Some(path)
            }
        };
        Ok(Self {
            on,
            percent,
            searches,
            wall_ms,
            slots,
        })
    }

    /// What a table of this engine lets its house's searches spend, or `None`
    /// where the house does not search: switched off, or no slots to hold
    /// the machine's count by.
    ///
    /// # Errors
    /// A slots directory that others may write to: anyone who can write
    /// there can take the slots or put something in their place.
    pub fn host(&self) -> Result<Option<HostForesight>, String> {
        if !self.on {
            return Ok(None);
        }
        let Some(dir) = &self.slots else {
            return Ok(None);
        };
        let slots = Slots::new(dir, self.searches)?;
        Ok(Some(HostForesight {
            percent: self.percent,
            ceiling: CEILING,
            wall: Some(WallGuard {
                clock: monotonic,
                limit: Duration::from_millis(u64::from(self.wall_ms)),
            }),
            permits: Some(std::sync::Arc::new(slots)),
            yields: true,
        }))
    }
}

/// Half this machine's cores as the process may use them (its CPU quota
/// included), at least one: the other half is left to the gateway, the
/// database and every table's own heuristics.
#[must_use]
pub fn default_searches() -> usize {
    std::thread::available_parallelism().map_or(1, |cores| (cores.get() / 2).max(1))
}

/// The time since this process first asked: the wall guard's clock.
#[must_use]
pub fn monotonic() -> Duration {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed()
}

/// The machine's search slots: `count` files in a directory the agent made,
/// each held by at most one search at a time (`flock(2)` through
/// [`File::try_lock`]).
///
/// A file is opened for each attempt and kept open while its lock is held,
/// so a slot is given back when the search ends or its process does. A slot
/// file that was cleared away is made again; a directory that is gone gives
/// no slot, and the house answers without searching.
#[derive(Debug)]
pub struct Slots {
    dir: PathBuf,
    count: usize,
    /// Whether a failure to open a slot was logged already.
    said: AtomicBool,
}

impl Slots {
    /// The slots in `dir`, `count` of them.
    ///
    /// # Errors
    /// `dir` is not a directory, or others than its owner may write in it.
    pub fn new(dir: &Path, count: usize) -> Result<Self, String> {
        let meta = std::fs::metadata(dir)
            .map_err(|e| format!("BAYLEE_FORESIGHT_SLOTS {}: {e}", dir.display()))?;
        if !meta.is_dir() {
            return Err(format!(
                "BAYLEE_FORESIGHT_SLOTS {} is not a directory",
                dir.display()
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if meta.permissions().mode() & 0o022 != 0 {
                return Err(format!(
                    "BAYLEE_FORESIGHT_SLOTS {} may be written by others than its owner",
                    dir.display()
                ));
            }
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            count,
            said: AtomicBool::new(false),
        })
    }

    fn open(&self, slot: usize) -> std::io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.dir.join(format!("slot-{slot}")))
    }
}

impl SearchPermits for Slots {
    fn try_acquire(&self) -> Option<SearchPermit> {
        for slot in 0..self.count {
            let file = match self.open(slot) {
                Ok(file) => file,
                Err(e) => {
                    if !self.said.swap(true, Ordering::Relaxed) {
                        tracing::warn!(
                            dir = %self.dir.display(),
                            %e,
                            "a search slot cannot be opened; the house answers without searching"
                        );
                    }
                    return None;
                }
            };
            if file.try_lock().is_ok() {
                return Some(SearchPermit::new(file));
            }
        }
        None
    }
}

/// A directory for search slots as the agent makes one: only its owner may
/// enter it, whatever the umask.
#[cfg(test)]
pub(crate) fn slots_dir() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    builder
        .permissions(<std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700));
    builder.tempdir().expect("a slots directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    fn read(pairs: &[(&str, &str)]) -> Result<Settings, String> {
        let env: BTreeMap<String, OsString> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), OsString::from(value)))
            .collect();
        Settings::read(|name| env.get(name).cloned(), 3)
    }

    /// Nothing set: on, each level's whole bound, the machine's default count,
    /// the default guard, and no slots, so no search until the agent gives
    /// some.
    #[test]
    fn the_defaults_search_within_each_levels_bound() {
        let settings = read(&[]).expect("no settings is a setting");
        assert_eq!(
            settings,
            Settings {
                on: true,
                percent: 100,
                searches: 3,
                wall_ms: WALL_MS,
                slots: None,
            }
        );
        assert!(
            settings.host().expect("no slots is no error").is_none(),
            "no slots, no search"
        );
        assert!(default_searches() >= 1);
    }

    /// Every setting in its range is taken as written.
    #[test]
    fn settings_in_range_are_taken() {
        let dir = std::env::temp_dir();
        let dir = dir.to_str().expect("a UTF-8 temp dir");
        let settings = read(&[
            ("BAYLEE_FORESIGHT", "off"),
            ("BAYLEE_FORESIGHT_WORK", "50"),
            ("BAYLEE_FORESIGHT_SEARCHES", "256"),
            ("BAYLEE_FORESIGHT_WALL_MS", "100"),
            ("BAYLEE_FORESIGHT_SLOTS", dir),
        ])
        .expect("in range");
        assert_eq!(
            (
                settings.on,
                settings.percent,
                settings.searches,
                settings.wall_ms
            ),
            (false, 50, 256, 100)
        );
        assert!(settings.host().expect("off is no error").is_none(), "off");
    }

    /// A value outside its range or not a number stops the launch rather
    /// than bounding the house otherwise than the operator meant.
    #[test]
    fn a_setting_out_of_range_stops_the_launch() {
        for (name, value) in [
            ("BAYLEE_FORESIGHT", "yes"),
            ("BAYLEE_FORESIGHT", "ON"),
            ("BAYLEE_FORESIGHT", ""),
            ("BAYLEE_FORESIGHT_WORK", "0"),
            ("BAYLEE_FORESIGHT_WORK", "101"),
            ("BAYLEE_FORESIGHT_WORK", "-1"),
            ("BAYLEE_FORESIGHT_WORK", "50%"),
            ("BAYLEE_FORESIGHT_WORK", ""),
            ("BAYLEE_FORESIGHT_SEARCHES", "0"),
            ("BAYLEE_FORESIGHT_SEARCHES", "257"),
            ("BAYLEE_FORESIGHT_SEARCHES", "99999999999999999999999"),
            ("BAYLEE_FORESIGHT_WALL_MS", "99"),
            ("BAYLEE_FORESIGHT_WALL_MS", "60001"),
            ("BAYLEE_FORESIGHT_WALL_MS", " 3000"),
            ("BAYLEE_FORESIGHT_SLOTS", "relative/dir"),
        ] {
            assert!(
                read(&[(name, value)]).is_err(),
                "{name}={value:?} was taken"
            );
        }
    }

    /// A slots directory others may write to is refused; the agent makes one
    /// only its own user may enter.
    #[cfg(unix)]
    #[test]
    fn a_slots_directory_others_may_write_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o777))
            .expect("chmod");
        assert!(Slots::new(dir.path(), 1).is_err());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod");
        assert!(Slots::new(dir.path(), 1).is_ok());
        let file = dir.path().join("file");
        std::fs::write(&file, b"").expect("a file");
        assert!(Slots::new(&file, 1).is_err(), "not a directory");
    }

    /// Two engines' slots in one directory: no more searches at once than
    /// there are slots, each given back when it is dropped, and a slot file
    /// cleared away is made again.
    #[test]
    fn slots_hold_the_count_across_holders() {
        let dir = slots_dir();
        let one = Slots::new(dir.path(), 2).expect("slots");
        let other = Slots::new(dir.path(), 2).expect("slots");
        let first = one.try_acquire().expect("a slot");
        let second = other.try_acquire().expect("the other slot");
        assert!(one.try_acquire().is_none(), "both taken");
        assert!(other.try_acquire().is_none(), "both taken");
        drop(first);
        let third = other.try_acquire().expect("one was given back");
        drop((second, third));
        std::fs::remove_file(dir.path().join("slot-0")).expect("clear one away");
        assert!(one.try_acquire().is_some(), "made again");
        let gone = dir.path().to_path_buf();
        drop(dir);
        assert!(!gone.exists());
        assert!(one.try_acquire().is_none(), "no directory, no slot");
    }
}
