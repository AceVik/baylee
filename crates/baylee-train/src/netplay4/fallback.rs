//! Legal fallback choices when a loop guard or failed inference needs one.
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_view::PlayerView;

use crate::features4::EncodedCandidates4;

pub(super) fn fallback_action(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
) -> PlayerAction {
    for c in &cands.candidates {
        if let Some(act) = &c.action
            && pending.answer_fault(act).is_none()
        {
            return act.clone();
        }
    }
    if let Some(action) =
        timeout_answer(pending).filter(|action| pending.answer_fault(action).is_none())
    {
        return action;
    }
    // Clock defaults deliberately omit number, naming and mandatory choices.
    // A missing default does not authorize PassPriority. Use the ordinary
    // view-only house interface, with public team assignments and no scouting.
    HeuristicAgent::new(AIProfile::STEADY)
        .with_teams(teams.to_vec())
        .act(view, pending)
}
