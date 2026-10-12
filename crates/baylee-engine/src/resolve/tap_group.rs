//! Simultaneous optional nonsymbol tap payments, with exact offered identities.
use super::{AwaitingOp, Flow, Pending, Resolution};
use crate::{
    choice::ChoicePrompt,
    engine::cost_wizard,
    event::{Cause, GameEvent},
    state::GameState,
};
use baylee_cards_dsl::{CostPart, Effect};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

/// Exact offered objects and the effect contingent on their simultaneous tap.
#[derive(Clone, Debug, Hash)]
pub struct Selection {
    pub(super) player: PlayerId,
    pub(super) part: &'static CostPart,
    pub(super) offered: Vec<DamageSourceRef>,
    pub(super) on_payment: bool,
    pub(super) effect: &'static Effect,
}

pub(super) fn offer(
    state: &GameState,
    res: &mut Resolution,
    player: PlayerId,
    part: &'static CostPart,
    on_payment: bool,
    effect: &'static Effect,
) -> Option<Pending> {
    let CostPart::TapN { count, .. } = part else {
        unreachable!()
    };
    let options = cost_wizard::options_with_context(state, player, res.rule_context(), part);
    if options.len() < usize::try_from(*count).unwrap_or(usize::MAX) {
        return None;
    }
    let offered = options
        .iter()
        .filter_map(|id| state.source_identity(*id))
        .collect();
    res.awaiting = Some(AwaitingOp::TapGroup(Box::new(Selection {
        player,
        part,
        offered,
        on_payment,
        effect,
    })));
    Some(Pending::ChooseCards {
        player,
        options,
        min: 0,
        max: u8::try_from(*count).unwrap_or(u8::MAX),
        prompt: ChoicePrompt::CostTapExactlyOrNone { count: *count },
        total: None,
    })
}

pub(crate) fn valid_answer(state: &GameState, res: &Resolution, chosen: &[ObjectId]) -> bool {
    let Some(AwaitingOp::TapGroup(selection)) = &res.awaiting else {
        return true;
    };
    if chosen.is_empty() {
        return true;
    }
    let CostPart::TapN { count, .. } = selection.part else {
        unreachable!()
    };
    let options = cost_wizard::options_with_context(
        state,
        selection.player,
        res.rule_context(),
        selection.part,
    );
    chosen.len() == usize::try_from(*count).unwrap_or(usize::MAX)
        && chosen.iter().enumerate().all(|(i, id)| {
            !chosen[..i].contains(id)
                && options.contains(id)
                && state
                    .source_identity(*id)
                    .is_some_and(|reference| selection.offered.contains(&reference))
        })
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    selection: &Selection,
    chosen: &[ObjectId],
) -> Flow {
    let paid = !chosen.is_empty();
    if paid {
        for &id in chosen {
            state.set_tapped(id, true);
        }
        for &id in chosen {
            state.journal.record(GameEvent::ObjectTapped {
                object: id,
                cause: Cause::Cost,
            });
        }
    }
    if paid == selection.on_payment {
        return super::run_fallback(state, res, std::slice::from_ref(selection.effect));
    }
    res.pc += 1;
    super::run(state, res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_continuation_hash_retains_exact_offered_incarnations() {
        static PART: CostPart = CostPart::TapN {
            count: 5,
            filter: &baylee_cards_dsl::Filter::Any,
        };
        let selection = Selection {
            player: PlayerId::new(0),
            part: &PART,
            offered: vec![DamageSourceRef {
                object: ObjectId::new(1, 0),
                version: 4,
            }],
            on_payment: true,
            effect: &Effect::TransformSource,
        };
        let original = AwaitingOp::TapGroup(Box::new(selection.clone())).search_fingerprint();
        let mut changed = selection.clone();
        changed.offered[0].version += 1;
        assert_ne!(
            original,
            AwaitingOp::TapGroup(Box::new(changed)).search_fingerprint()
        );
        let mut declined_branch = selection;
        declined_branch.on_payment = false;
        assert_ne!(
            original,
            AwaitingOp::TapGroup(Box::new(declined_branch)).search_fingerprint()
        );
    }
}
