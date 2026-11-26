//! Resolution handlers for Wave 18b Bookkeeping & special search primitives:
//! - [`Effect::CreepingDread`]
//! - [`Effect::Flickerform`] and [`Effect::FlickerformReturnFromRelated`]
//! - [`Effect::PhyrexianGrimoire`]
//! - [`Effect::TaintedPact`]
//! - [`Effect::SearchLibraryAttachedToEvent`]

use baylee_cards_dsl::{Filter, PlayerRel};
use baylee_core::generated::subtypes;
use baylee_core::ids::{DamageSourceRef, NameRef, ObjectId, PlayerId, TargetRef};
use baylee_core::types::TypeSet;

use super::awaiting::AwaitingOp;
use super::remembered::PAPER_CARD_TYPES;
use super::{
    Resolution, attached_host, matches_in_resolution, players_of, source_version, subjects, wave15b,
};
use crate::choice::{ChoicePrompt, Pending};
use crate::event::Cause;
use crate::object::ObjectKind;
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use crate::zone::{Zone, ZoneLocation, ZonePosition};

/// Start `Creeping Dread` upkeep trigger resolution.
pub(super) fn exec_creeping_dread(
    state: &mut GameState,
    res: &mut Resolution,
    damage: u16,
) -> Option<Pending> {
    let mut remaining_players = players_of(PlayerRel::EachPlayer, state, res.controller, res);
    remaining_players.sort_by_key(|p| crate::battle::sba::apnap_distance(state, *p));
    remaining_players.reverse();
    advance_creeping_dread(state, res, remaining_players, Vec::new(), damage)
}

/// Ask the next player in APNAP order to choose a card to discard, or finish life loss.
pub(super) fn advance_creeping_dread(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining_players: Vec<PlayerId>,
    discarded: Vec<(PlayerId, TypeSet)>,
    damage: u16,
) -> Option<Pending> {
    while let Some(player) = remaining_players.pop() {
        let options: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
        if options.is_empty() {
            continue;
        }
        res.awaiting = Some(AwaitingOp::CreepingDread {
            remaining_players,
            discarded,
            damage,
        });
        return Some(Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Discard,
            total: None,
        });
    }
    let your_types = discarded
        .iter()
        .find(|&&(p, _)| p == res.controller)
        .map_or(TypeSet::EMPTY, |&(_, types)| types);
    if your_types != TypeSet::EMPTY && damage > 0 {
        for (player, types) in discarded {
            if state.is_opponent(player, res.controller)
                && types.intersection(your_types) != TypeSet::EMPTY
            {
                state.change_life(player, -i32::from(damage), Cause::Effect);
            }
        }
    }
    None
}

/// Resume `Creeping Dread` after one player discards a chosen card.
pub(super) fn resume_creeping_dread(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining_players: Vec<PlayerId>,
    mut discarded: Vec<(PlayerId, TypeSet)>,
    damage: u16,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(&card) = chosen.first()
        && let Some(obj) = state.object(card)
        && obj.zone == Zone::Hand
    {
        let player = obj.owner;
        if let Some(types) = state.discard_card(card, player, Cause::Effect, None) {
            discarded.push((player, types.intersection(PAPER_CARD_TYPES)));
        }
    }
    advance_creeping_dread(
        state,
        res,
        std::mem::take(&mut remaining_players),
        discarded,
        damage,
    )
}

/// Exile the enchanted creature and all Auras attached to it, scheduling end-step return.
pub(super) fn exec_flickerform(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let (host_id, host_ver) = attached_host(state, res)?;
    let host_obj = state.object(host_id)?;
    if host_obj.zone != Zone::Battlefield || host_obj.version != host_ver {
        return None;
    }
    let host_owner = host_obj.owner;
    let auras: Vec<(ObjectId, PlayerId)> = state
        .battlefield_seen()
        .filter_map(|id| {
            let obj = state.object(id)?;
            (id != host_id
                && obj.attached_to == Some(host_id)
                && obj
                    .characteristics()
                    .subtypes
                    .contains(subtypes::enchantment::AURA))
            .then_some((id, obj.owner))
        })
        .collect();

    if state
        .move_object(
            host_id,
            ZoneLocation::Exile(host_owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .is_err()
    {
        return None;
    }
    let host_exile_ver = state.object(host_id)?.version;
    let host_ref = DamageSourceRef {
        object: host_id,
        version: host_exile_ver,
    };

    let mut aura_refs = Vec::with_capacity(auras.len());
    for (aura_id, aura_owner) in auras {
        if state
            .move_object(
                aura_id,
                ZoneLocation::Exile(aura_owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .is_ok()
            && let Some(obj) = state.object(aura_id)
        {
            aura_refs.push(DamageSourceRef {
                object: aura_id,
                version: obj.version,
            });
        }
    }

    let src_version = source_version(state, res)
        .or_else(|| state.object(res.source).map(|o| o.version))
        .unwrap_or(0);
    state.delayed.push(DelayedTrigger {
        controller: res.controller,
        when: DelayedWhen::NextEndStep,
        action: DelayedAction::FlickerformReturn {
            source: DamageSourceRef {
                object: res.source,
                version: src_version,
            },
            host: host_ref,
            auras: aura_refs,
        },
    });
    None
}

/// Delayed trigger body for [`DelayedAction::FlickerformReturn`]:
/// return the creature card, and if it enters, return the exiled Auras attached to it.
pub(super) fn exec_flickerform_return(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let related: Vec<DamageSourceRef> = state
        .object(res.on_stack)
        .into_iter()
        .flat_map(|object| object.riders.iter())
        .filter_map(|rider| match rider {
            crate::object::Rider::DelayedReturnObject(reference) => Some(*reference),
            _ => None,
        })
        .collect();
    let &host = related.first()?;
    let host_owner = state
        .object(host.object)
        .filter(|o| o.zone == Zone::Exile && o.version == host.version)?
        .owner;
    if state.has_left(host_owner) {
        return None;
    }
    if let Some(obj) = state.object_mut(host.object) {
        obj.set_controller(host_owner);
        obj.kind = ObjectKind::Permanent;
    }
    if state
        .move_object(
            host.object,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .is_err()
    {
        return None;
    }
    let new_host = state
        .source_identity(host.object)
        .filter(|r| subjects::on_battlefield(state, *r))?;

    for &aura in &related[1..] {
        let Some(aura_owner) = state
            .object(aura.object)
            .filter(|o| o.zone == Zone::Exile && o.version == aura.version)
            .map(|o| o.owner)
        else {
            continue;
        };
        if state.has_left(aura_owner) {
            continue;
        }
        let _ = wave15b::enter_card_attached(
            state,
            aura,
            aura_owner,
            true,
            false,
            Some(TargetRef::Object(new_host)),
        );
    }
    None
}

/// Resolve *Phyrexian Grimoire*: target opponent chooses one of the top two cards
/// of your graveyard to exile; put the other into your hand.
pub(super) fn exec_phyrexian_grimoire(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
) -> Option<Pending> {
    let you = res.controller;
    let chooser_id = players_of(chooser, state, you, res).first().copied()?;
    let top2: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Graveyard(you))
        .iter()
        .rev()
        .take(2)
        .copied()
        .collect();
    match top2.as_slice() {
        [] => None,
        &[only] => {
            if let Some(owner) = state.object(only).map(|o| o.owner) {
                let _ = state.move_object(
                    only,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::PhyrexianGrimoire {
                cards: top2.clone(),
            });
            Some(Pending::ChooseCards {
                player: chooser_id,
                options: top2,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
    }
}

/// Resume *Phyrexian Grimoire* after the opponent picks which card to exile.
pub(super) fn resume_phyrexian_grimoire(
    state: &mut GameState,
    cards: Vec<ObjectId>,
    chosen: &[ObjectId],
) {
    let Some(&to_exile) = chosen.first().filter(|id| cards.contains(id)) else {
        return;
    };
    if let Some(owner) = state
        .object(to_exile)
        .filter(|o| o.zone == Zone::Graveyard)
        .map(|o| o.owner)
    {
        let _ = state.move_object(
            to_exile,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    for other in cards {
        if other != to_exile
            && let Some(owner) = state
                .object(other)
                .filter(|o| o.zone == Zone::Graveyard)
                .map(|o| o.owner)
        {
            let _ = state.move_object(
                other,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }
}

/// Step *Tainted Pact*: exile top card; stop if duplicate name, else offer to put in hand.
pub(super) fn step_tainted_pact(
    state: &mut GameState,
    res: &mut Resolution,
    mut seen_names: Vec<NameRef>,
) -> Option<Pending> {
    let you = res.controller;
    let &top = state.zones.list(ZoneLocation::Library(you)).last()?;
    let owner = state.object(top).map_or(you, |o| o.owner);
    if state
        .move_object(
            top,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .is_err()
    {
        return None;
    }
    let name = state.object(top)?.characteristics().name;
    if seen_names.contains(&name) {
        return None;
    }
    seen_names.push(name);
    res.awaiting = Some(AwaitingOp::TaintedPact {
        card: top,
        seen_names,
    });
    Some(Pending::ChooseCards {
        player: you,
        options: vec![top],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

/// Resume *Tainted Pact* after choosing whether to take the exiled card into hand.
pub(super) fn resume_tainted_pact(
    state: &mut GameState,
    res: &mut Resolution,
    card: ObjectId,
    seen_names: Vec<NameRef>,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if chosen.contains(&card) {
        if let Some(owner) = state
            .object(card)
            .filter(|o| o.zone == Zone::Exile)
            .map(|o| o.owner)
        {
            let _ = state.move_object(
                card,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        return None;
    }
    step_tainted_pact(state, res, seen_names)
}

/// Resolve [`Effect::SearchLibraryAttachedToEvent`] (*Sovereigns of Lost Alara*).
pub(super) fn exec_search_library_attached_to_event(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    optional: bool,
) -> Option<Pending> {
    let you = res.controller;
    if !super::search::allowed(state, res, you, you) {
        return None;
    }
    let host_ref = subjects::event(state, res).filter(|r| subjects::on_battlefield(state, *r))?;
    let lib: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(you)).clone();
    let mut options = Vec::new();
    for id in lib {
        let filter_ok = state
            .object(id)
            .is_some_and(|obj| matches_in_resolution(filter, state, obj, you, res, None));
        if filter_ok && wave15b::card_can_attach_as(state, id, you, TargetRef::Object(host_ref)) {
            options.push(id);
        }
    }
    if options.is_empty() {
        super::search::finish_shuffle(state, res, you);
        return None;
    }
    let min = u8::from(!optional && matches!(filter, Filter::Any));
    res.awaiting = Some(AwaitingOp::SearchLibraryAttachedToEvent {
        host: host_ref,
        library: you,
    });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min,
        max: 1,
        prompt: ChoicePrompt::SearchLibrary,
        total: None,
    })
}

/// Resume [`AwaitingOp::SearchLibraryAttachedToEvent`] after the player picks an Aura (or none).
pub(super) fn resume_search_library_attached_to_event(
    state: &mut GameState,
    res: &mut Resolution,
    host: DamageSourceRef,
    library: PlayerId,
    chosen: &[ObjectId],
) {
    let you = res.controller;
    if let Some(&card) = chosen.first()
        && let Some(card_ref) = state
            .object(card)
            .filter(|o| o.zone == Zone::Library)
            .map(|o| DamageSourceRef {
                object: card,
                version: o.version,
            })
        && subjects::on_battlefield(state, host)
    {
        let _ = wave15b::enter_card_attached(
            state,
            card_ref,
            you,
            true,
            false,
            Some(TargetRef::Object(host)),
        );
    }
    super::search::finish_shuffle(state, res, library);
}
