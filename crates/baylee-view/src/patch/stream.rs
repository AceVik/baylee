//! One seat's stream of views: a snapshot, then what changed.
//!
//! A host keeps a [`ViewSender`] per seat and whoever reads the seat keeps a
//! [`ViewReader`]; everything between them is bytes, and everything above them
//! holds a whole [`PlayerView`]. The rule the two keep between them is short: a
//! reader that cannot place a frame holds the view it had and asks for a
//! snapshot, once, and until one arrives it passes over what comes. A gap, a
//! wrong digest and bytes that do not read all end there, so nothing a socket
//! delivers can leave a reader on a wrong table or take it down.

use serde::{Deserialize, Serialize};

use super::{PatchError, ViewPatch, apply, diff};
use crate::player_view::PlayerView;
use baylee_core::ids::PlayerId;
use baylee_core::wire::{self, Unreadable};

/// What a frame's view bytes hold.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ViewFrame {
    /// The whole view: on joining, on coming back, and when a reader asks.
    Snapshot(Box<PlayerView>),
    /// What changed since the view the reader holds.
    Patch(Box<ViewPatch>),
}

/// [`ViewFrame`] as it is written, so that a host need not own what it sends.
/// The variants and their order are [`ViewFrame`]'s, and a test holds them to it.
#[derive(Serialize)]
enum Written<'a> {
    Snapshot(&'a PlayerView),
    Patch(&'a ViewPatch),
}

impl Written<'_> {
    fn encode(&self) -> Vec<u8> {
        wire::encode(self)
    }
}

impl ViewFrame {
    /// Reads a frame's view bytes.
    ///
    /// # Errors
    /// When they are not a view frame.
    pub fn decode(bytes: &[u8]) -> Result<Self, Unreadable> {
        wire::decode(bytes)
    }

    /// Whose view this is.
    #[must_use]
    pub fn seat(&self) -> PlayerId {
        match self {
            Self::Snapshot(view) => view.seat,
            Self::Patch(patch) => patch.seat,
        }
    }

    /// `view` whole, as a frame's bytes: for what is told once and keeps no
    /// stream, such as a replay's moment.
    #[must_use]
    pub fn snapshot_of(view: &PlayerView) -> Vec<u8> {
        Written::Snapshot(view).encode()
    }

    /// The whole view these bytes hold; `None` for anything else.
    #[must_use]
    pub fn whole(bytes: &[u8]) -> Option<PlayerView> {
        match Self::decode(bytes) {
            Ok(Self::Snapshot(view)) => Some(*view),
            _ => None,
        }
    }
}

/// What a host remembers of one seat: the last view it told it. One seat's and
/// no other's, because a patch is that seat's cards ([`super`], "Hidden
/// information"): it refuses to patch across seats and goes whole instead.
#[derive(Clone, Debug, Default)]
pub struct ViewSender {
    last: Option<Box<PlayerView>>,
    /// Every frame goes whole ([`Self::tell_whole`]).
    whole: bool,
}

impl ViewSender {
    /// From here on every frame carries the whole view: for what keeps no
    /// stream, a replay whose moments are read in any order.
    pub const fn tell_whole(&mut self) {
        self.whole = true;
    }

    /// The frame that tells the seat `view`: a patch on what it was last told,
    /// or the whole view when it holds none.
    ///
    /// A debug build proves every patch, whole or not, by applying it and
    /// comparing what it left with `view`.
    pub fn next(&mut self, view: &PlayerView) -> Vec<u8> {
        let patch = self
            .last
            .as_ref()
            .filter(|last| last.seat == view.seat && (cfg!(debug_assertions) || !self.whole))
            .map(|last| {
                let patch = diff(last, view);
                debug_assert_eq!(
                    apply(last, &patch).as_ref(),
                    Ok(view),
                    "a patch leaves its view"
                );
                patch
            });
        let frame = match patch.filter(|_| !self.whole) {
            Some(patch) => Written::Patch(&patch).encode(),
            None => Written::Snapshot(view).encode(),
        };
        self.last = Some(Box::new(view.clone()));
        frame
    }

    /// The frame that tells the seat `view` whole, whatever it holds.
    pub fn snapshot(&mut self, view: &PlayerView) -> Vec<u8> {
        self.last = Some(Box::new(view.clone()));
        Written::Snapshot(view).encode()
    }

    /// The seat holds nothing any more: a new socket. Its next frame is whole.
    pub fn forget(&mut self) {
        self.last = None;
    }
}

/// Why a [`ViewReader`] holds no current view: it has to ask for a snapshot.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ViewLost {
    /// The bytes did not read.
    Unreadable(Unreadable),
    /// A patch came with no view to apply it to.
    NoBase,
    /// A patch did not apply.
    Patch(PatchError),
}

impl std::fmt::Display for ViewLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(why) => why.fmt(f),
            Self::NoBase => f.write_str("a patch came before any view"),
            Self::Patch(why) => why.fmt(f),
        }
    }
}

impl std::error::Error for ViewLost {}

/// What a reader of one seat holds: the view its frames left.
#[derive(Clone, Debug, Default)]
pub struct ViewReader {
    view: Option<PlayerView>,
    /// A snapshot has been asked for and has not come.
    waiting: bool,
}

impl ViewReader {
    /// Reads a frame's view bytes, and says what the seat's view is now.
    ///
    /// `Ok(Some)` is the view after this frame. Empty bytes change nothing: a
    /// frame that carries more of a long log and no view. `Ok(None)` is a patch
    /// passed over while a snapshot is on its way, and the view held stays.
    ///
    /// # Errors
    /// [`ViewLost`]: the caller asks for a snapshot, once. Until one arrives
    /// this answers `Ok(None)`, so a burst of patches costs one question.
    pub fn read(&mut self, bytes: &[u8]) -> Result<Option<&PlayerView>, ViewLost> {
        if bytes.is_empty() {
            return Ok(self.view.as_ref());
        }
        let frame = match ViewFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(why) => return self.lost(ViewLost::Unreadable(why)),
        };
        match frame {
            ViewFrame::Snapshot(view) => {
                self.waiting = false;
                self.view = Some(*view);
            }
            ViewFrame::Patch(_) if self.waiting => return Ok(None),
            ViewFrame::Patch(patch) => {
                let Some(held) = &self.view else {
                    return self.lost(ViewLost::NoBase);
                };
                match apply(held, &patch) {
                    Ok(view) => self.view = Some(view),
                    Err(why) => return self.lost(ViewLost::Patch(why)),
                }
            }
        }
        Ok(self.view.as_ref())
    }

    /// The view the last frame left.
    #[must_use]
    pub const fn view(&self) -> Option<&PlayerView> {
        self.view.as_ref()
    }

    fn lost(&mut self, why: ViewLost) -> Result<Option<&PlayerView>, ViewLost> {
        if self.waiting {
            return Ok(None);
        }
        self.waiting = true;
        Err(why)
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
