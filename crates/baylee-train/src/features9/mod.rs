//! Encoder 9: the Encoder-8 prefix plus authorized settled announcement X.
use crate::features4::{EncodedState4, SeatKnowledge4};
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;
/// Explicit experimental observation contract. Existing defaults remain 7.
pub const ENCODER_VERSION9: u32 = 9;
/// Eighty unchanged Encoder-8 globals plus presence and selected X.
pub const GLOBAL_FEAT_WIDTH9: usize = 82;
/// Encode only the separately opted-in, authorized announcement view.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "Versioned scalar u32-to-f32 observation ABI"
)]
pub fn encode_state9(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> EncodedState4 {
    let mut state = crate::features8::encode_state8(view, pending, knowledge, teams);
    state
        .global_feat
        .push(f32::from(u8::from(view.announced_x.is_some())));
    state.global_feat.push(view.announced_x.unwrap_or(0) as f32);
    state
}
fn extend_x(mut hash: u64, view: &PlayerView) -> u64 {
    for byte in [9, u8::from(view.announced_x.is_some())]
        .into_iter()
        .chain(view.announced_x.unwrap_or(0).to_le_bytes())
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}
/// Passive identity including selected X, with an Encoder-9 domain separator.
#[must_use]
pub fn board_observation_hash9(view: &PlayerView) -> u64 {
    extend_x(crate::features8::board_observation_hash8(view), view)
}
/// Decision loop identity; no hidden engine state participates.
#[must_use]
pub fn view_only_state_hash9(view: &PlayerView, pending: &Pending) -> u64 {
    extend_x(crate::features8::view_only_state_hash8(view, pending), view)
}
#[cfg(test)]
mod tests;
