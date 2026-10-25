//! Opt-in announce-before-payment candidates. Encoder 6 remains unchanged.
use std::collections::BTreeSet;
use std::fmt;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use crate::features4::{EncodedCandidates4, EncodedState4, MAX_CANDIDATES, encode_candidates4};

/// A complete Priority menu cannot fit in the candidate ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateLimit7 {
    /// Number of distinct entries that would be needed.
    pub required: usize,
}

impl fmt::Display for CandidateLimit7 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "encoder 7 needs {} candidates; limit is {MAX_CANDIDATES}",
            self.required
        )
    }
}

impl std::error::Error for CandidateLimit7 {}

/// Preserves the legacy menu and appends distinct legal unpaid spell entries.
///
/// No existing candidate feature slot is reassigned. The `view` must come from
/// a context-aware observation path so an in-progress cast offers `CancelCast`.
/// Cancellation is offered here only in Priority/payment menus. Non-Priority
/// cast questions retain their legacy heads; their legal cancellation sideband
/// still needs a separate contract rather than a fake selectable target.
/// The original question, including its payment offers, is never mutated and
/// remains the authority used to validate/decode the chosen answer.
///
/// # Errors
/// Returns [`CandidateLimit7`] rather than silently truncating a Priority menu,
/// including when the legacy portion alone would already have been truncated.
pub fn encode_candidates7(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> Result<EncodedCandidates4, CandidateLimit7> {
    let Pending::Priority { player, legal } = pending else {
        return Ok(encode_candidates4(state, view, pending));
    };
    let existing: BTreeSet<_> = legal.castable.iter().copied().collect();
    let mut seen = existing;
    let additional: Vec<_> = legal
        .payable
        .iter()
        .copied()
        .filter(|id| seen.insert(*id))
        .collect();
    let required = usize::from(legal.can_pass)
        + usize::from(view.casting.is_some())
        + legal.lands.len()
        + legal.castable.len()
        + legal.suspendable.len()
        + legal.mana_abilities.len()
        + legal.abilities.len()
        + legal.granted_actions.len()
        + additional.len();
    if required > MAX_CANDIDATES {
        return Err(CandidateLimit7 { required });
    }
    let mut out = encode_candidates4(state, view, pending);
    if additional.is_empty() {
        return Ok(out);
    }
    // Reuse the exact CastSpell feature/entity encoding without reordering the
    // original candidates. Other legal fields remain intact in this local copy.
    let mut announced = legal.clone();
    announced.castable = additional;
    let extra = encode_candidates4(
        state,
        view,
        &Pending::Priority {
            player: *player,
            legal: announced,
        },
    );
    out.candidates.extend(
        extra
            .candidates
            .into_iter()
            .filter(|candidate| matches!(candidate.action, Some(PlayerAction::CastSpell { .. }))),
    );
    Ok(out)
}

#[cfg(test)]
mod tests;
