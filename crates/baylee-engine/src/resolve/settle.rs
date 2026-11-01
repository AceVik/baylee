//! Exile a player's attackers, then let them search for one basic land each.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::zone::Zone;
use baylee_cards_dsl::TypeSet;

/// The compensation counts creatures that actually reached exile, not merely
/// the creatures selected before a replacement effect could redirect them.
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerRel,
) -> Option<Pending> {
    let player = players_of(player, state, res.controller, res)
        .first()
        .copied()?;
    let attackers: Vec<_> = state
        .battlefield_seen()
        .filter(|&id| {
            state.combat.is_attacking(id)
                && state.object(id).is_some_and(|object| {
                    object.controller == player
                        && object.characteristics().types.contains(TypeSet::CREATURE)
                })
        })
        .collect();
    let mut exiled = 0_usize;
    for attacker in attackers {
        let Some(before) = state
            .object(attacker)
            .map(|object| (object.owner, object.version))
        else {
            continue;
        };
        let _ = state.move_object(
            attacker,
            ZoneLocation::Exile(before.0),
            ZonePosition::Top,
            Cause::Effect,
        );
        if state
            .object(attacker)
            .is_some_and(|object| object.zone == Zone::Exile && object.version != before.1)
        {
            exiled += 1;
        }
    }
    let count = u8::try_from(exiled).unwrap_or(u8::MAX);
    if state.has_left(player) {
        return None;
    }
    if !search::allowed(state, res, player, player) {
        // Accepting the separate search instruction is not possible, but a
        // prohibition on searching does not prohibit the following shuffle.
        search::finish_shuffle(state, res, player);
        return None;
    }
    res.awaiting = Some(AwaitingOp::MaySearchBasicLand { player, count });
    Some(Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        source: resolving_ability(state, res),
    })
}
