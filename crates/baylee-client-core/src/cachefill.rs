//! The card cache's passive fill: pictures nobody has asked for yet, fetched
//! in the background so that the deck builder and the table find them on disk.
//!
//! Decisions only, like [`crate::images`]: what is fetched and in which order
//! ([`plan`]), when a request may start ([`Gate`]), when the fill rests, cools
//! or stops ([`Fill`]), and how that is said ([`words`]). `baylee-client` holds
//! the threads, the HTTP and the disk (`src/cachefill/`), and tells this
//! module the time, so every rule here is tested on a fake clock.
//!
//! # A picture wanted now is never behind this
//!
//! A wanted picture has nothing in front of it: it goes as it always went, and
//! only says that it is under way. It is the fill that looks: it starts
//! nothing while a wanted picture runs or for [`QUIET`] after, and a fill
//! request already running gives way between two reads of its body
//! ([`Gate::yielding`]).
//!
//! # It goes straight to Scryfall
//!
//! The fill asks `cards.scryfall.io` and `backs.scryfall.io` directly, signed
//! in or not, so none of it reaches a gateway, and the URLs come from the
//! compiled registry, so none of it reaches `api.scryfall.com`. The image hosts
//! have no rate limit (`docs/legal.md` §3): a few requests at once out of
//! courtesy ([`Pace`]), and it still honours a 429 and `Retry-After`, backs off
//! on a 5xx and stops after repeated failure ([`Backoff`]).
//!
//! # It fills the file a draw reads
//!
//! The cache names a picture by what is drawn, the path below its base
//! ([`crate::images::art_path`]), so a picture filled from the CDN is the one a
//! signed-in draw reads under the mirror's address.
//!
//! # Nothing is stored about the fill
//!
//! There is no list on disk. On every start, and whenever the reach changes,
//! the client lists the cache and hands [`Fill::planned`] what is already
//! there, so a restart resumes where the disk says it stopped.

mod choices;
mod fill;
mod gate;
mod meter;
mod plan;
mod retry;
pub mod words;

pub use choices::{Budget, FillChoices, Pace, Reach};
pub use fill::{Answer, Fill, Job, Next, Progress, Status, Steering, Stop, Trouble, Why};
pub use gate::{FILL_SLOTS, Gate, QUIET, Turn};
pub use plan::{Picture, Round, Source, plan};
pub use retry::{Backoff, MAX_FAILURES, retry_after};

/// How much room the fill leaves on the cache's disk: it stops at this, and a
/// cache that may not fill a disk has to stop well before the disk does.
pub const FREE_FLOOR: u64 = 2_000_000_000;

#[cfg(test)]
mod tests;
