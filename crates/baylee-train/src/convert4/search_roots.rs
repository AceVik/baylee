//! The roots a search-advantage conversion labels (`search_advantage_v1`,
//! `convert4 --search-advantage`), asked of the replay by the record input
//! that answered each: at such a root the replay keeps, beside the frame, the
//! root's payable menu (the frame's own menu and, after it, the spells the
//! planner pays for, as the student's second pass reads it) with what a
//! converter needs to label it and write its twin.
//!
//! The menu is built by the caller ([`Roots4::menu`]): the payable spells are
//! the planner's (`netplay4::menu::PayableMenu4`), which a conversion without
//! a net does not build.

use std::collections::{BTreeMap, BTreeSet};

use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use baylee_engine::choice::PlayerAction;

use crate::features4::EncodedCandidates4;

/// A candidate a search tried at a root, as its event names it: the answer
/// at the root and, for a planned cast, the cast its taps pay for, or the
/// target it was tried at.
#[derive(Clone, Debug, PartialEq)]
pub struct AfterCandidate4 {
    /// The answer at the root (a planned cast's first tap).
    pub action: PlayerAction,
    /// The cast a planned candidate's taps pay for; `None` for a single answer.
    pub script: Option<PlayerAction>,
    /// The target the search tried the answer at, the answer to the target
    /// question that follows it; `None` where it tried none (the next
    /// question, if it is one, was then the rollout's).
    pub target: Option<PlayerAction>,
}

/// Builds a root's payable menu: `own` (the frame's menu) extended, with the
/// index of the first added item; `encode` encodes another question as the
/// frame's menu is encoded.
pub type PayableMenuFn4 = fn(
    view: &PlayerView,
    pending: &Pending,
    own: &EncodedCandidates4,
    encode: &dyn Fn(&Pending) -> EncodedCandidates4,
) -> (EncodedCandidates4, usize);

/// The roots of one game a conversion asks the replay for.
pub struct Roots4 {
    /// Each root's seat and the record input that answered it (`n`).
    pub roots: BTreeSet<(u8, u64)>,
    /// How a root's payable menu is built.
    pub menu: PayableMenuFn4,
    /// The roots whose afterstates are captured (`search_afterstate_v1`), by
    /// seat and record input, with the candidates their search tried, in its
    /// order; empty for none (`super::afterstates`).
    pub afterstates: BTreeMap<(u8, u64), Vec<AfterCandidate4>>,
}

impl Roots4 {
    /// Whether the input `n` of `seat` answers a root asked for.
    #[must_use]
    pub fn asks(&self, seat: u8, n: u64) -> bool {
        self.roots.contains(&(seat, n))
    }
}

/// What the replay keeps at a root: the record input it answers, its
/// question, and its payable menu with the first added item and every
/// item's coarse class (`coarse_class_v1`).
#[derive(Clone, Debug)]
pub struct SearchRoot4 {
    /// The record input (`n`) that answered the root.
    pub record_input_n: u64,
    /// The root's question.
    pub pending: Pending,
    /// The root's payable menu.
    pub menu: EncodedCandidates4,
    /// The index of the first item the payable menu added.
    pub first_added: usize,
    /// Each item's coarse class, in menu order.
    pub classes: Vec<u8>,
    /// The root's afterstates, where [`Roots4::afterstates`] asks for them.
    #[cfg(feature = "onnx")]
    pub afterstates: Option<super::afterstates::RootAfterstates4>,
}
