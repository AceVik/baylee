//! Plays AVIF image sequences (`avis`): the format Central stores a moving
//! profile picture in, and the format the front door's two loops ship in.
//!
//! Two halves, and only one of them trusts anything:
//!
//! | What | Where | Reads outside input |
//! | --- | --- | --- |
//! | The file's boxes and sample tables, every length checked against its parent and the [`Bounds`] before it is used | [`Sequence::read`] | yes, safe Rust only |
//! | The AV1 decoder (rav1d, the owner's choice of 12.11.2026) | [`Player`] over the private `dav1d` module | yes, through rav1d's C interface |
//! | A decoded picture's planes and their conversion to RGBA | [`Frame`] | no |
//!
//! A picture from another player reaches this crate only after Central has
//! decoded, bounded and re-encoded it itself (another player chooses pixels,
//! never the AV1 bitstream), but the client still treats the bytes as hostile:
//! a server can be wrong, and a picture crosses the network. So every size the
//! file states is held to [`Bounds`] before memory is reserved for it, the
//! decoder is told the largest frame it may make, and every decoded frame must
//! be exactly the size its track declared.
//!
//! **Not yet for pictures.** rav1d 1.1.0 does not survive every damaged
//! stream: `rav1d_submit_frame`'s error path unwraps a frame header that is
//! not there, inside an `extern "C"` entry that cannot unwind, and the
//! process aborts (found on 12.11.2026 by
//! `tests/real_file.rs::a_damaged_frame_is_an_error_and_never_a_panic`, kept
//! there as an ignored test). Nothing is corrupted, but a wrong or compromised
//! server could stop a client with one picture. So until a rav1d that returns
//! an error is pinned, or the decode runs outside the client's process,
//! [`Player`] is opened only on files the client ships
//! ([`Bounds::SHIPPED_LOOP`]). [`Bounds::PICTURE`] bounds what
//! [`Sequence::read`] accepts, which is safe Rust and may read anything.
//!
//! The crate draws nothing and does no I/O: callers hand in bytes and take
//! planes or RGBA out.

#![deny(unsafe_code)]

mod boxes;
#[cfg(not(target_arch = "wasm32"))]
mod dav1d;
mod frame;
#[cfg(not(target_arch = "wasm32"))]
mod player;
mod sequence;

pub use frame::{Frame, Matrix};
#[cfg(not(target_arch = "wasm32"))]
pub use player::Player;
pub use sequence::{Sample, Sequence, Track};

/// What a file may ask of the client before it is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The whole file.
    pub max_bytes: usize,
    /// Each side of a frame, in pixels.
    pub max_side: u32,
    /// Frames in one track.
    pub max_samples: u32,
    /// One pass through the sequence, in milliseconds.
    pub max_duration_ms: u32,
    /// When set, a frame of any other size is refused.
    pub exact_side: Option<u32>,
}

impl Bounds {
    /// A profile picture from Central, as Central bounds what it serves
    /// (Central's `docs/dependency-proposal/profile-picture.md` §11): 256 ×
    /// 256, at most 100 frames and 512 KiB. The ten seconds are Central's
    /// bound on the animations it takes in.
    pub const PICTURE: Self = Self {
        max_bytes: 512 * 1024,
        max_side: 256,
        max_samples: 100,
        max_duration_ms: 10_000,
        exact_side: Some(256),
    };

    /// A loop the client ships itself: trusted, but held to the size it was
    /// made at so a wrong file is refused rather than played at any cost.
    pub const SHIPPED_LOOP: Self = Self {
        max_bytes: 32 * 1024 * 1024,
        max_side: 3840,
        max_samples: 900,
        max_duration_ms: 30_000,
        exact_side: None,
    };

    /// The most pixels one frame may hold, which the decoder is told.
    #[must_use]
    pub fn max_pixels(&self) -> u32 {
        self.max_side.saturating_mul(self.max_side)
    }
}

/// Why a file was not played. Every refusal is final for that file: the
/// caller shows the still instead.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The file is larger than its [`Bounds`] allow.
    #[error("the file is {0} bytes, more than its bounds allow")]
    TooLarge(usize),
    /// A box's length runs past its parent, or a field past its box.
    #[error("a box is cut short: {0}")]
    Truncated(&'static str),
    /// The structure is not an AVIF image sequence this crate plays.
    #[error("not a playable image sequence: {0}")]
    Unsupported(&'static str),
    /// A stated size, count or duration is outside the [`Bounds`].
    #[error("outside the bounds: {0}")]
    OutOfBounds(&'static str),
    /// The decoder refused a sample or made a frame the track did not declare.
    #[error("the decoder failed: {0}")]
    Decode(&'static str),
}
