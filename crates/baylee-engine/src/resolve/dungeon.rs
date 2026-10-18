//! Dungeon progression and room effects.

#[allow(clippy::wildcard_imports)] // the parent interpreter vocabulary
use super::*;

static DUNGEON_TREASURE: baylee_cards_dsl::TokenDef = baylee_cards_dsl::TokenDef {
    name: "Treasure",
    types: baylee_core::types::TypeSet::ARTIFACT,
    subtypes: &[baylee_core::generated::subtypes::artifact::TREASURE],
    abilities: &[baylee_cards_dsl::mana_ability!(
        baylee_cards_dsl::cost!(TapSelf, SacrificeSelf),
        &[Effect::mana_choice(&[
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue,
            baylee_core::mana::ManaColor::Black,
            baylee_core::mana::ManaColor::Red,
            baylee_core::mana::ManaColor::Green,
        ])]
    )],
    scryfall_id: "0559f9f3-eff0-465d-93c1-e875a8afe87f",
    ..baylee_cards_dsl::TokenDef::DEFAULT
};

use crate::choice::{CastModeDesc, CastModeKind};
use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier, TargetReq};
use baylee_core::dungeon::Dungeon;
use baylee_core::ids::AbilityRef;

mod play;
mod rooms;
mod tax;
pub use play::Play;
pub use tax::Tax;

pub(super) fn venture_into_the_dungeon(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let player = res.controller;
    if state.has_left(player) {
        return None;
    }
    let seat = usize::from(player.get());
    if let Some((dungeon, _)) = state.players[seat].dungeon
        && dungeon
            .next(state.players[seat].dungeon_room - 1)
            .is_empty()
    {
        complete(state, player);
    }
    let current = state.players[seat].dungeon;
    let choices: Vec<_> = current.map_or_else(
        || Dungeon::ALL.into_iter().map(|d| (d, 0)).collect(),
        |(d, _)| {
            d.next(state.players[seat].dungeon_room - 1)
                .iter()
                .map(|r| (d, *r))
                .collect()
        },
    );
    if let [(dungeon, room)] = choices.as_slice() {
        enter(state, res, *dungeon, *room);
        return None;
    }
    let options = choices
        .iter()
        .enumerate()
        .map(|(index, &(dungeon, room))| CastModeDesc {
            index: u32::try_from(index).expect("three dungeons"),
            kind: if current.is_none() {
                CastModeKind::Dungeon(dungeon)
            } else {
                CastModeKind::DungeonRoom { dungeon, room }
            },
            cost: baylee_core::mana::ManaCost::ZERO,
        })
        .collect();
    res.awaiting = Some(AwaitingOp::Dungeon { choices });
    Some(Pending::ChooseCastMode {
        player,
        object: res.source,
        options,
    })
}

pub(super) fn resume(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    let Some(AwaitingOp::Dungeon { choices }) = res.awaiting.take() else {
        unreachable!("dungeon choice")
    };
    let (dungeon, room) = choices[index];
    enter(state, res, dungeon, room);
    res.pc += 1;
    run(state, res)
}

fn enter(state: &mut GameState, res: &Resolution, dungeon: Dungeon, room: u8) {
    let player = res.controller;
    let seat = usize::from(player.get());
    let source = if let Some((_, object)) = state.players[seat].dungeon {
        object
    } else {
        let name = state.names.intern(dungeon.name());
        let object = state.create_bare(
            player,
            ObjectKind::Emblem,
            name,
            ZoneLocation::Command(player),
        );
        state
            .object_mut(object)
            .expect("new dungeon")
            .base_mut()
            .types = baylee_core::types::TypeSet::DUNGEON;
        state.players[seat].dungeon = Some((dungeon, object));
        object
    };
    state.players[seat].dungeon_room = room + 1;
    let (effects, target) = rooms::effects(dungeon, room);
    state.reflexive.push(crate::trigger::PendingTrigger {
        source,
        controller: player,
        timestamp: state.object(source).map_or(0, |o| o.timestamp),
        source_version: state.object(source).map(|o| o.version),
        related_objects: None,
        event_subject: None,
        event_ability: None,
        event_object_identity: None,
        counter_source_version: None,
        event_mana: None,
        event_mana_value: None,
        event_departure: None,
        event_amount: None,
        event_player: None,
        ability_index: AbilityRef::SYNTHETIC,
        abilities: None,
        text: res.text,
        event_object: None,
        implicit_target: None,
        synthetic_effects: Some(effects),
        synthetic_target: target.map(TargetReq::one),
        once_per_turn: false,
        chosen_mode: None,
    });
}

/// A last room finishes only once no room ability remains queued or on the stack.
/// Venturing from the last room can also finish it while its ability still waits.
pub(crate) fn complete(state: &mut GameState, player: PlayerId) {
    let seat = usize::from(player.get());
    let Some((_, object)) = state.players[seat].dungeon.take() else {
        return;
    };
    state.players[seat].dungeon_room = 0;
    state.players[seat].dungeons_completed =
        state.players[seat].dungeons_completed.saturating_add(1);
    state.zones.remove(object, ZoneLocation::Command(player));
    // Keep an immutable source for abilities already on the stack.
    state.remember_damage_source(object);
    let _ = state.arena.remove(object);
}

pub(crate) use play::continue_play;
pub(super) use play::{draw_three, exile_two};
pub(super) use tax::resume as resume_tax;
pub(super) use tax::start as tax;

pub(super) use tax::sacrifice_three;

#[cfg(test)]
mod tests;
