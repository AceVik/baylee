//! Opt-in payment context for `BayleeAI` v5. Encoder 7 remains byte-compatible.
//!
//! Tensor widths and candidate sampling are unchanged. Entity feature 33 now
//! identifies the announced spell in every visible zone, including hand, and
//! survives nested mana choices. Global 29 still means cancellation permission;
//! global 36 identifies an announcement. Globals 37–39 carry the engine's exact
//! scalar prompt bounds and an explicit, stable number-prompt code. They do not
//! claim that every engine-legal X can be paid with the current mana sources.

use baylee_engine::choice::{NumberPrompt, Pending};
use baylee_view::PlayerView;

use crate::features4::{EncodedState4, SeatKnowledge4, pack_object_id};

/// Versioned observation contract; independent of neural model version 5.
pub const ENCODER_VERSION8: u32 = 8;

/// Encodes only facts available in the authorized view and pending question.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "Counts enter the existing f32 neural ABI"
)]
pub fn encode_state8(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> EncodedState4 {
    let mut state = crate::features7::encode_state7(view, pending, knowledge, teams);
    let source = view.announced_cast.or(view.casting);
    // Clear the old public-only marker before marking the actual source. The
    // cancellation boolean remains separate in global[29].
    for row in &mut state.entity_feat {
        row[33] = 0.0;
    }
    if let Some(index) = source.and_then(|id| state.find_entity(id)) {
        state.entity_feat[index][33] = 1.0;
    }
    state.global_feat[36] = f32::from(u8::from(source.is_some()));
    if let Some(Pending::ChooseNumber {
        min, max, reason, ..
    }) = pending
    {
        state.global_feat[37] = *min as f32;
        state.global_feat[38] = *max as f32;
        state.global_feat[39] = f32::from(number_prompt_code(reason));
    }
    state
}

// Explicit discriminants, never Rust enum layout or debug text. Exhaustive
// matching requires a conscious contract decision for future prompt variants.
const fn number_prompt_code(reason: &NumberPrompt) -> u8 {
    match reason {
        NumberPrompt::X => 1,
        NumberPrompt::Replicate { .. } => 2,
        NumberPrompt::ManaPayment { .. } => 3,
        NumberPrompt::EnergyPayment { .. } => 4,
        NumberPrompt::Counters { .. } => 5,
        NumberPrompt::DivideDamage { .. } => 6,
        NumberPrompt::CombatDamage { .. } => 7,
        NumberPrompt::Sacrifices => 8,
        NumberPrompt::Choice => 9,
        NumberPrompt::TextReplacement { .. } => 10,
    }
}

fn extend_with_source(mut hash: u64, view: &PlayerView) -> u64 {
    // Domain separate from Encoder 7, including when no spell is announced.
    let source = view.announced_cast.or(view.casting);
    for byte in [8, u8::from(source.is_some())]
        .into_iter()
        .chain(source.map_or(0, pack_object_id).to_le_bytes())
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Passive observation identity including owner-visible announcement context.
#[must_use]
pub fn board_observation_hash8(view: &PlayerView) -> u64 {
    extend_with_source(crate::features4::board_observation_hash(view), view)
}

/// Decision loop identity, retaining the exact legacy observable/pending fields.
#[must_use]
pub fn view_only_state_hash8(view: &PlayerView, pending: &Pending) -> u64 {
    extend_with_source(crate::features4::view_only_state_hash(view, pending), view)
}

#[cfg(test)]
mod tests;
