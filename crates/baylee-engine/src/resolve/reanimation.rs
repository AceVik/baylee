//! Returning cards to the battlefield, including double-faced returns.
use super::zones::moving_objects;
#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::{state::CardLookup, zone::Zone};

#[allow(clippy::too_many_lines)] // one dispatch arm per return family; shared work stays in helpers
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::GraveyardToBattlefieldLinked { target } => {
            for id in moving_objects(state, res, target) {
                let Some(before) = state.source_identity(id).filter(|reference| {
                    state
                        .object(reference.object)
                        .is_some_and(|object| object.zone == Zone::Graveyard && object.is_card())
                }) else {
                    continue;
                };
                return_card(state, id, you, false, None);
                super::linked_reanimation::queue(
                    state,
                    res,
                    baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: before.version.wrapping_add(1),
                    },
                );
            }
            None
        }
        Effect::GraveyardToBattlefield {
            target,
            owner_control,
            counters,
        } => {
            for id in moving_objects(state, res, target) {
                if matches!(target, TargetSpec::EventObject)
                    && subjects::event(state, res).is_some_and(|r| !subjects::is_current(state, r))
                {
                    continue;
                }
                return_card(state, id, you, owner_control, counters);
            }
            None
        }
        Effect::GraveyardToBattlefieldModified {
            target,
            owner_control,
            tapped,
            counters,
            modifiers,
        } => {
            return_modified(
                state,
                res,
                target,
                owner_control,
                tapped,
                counters,
                modifiers,
            );
            None
        }
        Effect::Assimilate { target } => {
            use baylee_cards_dsl::{CounterKind, Modifier};
            use baylee_core::{generated::subtypes, types::TypeSet};
            const MODIFIERS: &[Modifier] = &[
                Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                Modifier::ReplaceCreatureTypes(subtypes::creature::BORG),
            ];
            return_modified(
                state,
                res,
                target,
                false,
                false,
                Some((CounterKind::P1P1, 1)),
                MODIFIERS,
            );
            None
        }
        Effect::ReturnSourceFromGraveyard {
            face,
            tapped,
            owner_control,
            counters,
        } => {
            if let Some(source) = subjects::moving_source(state, res)
                && state
                    .object(source.object)
                    .is_some_and(|o| o.zone == Zone::Graveyard)
            {
                arrive(state, source.object, you, owner_control, face, tapped);
                if let Some((kind, amount)) = counters
                    && (state.entries.contains(source.object)
                        || state
                            .object(source.object)
                            .is_some_and(|o| o.zone == Zone::Battlefield))
                {
                    crate::entry::put_entry_counters(state, source.object, kind, amount);
                }
            }
            None
        }
        Effect::ExileSelfReturnAsFace {
            face,
            owner_control,
        } => {
            let source =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))?;
            let owner = state.object(source.object).expect("current source").owner;
            let _ = state.move_object(
                source.object,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            if state
                .object(source.object)
                .is_some_and(|o| o.zone == Zone::Exile)
            {
                arrive(state, source.object, you, owner_control, Some(face), false);
            }
            None
        }
        Effect::ReturnCraftedTransformed => {
            return_crafted_transformed(state, res, you);
            None
        }
        Effect::MoveAndModify {
            subject,
            expected_origin,
            blink,
            owner_control,
            tapped,
            face,
            counters,
            modifiers,
            duration,
            attach_source,
            when_source_leaves,
        } => {
            move_and_modify(
                state,
                res,
                MoveAndModifySpec {
                    subject,
                    expected_origin,
                    blink,
                    owner_control,
                    tapped,
                    face,
                    counters,
                    modifiers,
                    duration,
                    attach_source,
                    when_source_leaves,
                },
            );
            None
        }
        _ => unreachable!("only return operations are dispatched here"),
    }
}

#[allow(clippy::struct_excessive_bools)] // Direct mirror of Effect::MoveAndModify parameters.
#[derive(Clone, Copy)]
struct MoveAndModifySpec {
    subject: baylee_cards_dsl::MoveSubject,
    expected_origin: Option<baylee_cards_dsl::ZoneRef>,
    blink: bool,
    owner_control: bool,
    tapped: bool,
    face: Option<u8>,
    counters: Option<(baylee_cards_dsl::CounterKind, baylee_cards_dsl::Amount)>,
    modifiers: &'static [baylee_cards_dsl::Modifier],
    duration: baylee_cards_dsl::Duration,
    attach_source: bool,
    when_source_leaves: Option<baylee_cards_dsl::SourceLeaveAction>,
}

fn zone_sel_candidates(
    state: &GameState,
    res: &Resolution,
    zone: baylee_cards_dsl::ZoneSel,
    filter: &baylee_cards_dsl::Filter,
) -> SmallVec<[ObjectId; 4]> {
    use baylee_cards_dsl::{PlayerRel, ZoneSel};
    let you = res.controller;
    let ctx = res.rule_context();
    if matches!(zone, ZoneSel::LinkedExile) {
        return eval::linked_exile_cards(state, you, ctx, source_version(state, res), filter)
            .into_iter()
            .collect();
    }
    let raw: Vec<ObjectId> = match zone {
        ZoneSel::Battlefield => state.battlefield_seen().collect(),
        ZoneSel::LibraryYou => state.zones.list(ZoneLocation::Library(you)).clone(),
        ZoneSel::GraveyardYou => state.zones.list(ZoneLocation::Graveyard(you)).clone(),
        ZoneSel::GraveyardOf(who) => players_of(who, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
            .collect(),
        ZoneSel::GraveyardOpponents => players_of(PlayerRel::EachOpponent, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
            .collect(),
        ZoneSel::GraveyardAll => players_of(PlayerRel::EachPlayer, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
            .collect(),
        ZoneSel::HandYou => state.zones.list(ZoneLocation::Hand(you)).clone(),
        ZoneSel::HandActivePlayer => state
            .zones
            .list(ZoneLocation::Hand(state.turn.active))
            .clone(),
        ZoneSel::HandChosen => players_of(PlayerRel::Chosen, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Hand(p)).iter().copied())
            .collect(),
        ZoneSel::ExileAll => players_of(PlayerRel::EachPlayer, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Exile(p)).iter().copied())
            .collect(),
        ZoneSel::HandOf(who) => players_of(who, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Hand(p)).iter().copied())
            .collect(),
        ZoneSel::LibraryOf(who) => players_of(who, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Library(p)).iter().copied())
            .collect(),
        ZoneSel::LinkedExile => unreachable!("handled above"),
    };
    raw.into_iter()
        .filter(|&id| {
            state
                .object(id)
                .is_some_and(|o| eval::matches_with_context(filter, state, o, you, ctx))
        })
        .collect()
}

fn prepare_source_aura(
    state: &mut GameState,
    controller: PlayerId,
    aura: baylee_core::ids::DamageSourceRef,
) {
    state.refresh_characteristics();
    let needs_aura_subtype = state.object(aura.object).is_some_and(|o| {
        let ch = o.characteristics();
        ch.types.contains(baylee_core::types::TypeSet::ENCHANTMENT)
            && !ch
                .subtypes
                .contains(baylee_core::generated::subtypes::enchantment::AURA)
            && !ch
                .subtypes
                .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
            && !ch
                .subtypes
                .contains(baylee_core::generated::subtypes::artifact::FORTIFICATION)
    });
    if needs_aura_subtype {
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(aura.object),
            controller,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Type,
            timestamp,
            duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
            filter: crate::effects::EffectFilter::ObjectIs(aura.object, aura.version),
            modifier: baylee_cards_dsl::Modifier::AddSubtype(
                baylee_core::generated::subtypes::enchantment::AURA,
            ),
        });
        state.refresh_characteristics();
    }
    if state.object(aura.object).is_some_and(|o| {
        o.characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA)
    }) && !state.reanimated_auras.iter().any(|b| b.aura == aura)
    {
        state
            .reanimated_auras
            .push(crate::aura_bindings::ReanimatedAura {
                aura,
                returned: None,
            });
    }
}

fn move_and_modify(state: &mut GameState, res: &Resolution, spec: MoveAndModifySpec) {
    let live_source = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r));
    if spec.attach_source {
        let Some(aura) = live_source else {
            return;
        };
        prepare_source_aura(state, res.controller, aura);
    }
    let counter_pair = spec.counters.and_then(|(kind, amt)| {
        let n = u16::try_from(amount2(&amt, state, res.controller, res)).unwrap_or(u16::MAX);
        (n > 0).then_some((kind, n))
    });
    let raw_candidates: SmallVec<[ObjectId; 4]> = match spec.subject {
        baylee_cards_dsl::MoveSubject::Target(target) => {
            if matches!(target, TargetSpec::EventObject)
                && subjects::event(state, res).is_some_and(|r| !subjects::is_current(state, r))
            {
                return;
            }
            moving_objects(state, res, target).into_iter().collect()
        }
        baylee_cards_dsl::MoveSubject::AllInZone { zone, filter } => {
            zone_sel_candidates(state, res, zone, filter)
        }
    };
    let arriving: SmallVec<[ObjectId; 4]> = if spec.blink {
        let mut exiled: SmallVec<[ObjectId; 4]> = SmallVec::new();
        for target_id in raw_candidates {
            if state.object(target_id).is_none_or(|o| {
                o.zone != Zone::Battlefield
                    || o.status.contains(crate::object::Status::PHASED_OUT)
                    || spec
                        .expected_origin
                        .is_some_and(|orig| !o.zone.matches_ref(orig))
            }) {
                continue;
            }
            exiled.extend(state.merged_components(target_id));
            let owner = state.object(target_id).map_or(res.controller, |o| o.owner);
            if let Some(obj) = state.object_mut(target_id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                target_id,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        exiled
            .into_iter()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| o.zone == Zone::Exile && o.is_card())
            })
            .collect()
    } else {
        raw_candidates
            .into_iter()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    o.is_card()
                        && o.zone != Zone::Battlefield
                        && spec
                            .expected_origin
                            .is_none_or(|orig| o.zone.matches_ref(orig))
                })
            })
            .collect()
    };
    for id in arriving {
        enter_with_modifiers(state, res, id, spec, counter_pair, live_source);
    }
}

fn prepare_entering_face(
    state: &mut GameState,
    id: ObjectId,
    face: Option<u8>,
    prior_face: u8,
) -> bool {
    if let Some(face) = face {
        let Some(def) = state
            .object(id)
            .and_then(|o| o.card)
            .and_then(|c| state.bases.card(c.index))
        else {
            return false;
        };
        let Some(entering) = def.faces.get(usize::from(face)) else {
            return false;
        };
        if !entering.types.is_permanent()
            || (face != 0
                && (entering.adventure
                    || def.faces[0]
                        .subtypes
                        .contains(&baylee_core::generated::subtypes::enchantment::ROOM)
                    || (entering.castable_from_hand && !def.faces[0].types.is_permanent())))
        {
            return false;
        }
        state.turn_over(id, def, usize::from(face));
    }
    if state
        .object(id)
        .is_none_or(|o| !o.characteristics().types.is_permanent())
    {
        if face.is_some()
            && let Some(def) = state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| state.bases.card(c.index))
        {
            state.turn_over(id, def, usize::from(prior_face));
        }
        return false;
    }
    true
}

fn enter_with_modifiers(
    state: &mut GameState,
    res: &Resolution,
    id: ObjectId,
    spec: MoveAndModifySpec,
    counter_pair: Option<(baylee_cards_dsl::CounterKind, u16)>,
    live_source: Option<baylee_core::ids::DamageSourceRef>,
) {
    use baylee_cards_dsl::Duration;
    use baylee_core::ids::EffectId;
    if !spec.owner_control && state.has_left(res.controller) {
        return;
    }
    let Some(before_obj) = state.object(id).filter(|o| o.is_card()) else {
        return;
    };
    let prior_zone = before_obj.zone;
    let prior_version = before_obj.version;
    let prior_face = before_obj.face_index;
    let previous = (
        before_obj.controller,
        before_obj.base_controller,
        before_obj.kind,
        before_obj.status.contains(crate::object::Status::TAPPED),
    );
    if !prepare_entering_face(state, id, spec.face, prior_face) {
        return;
    }
    let version = prior_version.wrapping_add(1);
    let needs_source = matches!(
        spec.duration,
        Duration::WhileSourceOnBattlefield
            | Duration::WhileYouControlSource
            | Duration::WhileSourceInZone(_)
            | Duration::WhileSourceTapped { .. }
    );
    let mut created = smallvec::SmallVec::<[EffectId; 4]>::new();
    if !needs_source || live_source.is_some() {
        let fx_source = needs_source.then(|| live_source.expect("checked live_source").object);
        for &modifier in spec.modifiers {
            let timestamp = state.next_timestamp();
            created.push(state.effects.register(crate::effects::ContinuousEffect {
                id: EffectId::new(0),
                source: fx_source,
                controller: res.controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: spec.duration,
                filter: crate::effects::EffectFilter::ObjectIs(id, version),
                modifier,
            }));
        }
    }
    if let Some(object) = state.object_mut(id) {
        object.kind = ObjectKind::Permanent;
        object.set_controller(if spec.owner_control {
            object.owner
        } else {
            res.controller
        });
    }
    state.set_tapped(id, spec.tapped);
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    let entered = state.object(id).is_some_and(|o| {
        (o.zone == Zone::Battlefield && o.version == version) || state.entries.contains(id)
    });
    if !entered {
        rollback_unentered_object(
            state,
            id,
            &created,
            prior_zone,
            prior_version,
            prior_face,
            spec.face,
            previous,
        );
        return;
    }
    if let Some((kind, n)) = counter_pair {
        crate::entry::put_entry_counters(state, id, kind, n);
    }
    if (spec.attach_source || spec.when_source_leaves.is_some())
        && let Some(aura) = live_source
    {
        state
            .reanimation_finishes
            .push(crate::aura_bindings::ReanimationFinish {
                aura,
                returned: baylee_core::ids::DamageSourceRef {
                    object: id,
                    version,
                },
                controller: res.controller,
                text: res.text,
                attach: spec.attach_source,
                leave_action: spec.when_source_leaves,
            });
    }
}

#[allow(clippy::too_many_arguments)]
fn rollback_unentered_object(
    state: &mut GameState,
    id: ObjectId,
    created: &[baylee_core::ids::EffectId],
    prior_zone: Zone,
    prior_version: u32,
    prior_face: u8,
    requested_face: Option<u8>,
    previous: (PlayerId, PlayerId, ObjectKind, bool),
) {
    state
        .effects
        .remove_where(|effect| created.contains(&effect.id));
    if state
        .object(id)
        .is_some_and(|o| o.zone == prior_zone && o.version == prior_version)
    {
        if requested_face.is_some()
            && let Some(def) = state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| state.bases.card(c.index))
        {
            state.turn_over(id, def, usize::from(prior_face));
        }
        state.restore_tapped(id, previous.3);
        let object = state.object_mut(id).expect("the same object");
        object.kind = previous.2;
        object.controller = previous.0;
        object.base_controller = previous.1;
    }
}

/// Returns the card exiled by `CostPart::ExileSelf` on a craft ability to the
/// battlefield transformed under its owner's control (`CR 702.167a`,
/// `CR 712.11a–b`) and links every exiled material card from
/// `CostPart::CraftMaterials` (`CR 702.167c`).
fn return_crafted_transformed(state: &mut GameState, res: &Resolution, you: PlayerId) {
    let Some(paid_source) = subjects::moving_source(state, res) else {
        return;
    };
    let id = paid_source.object;
    if !state.object(id).is_some_and(|o| {
        o.zone == Zone::Exile
            && o.version == paid_source.version
            && o.is_card()
            && !state.has_left(o.owner)
    }) {
        return;
    }
    let materials: Vec<baylee_core::ids::DamageSourceRef> =
        state.object(res.on_stack).map_or_default(|o| {
            o.riders
                .iter()
                .filter_map(|r| match r {
                    crate::object::Rider::CraftMaterial(mat) => Some(*mat),
                    _ => None,
                })
                .collect()
        });
    arrive(state, id, you, true, Some(1), false);
    let new_version = if let Some(obj) = state
        .object(id)
        .filter(|o| o.zone == Zone::Battlefield && o.version != paid_source.version)
    {
        obj.version
    } else if state.entries.contains(id) {
        paid_source.version.wrapping_add(1)
    } else {
        return;
    };
    let host_ref = baylee_core::ids::DamageSourceRef {
        object: id,
        version: new_version,
    };
    let host_ver = u16::try_from(new_version).unwrap_or(u16::MAX);
    for mat in materials {
        if let Some(mat_obj) = state
            .object_mut(mat.object)
            .filter(|o| o.zone == Zone::Exile && o.version == mat.version && o.is_card())
        {
            let with = crate::object::Rider::ExiledWith {
                host: id,
                version: host_ver,
            };
            if !mat_obj.riders.contains(&with) {
                mat_obj.riders.push(with);
            }
            let linked = crate::object::Rider::LinkedExile {
                source: host_ref,
                acquisition: 0,
            };
            if !mat_obj.riders.contains(&linked) {
                mat_obj.riders.push(linked);
            }
        }
    }
}

/// CR 611.2e applies these changes as the card enters, not afterwards.
/// A return that never happens leaves neither tapped state nor modifiers.
fn return_modified(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    owner_control: bool,
    tapped: bool,
    counters: Option<(baylee_cards_dsl::CounterKind, u16)>,
    modifiers: &'static [baylee_cards_dsl::Modifier],
) {
    use baylee_cards_dsl::Duration;
    use baylee_core::ids::EffectId;
    for id in moving_objects(state, res, target) {
        let Some(object) = state
            .object(id)
            .filter(|o| o.zone == Zone::Graveyard && o.is_card())
        else {
            continue;
        };
        let previous = (
            object.controller,
            object.base_controller,
            object.kind,
            object.status.contains(crate::object::Status::TAPPED),
        );
        let graveyard_version = object.version;
        let version = graveyard_version.wrapping_add(1);
        let mut created = smallvec::SmallVec::<[EffectId; 4]>::new();
        for &modifier in modifiers {
            let timestamp = state.next_timestamp();
            created.push(state.effects.register(crate::effects::ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: res.controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: Duration::Indefinitely,
                filter: crate::effects::EffectFilter::ObjectIs(id, version),
                modifier,
            }));
        }
        if tapped {
            state.set_tapped(id, true);
        }
        return_card(state, id, res.controller, owner_control, counters);
        if !state.object(id).is_some_and(|o| {
            (o.zone == Zone::Battlefield && o.version == version) || state.entries.contains(id)
        }) {
            state
                .effects
                .remove_where(|effect| created.contains(&effect.id));
            if state.object(id).is_some_and(|object| {
                object.zone == Zone::Graveyard && object.version == graveyard_version
            }) {
                state.restore_tapped(id, previous.3);
                let object = state.object_mut(id).expect("the same graveyard object");
                object.kind = previous.2;
                // This is a rollback, not a control change: preserve both the
                // projected controller and its previous layer-2 fallback.
                object.controller = previous.0;
                object.base_controller = previous.1;
            }
        }
    }
}

/// The shared arrival door for ordinary and characteristic-changing returns.
pub(super) fn return_card(
    state: &mut GameState,
    id: ObjectId,
    controller: PlayerId,
    owner_control: bool,
    counters: Option<(baylee_cards_dsl::CounterKind, u16)>,
) {
    if !owner_control && state.has_left(controller) {
        return;
    }
    let Some(object) = state
        .object_mut(id)
        .filter(|o| o.zone == Zone::Graveyard && o.is_card())
    else {
        return;
    };
    object.kind = ObjectKind::Permanent;
    object.set_controller(if owner_control {
        object.owner
    } else {
        controller
    });
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    if let Some((kind, n)) = counters {
        crate::entry::put_entry_counters(state, id, kind, n);
    }
}

/// Choose the entering face before any entry replacement or trigger can see it.
/// Entering transformed is not the transform action (CR 712.11a, 701.27a).
fn arrive(
    state: &mut GameState,
    id: ObjectId,
    you: PlayerId,
    owner_control: bool,
    face: Option<u8>,
    tapped: bool,
) {
    if !owner_control && state.has_left(you) {
        return;
    }
    let Some(object) = state.object(id) else {
        return;
    };
    if !object.is_card() {
        return;
    } // A token that left cannot return (CR 111.8).
    if let Some(face) = face {
        let Some(def) = object.card.and_then(|card| state.bases.card(card.index)) else {
            return;
        };
        let Some(entering) = def.faces.get(usize::from(face)) else {
            return;
        };
        if !entering.types.is_permanent()
            || (face != 0
                && (entering.adventure
                    || def.faces[0]
                        .subtypes
                        .contains(&baylee_core::generated::subtypes::enchantment::ROOM)
                    || (entering.castable_from_hand && !def.faces[0].types.is_permanent())))
        {
            return;
        }
        state.turn_over(id, def, usize::from(face));
    }
    if state
        .object(id)
        .is_none_or(|o| !o.characteristics().types.is_permanent())
    {
        return;
    }
    if let Some(object) = state.object_mut(id) {
        object.kind = ObjectKind::Permanent;
        object.set_controller(if owner_control { object.owner } else { you });
    }
    state.set_tapped(id, tapped);
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
}
