//! Independently payable life loss at the next actual draw step.
use super::{Resolution, players_of, resolving_ability, source_version};
use crate::{
    choice::{GrantedActionKind, GrantedActionOffer},
    state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState},
};
use baylee_cards_dsl::{PlayerRel, SpecialActionCost, SpecialActionTiming};
use baylee_core::{
    ids::{DamageSourceRef, GrantedActionId},
    mana::ManaCost,
};

pub(super) fn schedule(
    state: &mut GameState,
    res: &Resolution,
    player: PlayerRel,
    amount: u32,
    cost: ManaCost,
) {
    if amount > i32::MAX as u32 {
        state.numeric_failure = Some("delayed life loss exceeds i32");
        return;
    }
    let Some(version) = source_version(state, res) else {
        return;
    };
    let source = DamageSourceRef {
        object: res.source,
        version,
    };
    let ability = resolving_ability(state, res);
    for player in players_of(player, state, res.controller, res) {
        if state.has_left(player) {
            continue;
        }
        let obligation = state.grant_action(
            player,
            GrantedActionOffer {
                id: GrantedActionId::new(0),
                source,
                ability,
                timing: SpecialActionTiming::Priority,
                cost: SpecialActionCost::Mana(cost),
                effect: GrantedActionKind::CancelDelayedLifeLoss { amount },
            },
        );
        state.delayed.push(DelayedTrigger {
            controller: player,
            when: DelayedWhen::NextDraw,
            action: DelayedAction::DrawStepLifeLoss {
                obligation,
                source,
                controller: res.controller,
                amount,
            },
        });
    }
}
