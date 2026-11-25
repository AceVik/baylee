//! Ensure Astra weight attaches to a representable, nontrivial policy action.
use crate::features4::{EncodedCandidates4, HeadKind4, TeacherTarget4, decode_action4};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

pub(in crate::convert4) fn verified_direct_target(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    target: &TeacherTarget4,
    action: &PlayerAction,
) -> bool {
    let has_choice = has_policy_choice(cands, target);
    let decoded = decode_action4(cands, view, pending, target);
    let equivalent = decoded.as_ref() == Some(action);
    if !has_choice || !equivalent {
        eprintln!(
            "[convert4] Astra direct target excluded at seat={} seq={} head={:?}: choice={has_choice} executed={action:?} decoded={decoded:?}",
            view.seat.get(),
            view.seq,
            cands.head,
        );
    }
    has_choice && equivalent
}

fn has_policy_choice(cands: &EncodedCandidates4, target: &TeacherTarget4) -> bool {
    match cands.head {
        HeadKind4::SingleChoice => {
            cands.candidates.len() > 1 && usize::from(target.single_idx) < cands.candidates.len()
        }
        HeadKind4::SubsetPick => valid_mask(cands.candidates.len(), target),
        HeadKind4::PairPick => valid_mask(cands.pairs.len(), target),
        HeadKind4::ScalarValue => {
            cands.min_choices < cands.max_choices && (0.0..=1.0).contains(&target.scalar_norm)
        }
    }
}

fn valid_mask(len: usize, target: &TeacherTarget4) -> bool {
    target.multi_mask.len() == len
        && target.free_mask.len() == len
        && target.free_mask.contains(&1)
        && target
            .multi_mask
            .iter()
            .chain(&target.free_mask)
            .all(|bit| *bit <= 1)
        && target
            .pick_sequence
            .iter()
            .all(|index| usize::from(*index) < len)
}
