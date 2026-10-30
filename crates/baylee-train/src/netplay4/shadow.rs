//! Read-only snapshots for independently bound shadow labels.
use super::{
    NetPlayer4, Pending, PlayerView, encode_candidates4, encode_candidates7_grouped, encode_state4,
    encode_state7,
};
use crate::features4::{EncodedCandidates4, EncodedState4};

impl NetPlayer4 {
    /// Encodes the current decision using this player's actual seat knowledge.
    ///
    /// Call after `act_detailed` and before applying its action to the engine.
    /// This neither observes a view nor changes recurrent memory or policy RNG.
    #[must_use]
    pub fn policy_encoding(
        &self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> (EncodedState4, EncodedCandidates4) {
        let knowledge = self.knowledge.get(&view.seat.get());
        if self.is_encoder7() {
            let state = if self.uses_announcement_context() {
                crate::features9::encode_state9(view, Some(pending), knowledge, teams)
            } else if self.uses_payment_context() {
                crate::features8::encode_state8(view, Some(pending), knowledge, teams)
            } else {
                encode_state7(view, Some(pending), knowledge, teams)
            };
            let candidates = encode_candidates7_grouped(&state, view, pending);
            (state, candidates)
        } else {
            let state = encode_state4(view, Some(pending), knowledge, teams);
            let candidates = encode_candidates4(&state, view, pending);
            (state, candidates)
        }
    }
}
