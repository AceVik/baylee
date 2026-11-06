//! Legal fallback choices when a loop guard or failed inference needs one.
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{ArrangePrompt, ChoicePrompt, Pending, PlayerAction, timeout_answer};
use baylee_view::PlayerView;

use crate::features4::EncodedCandidates4;

/// Band legality also depends on banding/bands-with-other membership, which
/// the generic cardinality-only subset sampler does not represent. Declining
/// to form a band is always legal for this optional engine question. Keep this
/// outside neural sampling: it has no behavior probability or policy target.
///
/// The order of triggers (CR 603.3b) is the same kind of question: a seat of
/// the net at a table is asked it, the net was never trained on it, and the
/// order offered is the one every training game played. So it keeps that
/// order and plays as it did.
pub(super) fn unsupported_choice_answer(
    pending: &Pending,
    temperature: f32,
) -> Option<super::Answer4> {
    let action = match pending {
        Pending::ChooseCards {
            prompt: ChoicePrompt::Band { .. },
            min: 0,
            ..
        } => PlayerAction::ChooseObjects {
            objects: Vec::new(),
        },
        Pending::Arrange {
            prompt: ArrangePrompt::TriggerOrder,
            ..
        } => timeout_answer(pending)?,
        _ => return None,
    };
    Some(super::Answer4 {
        action,
        value: 0.5,
        provenance: super::StepProvenance4 {
            source: super::SourceKind4::House,
            temperature,
            thinking_steps: 0,
            behavior_logprob: 0.0,
            has_logprob: false,
        },
        target: super::TeacherTarget4::default(),
    })
}

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
