//! Profile pictures from Baylee Central (Central's `docs/profile-pictures.md`):
//! whose keys the client asks for and when, what it keeps of a picture, and
//! which frame a picture that moves shows. Everything here is a pure
//! decision; `baylee-client` fetches, decodes and draws (`portraits`,
//! `shellkit::avatar`).
//!
//! # Keys
//!
//! An account's picture lives under a random key, new at every upload, which
//! Central names only in answer to `SocialRequest::Pictures` (at most
//! [`MAX_PICTURE_TAGS`] tags). That answer is the only authority on who wears
//! what: a tag absent from it (no picture, a block between the two, a
//! removed picture) wears its monogram, whatever the cache still holds.
//! [`KeyBook`] decides when to ask:
//!
//! - **Near tags** (the account itself, its contacts and requests, and its
//!   groups' members) cost Central nothing. They are asked when a new one
//!   appears, again every [`REFRESH_MS`], and again after a push says the
//!   contacts changed, at most every [`PUSH_SPACING_MS`].
//! - **Strangers** (a recent player, a ranking's row, anyone else whose disc
//!   is drawn) spend Central's lookup budget, all or none, which the
//!   friends drawer's "find or add" shares. They are asked apart from the
//!   near tags, so one over budget never costs a friend's picture, at most
//!   [`STRANGER_BUDGET`] an hour and each at most every
//!   [`STRANGER_REFRESH_MS`].
//! - A refusal (a Central that does not serve the request) is quiet for
//!   [`REFUSED_QUIET_MS`]; no answer backs off from [`BACKOFF_FIRST_MS`] to
//!   [`BACKOFF_MAX_MS`]. Nothing is said: a disc without a picture wears its
//!   monogram.
//!
//! # What is kept
//!
//! A picture is 256 px square as Central serves it ([`SERVED_SIDE`]); the
//! client keeps it at [`KEPT_SIDE`] ([`halve`]), which covers the largest
//! disc it draws at twice its size. The raw bytes are kept on disk by key
//! ([`file_name`], [`over_cap`]) and read back through the same bounded
//! decode; decoded frames of moving pictures are kept in memory within
//! [`CLIP_CAP_BYTES`] ([`clips_to_drop`]).
//!
//! # Motion
//!
//! A moving picture plays by its own frames' durations ([`Timing`]) only
//! while one of its discs can be seen, the window is shown and the player
//! has not asked for less motion ([`plays`]); otherwise its first frame, the
//! still, stands.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub use central_api::picture::{MAX_PICTURE_TAGS, PictureKey, TaggedPicture, key_hex, parse_key};

/// A picture's side as Central serves it, in pixels.
pub const SERVED_SIDE: u32 = 256;
/// A picture's side as the client keeps it: half of [`SERVED_SIDE`].
pub const KEPT_SIDE: u32 = SERVED_SIDE / 2;
/// The most frames a moving picture has (Central's bound).
pub const MAX_FRAMES: usize = 100;
/// The largest still (PNG) the client reads. Central's master is a 256 px
/// PNG of at most four channels, compressed: an incompressible one is a
/// little over 256 KiB.
pub const MAX_STILL_BYTES: usize = 400 * 1024;
/// A frame shorter than this plays for [`SHORT_FRAME_MS`], as browsers and
/// Central play one.
pub const SHORT_FRAME_LIMIT_MS: u32 = 20;
/// How long a frame shorter than [`SHORT_FRAME_LIMIT_MS`] plays.
pub const SHORT_FRAME_MS: u32 = 100;
/// The longest one pass of a moving picture plays (Central's bound).
pub const MAX_PASS_MS: u64 = 10_000;

/// How often the near tags are asked again.
pub const REFRESH_MS: u64 = 10 * 60_000;
/// The least time between two rounds that a push asked for.
pub const PUSH_SPACING_MS: u64 = 2 * 60_000;
/// How long a refused request keeps the client from asking again.
pub const REFUSED_QUIET_MS: u64 = 30 * 60_000;
/// The first wait after a request that got no answer; it doubles.
pub const BACKOFF_FIRST_MS: u64 = 30_000;
/// The longest wait after requests that got no answer.
pub const BACKOFF_MAX_MS: u64 = 10 * 60_000;
/// Strangers asked at most this many an hour: a sixth of Central's lookup
/// budget of 120, so "find or add" keeps the rest.
pub const STRANGER_BUDGET: usize = 20;
/// A stranger is asked again at most this often.
pub const STRANGER_REFRESH_MS: u64 = 30 * 60_000;
/// After Central said the lookup budget is spent, strangers wait this long.
pub const STRANGER_BUSY_MS: u64 = HOUR_MS;
/// One hour.
const HOUR_MS: u64 = 60 * 60_000;

/// The disk cache's bound, in bytes.
pub const DISK_CAP_BYTES: u64 = 64 * 1024 * 1024;
/// The bound on decoded frames held in memory, in bytes: at
/// [`KEPT_SIDE`] a full clip is 6.4 MiB, so at least seven full clips.
pub const CLIP_CAP_BYTES: usize = 48 * 1024 * 1024;

/// The account tag a handle (`Alice#af03`) or a bare tag (`af03`) names: the
/// game's lowercase hex spelling of Central's number, 1 to `0x7fff_ffff`.
#[must_use]
pub fn tag_of(handle: &str) -> Option<u32> {
    let tag = handle.rsplit_once('#').map_or(handle, |(_, tag)| tag);
    if !baylee_protocol::social::valid_tag(tag) {
        return None;
    }
    u32::from_str_radix(tag, 16)
        .ok()
        .filter(|tag| (1..=0x7fff_ffff).contains(tag))
}

/// A tag as a handle spells it: lowercase hex, at least four digits.
#[must_use]
pub fn tag_text(tag: u32) -> String {
    format!("{tag:04x}")
}

/// What an account wears, as Central's last answer said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Worn {
    /// The picture's key.
    pub key: PictureKey,
    /// Whether it moves (a sequence stands beside the still).
    pub moving: bool,
}

/// Which of the two requests an [`Ask`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// The account, its contacts and its groups: free.
    Near,
    /// Anyone else: on the lookup budget.
    Strangers,
}

/// One `SocialRequest::Pictures` to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    /// Which request it is.
    pub lane: Lane,
    /// The tags, at most [`MAX_PICTURE_TAGS`], ascending.
    pub tags: Vec<u32>,
}

/// How an [`Ask`] ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Central's answer: the keys of the tags that have a picture.
    Pictures(Vec<TaggedPicture>),
    /// Refused (`Rejected`): a Central that does not serve it.
    Refused,
    /// `Busy`: past a budget.
    Busy,
    /// No answer.
    Unavailable,
}

/// What an answer changed for one tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    /// The account.
    pub tag: u32,
    /// What it wore before.
    pub was: Option<Worn>,
    /// What it wears now; `None` is its monogram.
    pub now: Option<Worn>,
}

/// The ask in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flight {
    lane: Lane,
    /// Part of a full round of the near tags (not only new ones).
    round: bool,
}

/// Who wears which picture, and when to ask Central again (module docs).
#[derive(Clone, Debug, Default)]
pub struct KeyBook {
    worn: BTreeMap<u32, Worn>,
    /// Near tags asked since the book began (or was forgotten).
    near_asked: BTreeSet<u32>,
    /// When the last full round of the near tags ended.
    near_at: Option<u64>,
    /// What is left of a full round.
    round: VecDeque<u32>,
    /// A push said the contacts changed since the last round began.
    pushed: bool,
    strangers_at: BTreeMap<u32, u64>,
    /// When each stranger lookup of the last hour was spent.
    spent: VecDeque<u64>,
    strangers_quiet_until: u64,
    quiet_until: u64,
    backoff: u64,
    flight: Option<Flight>,
}

impl KeyBook {
    /// What `tag` wears, as Central last said; `None` is its monogram.
    #[must_use]
    pub fn worn(&self, tag: u32) -> Option<Worn> {
        self.worn.get(&tag).copied()
    }

    /// Every account that wears a picture.
    pub fn all(&self) -> impl Iterator<Item = (u32, Worn)> + '_ {
        self.worn.iter().map(|(tag, worn)| (*tag, *worn))
    }

    /// Whether an ask is out.
    #[must_use]
    pub fn asking(&self) -> bool {
        self.flight.is_some()
    }

    /// Until when nothing is asked (a refusal or a backoff), Unix ms.
    #[must_use]
    pub fn quiet_until(&self) -> u64 {
        self.quiet_until
    }

    /// A push (contacts, groups or presence) or a new contact snapshot said
    /// something changed: the near tags are asked again, no sooner than
    /// [`PUSH_SPACING_MS`] after the last round.
    pub fn stale(&mut self) {
        self.pushed = true;
    }

    /// Lays in what `tag` wears without asking Central: for the development
    /// harness's fixed scenarios (`/seed`), never for a picture from the
    /// network.
    pub fn wear(&mut self, tag: u32, now: Option<Worn>) -> Option<Change> {
        let was = self.worn.get(&tag).copied();
        if was == now {
            return None;
        }
        match now {
            Some(worn) => self.worn.insert(tag, worn),
            None => self.worn.remove(&tag),
        };
        Some(Change { tag, was, now })
    }

    /// Forgets everything: who wears what, what was asked, the budgets. For a
    /// sign-out, or another account.
    pub fn forget(&mut self) -> Vec<Change> {
        let lost = std::mem::take(&mut self.worn)
            .into_iter()
            .map(|(tag, worn)| Change {
                tag,
                was: Some(worn),
                now: None,
            })
            .collect();
        *self = Self::default();
        lost
    }

    /// The next request to send at `now`, if one is due: `near` holds the
    /// near tags (the account itself among them), `strangers` the other
    /// tags whose discs are drawn. One request is out at a time.
    pub fn next(
        &mut self,
        now: u64,
        near: &BTreeSet<u32>,
        strangers: &BTreeSet<u32>,
    ) -> Option<Ask> {
        if self.flight.is_some() || now < self.quiet_until {
            return None;
        }
        if self.round.is_empty() && !near.is_empty() {
            let due = self.near_at.is_none_or(|at| {
                now >= at.saturating_add(REFRESH_MS)
                    || (self.pushed && now >= at.saturating_add(PUSH_SPACING_MS))
            });
            if due {
                self.pushed = false;
                self.round = near.iter().copied().collect();
            }
        }
        if !self.round.is_empty() {
            let take = self.round.len().min(MAX_PICTURE_TAGS);
            let tags: Vec<u32> = self.round.drain(..take).collect();
            self.flight = Some(Flight {
                lane: Lane::Near,
                round: true,
            });
            return Some(Ask {
                lane: Lane::Near,
                tags,
            });
        }
        let fresh: Vec<u32> = near
            .iter()
            .copied()
            .filter(|tag| !self.near_asked.contains(tag))
            .take(MAX_PICTURE_TAGS)
            .collect();
        if !fresh.is_empty() {
            self.flight = Some(Flight {
                lane: Lane::Near,
                round: false,
            });
            return Some(Ask {
                lane: Lane::Near,
                tags: fresh,
            });
        }
        self.strangers(now, near, strangers)
    }

    fn strangers(
        &mut self,
        now: u64,
        near: &BTreeSet<u32>,
        strangers: &BTreeSet<u32>,
    ) -> Option<Ask> {
        if now < self.strangers_quiet_until {
            return None;
        }
        while self
            .spent
            .front()
            .is_some_and(|at| now >= at.saturating_add(HOUR_MS))
        {
            self.spent.pop_front();
        }
        let room = STRANGER_BUDGET
            .saturating_sub(self.spent.len())
            .min(MAX_PICTURE_TAGS);
        // Those never asked first, then those asked longest ago.
        let mut due: Vec<(u64, u32)> = strangers
            .iter()
            .copied()
            .filter(|tag| !near.contains(tag))
            .filter_map(|tag| match self.strangers_at.get(&tag) {
                None => Some((0, tag)),
                Some(&at) => (now >= at.saturating_add(STRANGER_REFRESH_MS)).then_some((at, tag)),
            })
            .collect();
        due.sort_unstable();
        let mut tags: Vec<u32> = due.into_iter().take(room).map(|(_, tag)| tag).collect();
        tags.sort_unstable();
        if tags.is_empty() {
            return None;
        }
        self.flight = Some(Flight {
            lane: Lane::Strangers,
            round: false,
        });
        Some(Ask {
            lane: Lane::Strangers,
            tags,
        })
    }

    /// Takes the end of `ask` at `now` and says what changed. Only tags that
    /// were asked are read from an answer, each at most once.
    pub fn answer(&mut self, now: u64, ask: &Ask, answer: Answer) -> Vec<Change> {
        let Some(flight) = self.flight.take() else {
            return Vec::new();
        };
        match answer {
            Answer::Pictures(pictures) => {
                self.backoff = 0;
                let mut changes = Vec::new();
                for &tag in &ask.tags {
                    let now_worn =
                        pictures
                            .iter()
                            .find(|picture| picture.tag == tag)
                            .map(|picture| Worn {
                                key: picture.key,
                                moving: picture.moving,
                            });
                    let was = self.worn.get(&tag).copied();
                    if was != now_worn {
                        match now_worn {
                            Some(worn) => self.worn.insert(tag, worn),
                            None => self.worn.remove(&tag),
                        };
                        changes.push(Change {
                            tag,
                            was,
                            now: now_worn,
                        });
                    }
                }
                match flight.lane {
                    Lane::Near => {
                        self.near_asked.extend(ask.tags.iter().copied());
                        if flight.round && self.round.is_empty() {
                            self.near_at = Some(now);
                        }
                    }
                    Lane::Strangers => {
                        for &tag in &ask.tags {
                            self.strangers_at.insert(tag, now);
                            self.spent.push_back(now);
                        }
                    }
                }
                changes
            }
            Answer::Refused => {
                self.round.clear();
                self.quiet_until = now.saturating_add(REFUSED_QUIET_MS);
                Vec::new()
            }
            Answer::Busy if flight.lane == Lane::Strangers => {
                self.strangers_quiet_until = now.saturating_add(STRANGER_BUSY_MS);
                Vec::new()
            }
            Answer::Busy | Answer::Unavailable => {
                if flight.round {
                    // The round goes on where it stopped.
                    for &tag in ask.tags.iter().rev() {
                        self.round.push_front(tag);
                    }
                }
                self.backoff = if self.backoff == 0 {
                    BACKOFF_FIRST_MS
                } else {
                    self.backoff.saturating_mul(2).min(BACKOFF_MAX_MS)
                };
                self.quiet_until = now.saturating_add(self.backoff);
                Vec::new()
            }
        }
    }
}

/// When each frame of a moving picture ends within one pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timing {
    /// The end of each frame, in milliseconds from the pass's start.
    ends: Vec<u64>,
}

impl Timing {
    /// The timing of frames shown for `durations` milliseconds each: a frame
    /// shorter than [`SHORT_FRAME_LIMIT_MS`] plays for [`SHORT_FRAME_MS`],
    /// as browsers play it. `None` for no frames, more than [`MAX_FRAMES`],
    /// or a pass longer than [`MAX_PASS_MS`] after that rule.
    #[must_use]
    pub fn new(durations: &[u32]) -> Option<Self> {
        if durations.is_empty() || durations.len() > MAX_FRAMES {
            return None;
        }
        let mut at = 0u64;
        let mut ends = Vec::with_capacity(durations.len());
        for &ms in durations {
            let ms = if ms < SHORT_FRAME_LIMIT_MS {
                SHORT_FRAME_MS
            } else {
                ms
            };
            at = at.saturating_add(u64::from(ms));
            ends.push(at);
        }
        (at <= MAX_PASS_MS).then_some(Self { ends })
    }

    /// How many frames one pass has.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.ends.len()
    }

    /// One pass, in milliseconds.
    #[must_use]
    pub fn pass_ms(&self) -> u64 {
        self.ends.last().copied().unwrap_or(0)
    }

    /// The frame shown `elapsed` milliseconds after the picture began to
    /// play, the passes repeating for ever.
    #[must_use]
    pub fn frame_at(&self, elapsed: u64) -> usize {
        let pass = self.pass_ms();
        if self.ends.len() < 2 || pass == 0 {
            return 0;
        }
        let into = elapsed % pass;
        self.ends.partition_point(|end| *end <= into)
    }
}

/// Whether a moving picture plays: one of its discs can be seen, the window
/// is shown, and the player has not asked for less motion. Otherwise its
/// first frame stands.
#[must_use]
pub const fn plays(reduce_motion: bool, seen: bool, window_shown: bool) -> bool {
    !reduce_motion && seen && window_shown
}

/// `rgba` (`side` × `side`, straight alpha, 8 bits a channel) at half its
/// side: each pixel the alpha-weighted mean of four, so a transparent
/// pixel's colour bleeds into none. `None` for an odd side or a buffer of
/// another length.
#[must_use]
pub fn halve(rgba: &[u8], side: u32) -> Option<Vec<u8>> {
    let side = usize::try_from(side).ok()?;
    if side == 0 || side % 2 != 0 || rgba.len() != side.checked_mul(side)?.checked_mul(4)? {
        return None;
    }
    let half = side / 2;
    let mut out = vec![0u8; half * half * 4];
    for y in 0..half {
        for x in 0..half {
            let mut sum = [0u32; 3];
            let mut alpha = 0u32;
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let at = ((2 * y + dy) * side + 2 * x + dx) * 4;
                let a = u32::from(rgba[at + 3]);
                for (channel, total) in sum.iter_mut().enumerate() {
                    *total += u32::from(rgba[at + channel]) * a;
                }
                alpha += a;
            }
            let to = (y * half + x) * 4;
            // A pixel no one of the four shows has no colour.
            for (channel, total) in sum.iter().enumerate() {
                out[to + channel] = (total + alpha / 2).checked_div(alpha).unwrap_or(0) as u8;
            }
            out[to + 3] = ((alpha + 2) / 4) as u8;
        }
    }
    Some(out)
}

/// One of a picture's files the client keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum File {
    /// The still, as Central's PNG master.
    Still,
    /// The AVIF image sequence of a picture that moves.
    Moving,
}

impl File {
    /// The file's ending, after the key.
    #[must_use]
    pub const fn ending(self) -> &'static str {
        match self {
            Self::Still => ".png",
            Self::Moving => ".sequence.avif",
        }
    }
}

/// The name a picture's file is kept under: its key in lowercase hex and the
/// file's ending. No tag and no name: the key is all the cache knows.
#[must_use]
pub fn file_name(key: &PictureKey, file: File) -> String {
    format!("{}{}", key_hex(key), file.ending())
}

/// The key and file a kept file's name says; `None` for any other name, which
/// is not the cache's and is left alone.
#[must_use]
pub fn parse_file_name(name: &str) -> Option<(PictureKey, File)> {
    [File::Moving, File::Still].into_iter().find_map(|file| {
        let key = parse_key(name.strip_suffix(file.ending())?)?;
        Some((key, file))
    })
}

/// A file in the disk cache: its name, size and last write (any clock, the
/// same for all).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    /// Its name.
    pub name: String,
    /// Its size in bytes.
    pub bytes: u64,
    /// When it was last written.
    pub written: u64,
}

/// Which files to remove so the cache holds at most `cap` bytes: the oldest
/// first.
#[must_use]
pub fn over_cap(files: &[Kept], cap: u64) -> Vec<String> {
    let mut total: u64 = files.iter().map(|file| file.bytes).sum();
    let mut oldest: Vec<&Kept> = files.iter().collect();
    oldest.sort_by(|a, b| a.written.cmp(&b.written).then_with(|| a.name.cmp(&b.name)));
    let mut gone = Vec::new();
    for file in oldest {
        if total <= cap {
            break;
        }
        total = total.saturating_sub(file.bytes);
        gone.push(file.name.clone());
    }
    gone
}

/// A moving picture's decoded frames, held in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    /// Which picture.
    pub key: PictureKey,
    /// The frames' bytes.
    pub bytes: usize,
    /// When one of its discs was last seen (any clock, the same for all).
    pub seen: u64,
}

/// Which clips to let go so at most `cap` bytes of frames are held: the one
/// seen longest ago first. A clip let go shows its still until it is seen
/// again and decoded anew.
#[must_use]
pub fn clips_to_drop(held: &[Held], cap: usize) -> Vec<PictureKey> {
    let mut total: usize = held.iter().map(|clip| clip.bytes).sum();
    let mut oldest: Vec<&Held> = held.iter().collect();
    oldest.sort_by(|a, b| a.seen.cmp(&b.seen).then_with(|| a.key.cmp(&b.key)));
    let mut gone = Vec::new();
    for clip in oldest {
        if total <= cap {
            break;
        }
        total = total.saturating_sub(clip.bytes);
        gone.push(clip.key);
    }
    gone
}

#[cfg(test)]
mod tests;
