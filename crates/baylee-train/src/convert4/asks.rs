//! What a conversion asks of a replay beside its frames.

use super::search_roots::Roots4;

/// The asks of one game's replay; the default asks nothing.
#[derive(Clone, Copy, Default)]
pub struct Asks4<'a> {
    /// The roots whose payable menu the replay keeps beside their frame
    /// (`search_advantage_v1`).
    pub roots: Option<&'a Roots4>,
    /// Whether the game is an on-policy collection's, every one of whose
    /// draws the policy loss may read: the replay then refuses it at the
    /// first stochastic answer of a net to a menu that certifies
    /// (`supports_behavior_probability4`) whose sidecar has no probability.
    /// Such an answer would leave the loss for a reason that may depend on
    /// the answer drawn, which no later check can see.
    pub certified: bool,
}

/// The sampler draws above this temperature and takes the best answer, or the
/// house's mask, at or below it (`netplay4`'s `valid_raw_temperature`).
pub(super) const STOCHASTIC_ABOVE: f32 = 0.35;
