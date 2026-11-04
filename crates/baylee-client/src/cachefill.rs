//! The card cache's passive fill, on this machine: the threads, the disk and
//! what a draw tells it. What it does and why is
//! `baylee_client_core::cachefill`; what a player sees of it is
//! `lobby::card_fill`.
//!
//! The lobby tells this module what it wants ([`steer`]) and reads how it goes
//! ([`read`]), a few times a second at most and through one short lock. The
//! work happens on the workers' own threads: nothing here is on the frame
//! path, and a picture it fetches goes to the cache file and no further. No
//! bytes are decoded and no texture is made until a draw asks for the card.
//!
//! A browser has no disk cache of ours and a phone is not offered the fill, so
//! on those every entry point here is a no-op.

#[cfg(not(target_arch = "wasm32"))]
use baylee_client_core::cachefill::Fill;
use baylee_client_core::cachefill::{Progress, Steering};

#[cfg(not(target_arch = "wasm32"))]
pub mod courier;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub(crate) mod worker;

/// Starts the workers. Only a player's client calls this
/// (`standalone::run`), so no test and no bench fetches anything; and never a
/// driven build, because a harness's run has to stay measurable, and an agent
/// driving a client must not start bulk traffic to anybody.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub fn start() {
    if !cfg!(feature = "dev-control") {
        worker::start(courier::shared(), crate::artreader::art_cache());
    }
}

/// See the other arm.
#[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
pub const fn start() {}

/// Tells the fill what the client wants now.
#[cfg(not(target_arch = "wasm32"))]
pub fn steer(steering: Steering) {
    // A driven client has no workers ([`start`]): left to say it is checking
    // for ever, it would stand the header's mark in every photograph. Nor do
    // the lobby's tests, whose apps would steer one process-wide fill at once.
    if !cfg!(any(test, feature = "dev-control")) {
        courier::shared().with(|fill, now| fill.steer(steering, now));
    }
}

/// See the other arm.
#[cfg(target_arch = "wasm32")]
pub fn steer(_steering: Steering) {}

/// How the fill stands.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn read() -> Progress {
    courier::shared().with(Fill::progress)
}

/// See the other arm.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn read() -> Progress {
    Progress::default()
}

/// The player said to try again after a stop.
pub fn retry() {
    #[cfg(not(target_arch = "wasm32"))]
    courier::shared().with(Fill::retry);
}

/// The player asked for the cache to be emptied.
pub fn empty() {
    #[cfg(not(target_arch = "wasm32"))]
    courier::shared().with(|fill, _| fill.empty());
}
