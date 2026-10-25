//! Equinox's "counter target spell if it would destroy a land you control"
//! (`Condition::TargetSpellWouldDestroy`).
//!
//! The 2013 ruling: as the ability resolves, ask whether the targeted spell
//! would destroy such a land if it resolved right then. Only what the spell
//! itself destroys counts: its own top-level `Destroy` instructions aimed at
//! its targets and its `DestroyAll` sweeps, a modal spell's in the modes it
//! was cast with. Destruction behind a choice (a
//! "may", a branch, a random pick, a choice made as it resolves) or done by
//! the game (lethal damage, a sacrifice) does not (the 2004 and 2008
//! rulings), and an indestructible permanent would not be destroyed.

use baylee_cards_dsl::{AbilityDef, Effect, Filter, TargetSpec};
use baylee_core::ids::ObjectId;

use super::Resolution;
use crate::eval;
use crate::state::GameState;

pub(super) fn target_spell_would_destroy(
    state: &GameState,
    res: &Resolution,
    filter: &'static Filter,
) -> bool {
    let Some(spell) = res
        .targets
        .first()
        .and_then(|id| state.object(*id))
        .filter(|o| o.zone == crate::zone::Zone::Stack)
    else {
        return false;
    };
    let Some(list) = state.printed_ability_list(spell.id) else {
        return false;
    };
    // A modal spell's modes were chosen as it was cast (CR 700.2a, 601.2b):
    // the effects of the chosen ones are what it would do.
    let chosen = |index: usize| {
        spell.mode_index.map(usize::from) == Some(index)
            || (index < 8 && spell.modes & (1 << index) != 0)
    };
    let effects: Vec<Effect> = list
        .abilities
        .iter()
        .flat_map(|a| match a {
            AbilityDef::Spell { effects, .. } => effects.to_vec(),
            AbilityDef::ModalSpell { modes, .. } => modes
                .iter()
                .enumerate()
                .filter(|(index, _)| chosen(*index))
                .flat_map(|(_, mode)| mode.effects.iter().copied())
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    let you = res.controller;
    let doomed = |id: ObjectId| {
        state.object(id).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && !o
                    .characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
                && eval::matches_with_context(filter, state, o, you, res.rule_context())
        })
    };
    effects.iter().any(|effect| match effect {
        Effect::DestroyTargetSlots { targets, .. } => targets.iter().any(|slot| match slot {
            baylee_cards_dsl::TargetSlot::This => doomed(spell.id),
            baylee_cards_dsl::TargetSlot::First => spell.targets.iter().copied().any(doomed),
            baylee_cards_dsl::TargetSlot::Second => {
                spell.second_targets().iter().copied().any(doomed)
            }
        }),
        Effect::Destroy {
            target:
                TargetSpec::Object(_)
                | TargetSpec::AnyTarget
                | TargetSpec::PlayerOrObject(_)
                | TargetSpec::StackOrBattlefield(_)
                | TargetSpec::OpponentOrObject(_),
            ..
        } => spell.targets.iter().copied().any(doomed),
        Effect::DestroyAll { filter: sweep, .. }
        | Effect::DestroyAllThenForEachDestroyed { filter: sweep, .. } => {
            state.battlefield_seen().any(|id| {
                doomed(id)
                    && state
                        .object(id)
                        .is_some_and(|o| eval::matches(sweep, state, o, spell.controller, spell.id))
            })
        }
        _ => false,
    })
}
