//! Mutating spells merge without entering, preserving the existing permanent.
use super::{AwaitingOp, Flow, Pending, Resolution};
use crate::choice::{CastModeDesc, CastModeKind};
use crate::event::{Cause, GameEvent};
use crate::object::{ObjectKind, Rider};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::mana::ManaCost;

pub(super) fn start(res: &mut Resolution) -> Pending {
    res.awaiting = Some(AwaitingOp::MutatePosition);
    Pending::ChooseCastMode {
        player: res.controller,
        object: res.source,
        options: [true, false]
            .into_iter()
            .enumerate()
            .map(|(index, top)| CastModeDesc {
                index: u32::try_from(index).expect("two positions"),
                kind: CastModeKind::MutatePosition(top),
                cost: ManaCost::ZERO,
            })
            .collect(),
    }
}

pub(super) fn resume(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    res.awaiting = None;
    let spell = res.on_stack;
    let host = res.targets[0];
    let spell_object = state.object(spell).expect("resolving mutate spell");
    let spell_base = std::sync::Arc::clone(&spell_object.base);
    let spell_token = !spell_object.is_card();
    let owner = spell_object.owner;
    let spell_abilities = state.printed_ability_list(spell).expect("spell abilities");
    let host_abilities = state.printed_ability_list(host).expect("host abilities");
    let combined = if index == 0 {
        spell_abilities.merged_with(&host_abilities)
    } else {
        host_abilities.merged_with(&spell_abilities)
    };
    let host_object = state.object_mut(host).expect("mutate target");
    if host_object.original_base.is_none() {
        host_object.original_base = Some(std::sync::Arc::clone(&host_object.base));
    }
    let keywords = host_object.base.keywords.union(spell_base.keywords);
    if index == 0 {
        host_object.base = spell_base;
        host_object
            .riders
            .retain(|r| !matches!(r, Rider::MergedTop(_) | Rider::MergedToken(_)));
        host_object.riders.push(Rider::MergedTop(spell));
        host_object.riders.push(Rider::MergedToken(spell_token));
        host_object.status.remove(crate::object::Status::FACE_DOWN);
    }
    host_object.base_mut().keywords = keywords;
    host_object.take_abilities(combined);
    host_object.riders.push(Rider::MergedComponent(spell));
    host_object.cache.clear();
    let spell_object = state.object_mut(spell).expect("resolving spell");
    spell_object.kind = ObjectKind::Card;
    spell_object.riders.retain(|r| *r != Rider::Mutating);
    let _ = state.move_object(
        spell,
        ZoneLocation::Command(owner),
        ZonePosition::Top,
        Cause::Spell,
    );
    state
        .object_mut(spell)
        .expect("merged component")
        .riders
        .push(Rider::MergedInto(host));
    state.invalidate_projections();
    state
        .journal
        .record(GameEvent::CreatureMutated { object: host });
    res.pc += 1;
    super::run(state, res)
}
