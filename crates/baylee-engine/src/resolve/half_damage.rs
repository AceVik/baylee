//! "Choose a player who cast one or more sorcery spells this turn. Backdraft
//! deals damage to that player equal to half the damage dealt by one of
//! those sorcery spells this turn, rounded down." (`Effect::DealHalfOfSpellDamage`.)
//!
//! Choosing the spell names its caster, so the one question is which spell:
//! every spell the filter matches that was cast this turn, each with what it
//! dealt this turn ([`crate::state::PerTurn::damage_log`], its damage as a
//! spell on the stack). The damage itself is an ordinary `DealDamage` to the
//! chosen player, spliced in after this op, so prevention and replacement
//! see it like any other.

use baylee_cards_dsl::{Amount, Effect, Filter, PlayerRel, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::eval;
use crate::event::GameEvent;
use crate::state::GameState;
use crate::zone::Zone;

/// The spells offered, each with its caster and the damage it dealt.
#[derive(Clone, Debug, Hash)]
pub struct Spells {
    options: Vec<(ObjectId, PlayerId, u32)>,
}

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    spell: &'static Filter,
) -> Option<Pending> {
    let you = res.controller;
    let entries = state.journal.entries();
    let turn_began = entries
        .iter()
        .rposition(|e| matches!(e.event, GameEvent::TurnStarted { .. }))
        .map_or(0, |at| at + 1);
    let mut options: Vec<(ObjectId, PlayerId, u32)> = Vec::new();
    for entry in &entries[turn_began..] {
        let GameEvent::SpellCast { object, player, .. } = entry.event else {
            continue;
        };
        if options.iter().any(|(o, _, _)| *o == object)
            || !state.object(object).is_some_and(|o| {
                eval::matches_with_context(spell, state, o, you, res.rule_context())
            })
        {
            continue;
        }
        let damage = state
            .per_turn
            .damage_log
            .iter()
            .filter(|d| d.source.object == object && d.snapshot.zone == Zone::Stack)
            .map(|d| d.amount)
            .fold(0, u32::saturating_add);
        options.push((object, player, damage));
    }
    match options.as_slice() {
        [] => None,
        [only] => {
            deal(res, *only);
            None
        }
        _ => {
            let ids = options.iter().map(|(o, _, _)| *o).collect();
            res.awaiting = Some(AwaitingOp::HalfSpell(Box::new(Spells { options })));
            Some(Pending::ChooseCards {
                player: you,
                options: ids,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
    }
}

pub(super) fn resume(
    _state: &mut GameState,
    res: &mut Resolution,
    spells: &Spells,
    chosen: &[ObjectId],
) {
    if let Some(picked) = chosen
        .first()
        .and_then(|c| spells.options.iter().find(|(o, _, _)| o == c))
    {
        deal(res, *picked);
    }
}

/// Half the spell's damage, rounded down, to its caster: an ordinary damage
/// op run next, with the caster as the chosen player.
fn deal(res: &mut Resolution, (_, caster, damage): (ObjectId, PlayerId, u32)) {
    res.chosen_player = Some(caster);
    let next = res.pc + 1;
    res.effects.insert(
        next,
        Effect::DealDamage {
            amount: Amount::Fixed(damage / 2),
            target: TargetSpec::Player(PlayerRel::Chosen),
        },
    );
}
