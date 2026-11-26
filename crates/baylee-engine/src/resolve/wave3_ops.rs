//! Engine Wave 3 resolution operations:
//! - [`grant_protection_from_chosen_color`] (`CR 702.16`, `CR 611.2c`)
//! - [`repeat_each`] (`CR 101.4`, `CR 608.2h` re-entrant bounded iteration)
//! - [`choose_card_start`] / [`choose_card_finish`] (`CR 608.2d`)
//! - [`peek_and_reveal_start`] / [`peek_and_reveal_finish`] (`CR 701.20a–b`)
//! - [`reveal_from_hand_start`] / [`reveal_from_hand_resume`] / [`reveal_top`]

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{
    ChooseMove, ChooseZone, Duration, Filter, Layer, Modifier, PeekPickDest, PeekRestDest,
    RepeatDomain,
};
use baylee_core::ids::DamageSourceRef;

/// One remaining iteration item in a [`RepeatFrame`].
#[derive(Clone, Copy, Debug, Hash)]
pub(crate) enum RepeatStep {
    /// A player step (`RepeatDomain::EachPlayer` / `EachOpponent`, or `ChooseCard` body).
    Player(PlayerId),
    /// A snapshotted permanent step (`RepeatDomain::EachMatchingPermanent` or `ChooseCard`).
    Permanent {
        identity: DamageSourceRef,
        controller: PlayerId,
        require_battlefield: bool,
    },
    /// A snapshotted object in a specific zone (`Effect::RepeatEachObject`).
    ObjectInZone {
        identity: DamageSourceRef,
        zone: crate::zone::Zone,
        event_player: PlayerId,
    },
    /// A remembered object from `Effect::CaptureAffected` (`Effect::ForEachRemembered`).
    Remembered(super::remembered::RememberedObject),
    /// A player paired with a branch index (`Effect::PlayersChooseMode`).
    PlayerBranch {
        /// Player bound as [`PlayerRel::EventPlayer`] for this step.
        player: PlayerId,
        /// Index into `MultiPlayerChoice::branches`.
        branch_index: u8,
    },
}

/// Remaining iteration state of an active [`RepeatFrame`].
#[derive(Clone, Debug, Hash)]
pub(crate) enum RepeatSteps {
    /// Explicit queue in reverse order (`pop()` yields the next iteration).
    Items(Vec<RepeatStep>),
    /// Remaining count for `RepeatDomain::Times` (`O(1)` memory even at `REPEAT_CAP`).
    Ticks(u32),
}

/// Re-entrant loop frame stored on [`SubjectContext`] across `Pending` suspensions.
#[derive(Clone, Debug, Hash)]
pub(crate) struct RepeatFrame {
    /// Exact suffix length (`res.effects.len() - 1 - continuation_pc`) identifying
    /// this frame's spliced continuation instruction so nested `RepeatEach` loops
    /// never collide with outer frames.
    pub(crate) tail_from_end: usize,
    pub(crate) steps: RepeatSteps,
    pub(crate) saved_event_object: Option<ObjectId>,
    pub(crate) saved_event_identity: Option<(DamageSourceRef, i16)>,
    pub(crate) saved_event_player: Option<PlayerId>,
}

/// Bind `id` onto `res.event_object` and `res.subject.event_identity` (`Remembered`).
pub(super) fn bind_remembered_object(state: &GameState, res: &mut Resolution, id: ObjectId) {
    res.event_object = Some(id);
    if let Some(obj) = state.object_or_departed(id) {
        let power = obj.characteristics().power.unwrap_or(0);
        res.subject.set_event_identity(Some((
            DamageSourceRef {
                object: id,
                version: obj.version,
            },
            power,
        )));
    }
}

/// Grant protection from `res.source_chosen_color(state)` until end of turn to
/// permanents selected by `filter` (`CR 702.16`, `CR 611.2c`).
pub(super) fn grant_protection_from_chosen_color(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
) {
    let Some(color) = res.source_chosen_color(state) else {
        return;
    };
    let Some(prot_filter) = super::answers::PROTECTION_COLORS.get(color as usize) else {
        return;
    };
    let duration = Duration::UntilEndOfTurn;
    if !super::continuous_subjects::source_duration_starts(state, res, duration) {
        return;
    }
    let you = res.controller;
    let modifier = Modifier::ProtectionFrom(prot_filter);
    let filters: SmallVec<[crate::effects::EffectFilter; 4]> = match filter {
        Filter::This if res.targeted => res
            .targets
            .iter()
            .copied()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })
            .map(|id| crate::effects::EffectFilter::object(state, id))
            .collect(),
        Filter::This => super::this_to_affect(state, res)
            .map(|id| crate::effects::EffectFilter::object(state, id))
            .into_iter()
            .collect(),
        Filter::AttachedToBySource => super::attached_host(state, res)
            .map(|(host, version)| crate::effects::EffectFilter::ObjectIs(host, version))
            .into_iter()
            .collect(),
        other => {
            super::bound_now_with_context(state, other, &modifier, you, res.rule_context(), None)
        }
    };
    let timestamp = state.next_timestamp();
    for effect_filter in filters {
        if !crate::permanent_guards::permits_resolution_effect(state, effect_filter, modifier, you)
            || !state.source_duration_holds(duration, res.source, you, effect_filter)
        {
            continue;
        }
        let id = state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::Ability,
            timestamp,
            duration,
            filter: effect_filter,
            modifier,
        });
        // CR 612.1 / CR 612.3: freeze identity text so text-changing effects on
        // the spell do not rewrite a color chosen during resolution.
        state.effect_text_overrides.push((
            id,
            crate::text_changes::TextOrigin::Frozen(crate::text_changes::TextChangeMap::IDENTITY),
        ));
    }
    state.refresh_characteristics();
}

/// Entry and step-advance handler for [`Effect::RepeatEach`].
pub(super) fn repeat_each(
    state: &mut GameState,
    res: &mut Resolution,
    domain: RepeatDomain,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let resuming = res
        .subject
        .repeat_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if !resuming {
        if effects.is_empty() {
            return None;
        }
        let steps = build_domain_steps(state, res, domain);
        if matches!(&steps, RepeatSteps::Items(v) if v.is_empty())
            || matches!(&steps, RepeatSteps::Ticks(0))
        {
            return None;
        }
        res.subject.repeat_frames.push(RepeatFrame {
            tail_from_end: after_len,
            steps,
            saved_event_object: res.event_object,
            saved_event_identity: res.subject.event_identity(),
            saved_event_player: res.subject.event_player(),
        });
    }
    advance_repeat_frame(state, res, effects, op)
}

fn build_domain_steps(state: &GameState, res: &Resolution, domain: RepeatDomain) -> RepeatSteps {
    let you = res.controller;
    let cap = super::immediate::REPEAT_CAP as usize;
    match domain {
        RepeatDomain::EachPlayer | RepeatDomain::EachOpponent => {
            let rel = if matches!(domain, RepeatDomain::EachPlayer) {
                PlayerRel::EachPlayer
            } else {
                PlayerRel::EachOpponent
            };
            let mut seats = eval::players(rel, state, you).unwrap_or_default();
            seats.sort_by_key(|p| crate::battle::sba::apnap_distance(state, *p));
            seats.truncate(cap);
            seats.reverse();
            RepeatSteps::Items(seats.into_iter().map(RepeatStep::Player).collect())
        }
        RepeatDomain::EachPlayerStartingWithYou => {
            let mut seats = super::player_number_choices::turn_order_starting_with(state, you);
            seats.truncate(cap);
            seats.reverse();
            RepeatSteps::Items(seats.into_iter().map(RepeatStep::Player).collect())
        }
        RepeatDomain::EachMatchingPermanent(filter) => {
            let context = res.rule_context();
            let mut matched: Vec<(DamageSourceRef, PlayerId)> = state
                .battlefield_seen()
                .filter_map(|id| {
                    let obj = state.object(id)?;
                    eval::matches_with_context(filter, state, obj, you, context).then_some((
                        DamageSourceRef {
                            object: id,
                            version: obj.version,
                        },
                        obj.controller,
                    ))
                })
                .collect();
            matched.sort_by_key(|(_, c)| crate::battle::sba::apnap_distance(state, *c));
            matched.truncate(cap);
            matched.reverse();
            RepeatSteps::Items(
                matched
                    .into_iter()
                    .map(|(identity, controller)| RepeatStep::Permanent {
                        identity,
                        controller,
                        require_battlefield: true,
                    })
                    .collect(),
            )
        }
        RepeatDomain::Times(amount) => {
            let n = amount2(&amount, state, you, res).min(super::immediate::REPEAT_CAP);
            RepeatSteps::Ticks(n)
        }
    }
}

pub(super) fn advance_repeat_frame(
    state: &mut GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    loop {
        let next_step = {
            let frame = res.subject.repeat_frames.last_mut()?;
            match &mut frame.steps {
                RepeatSteps::Items(items) => items.pop().map(Ok),
                RepeatSteps::Ticks(0) => None,
                RepeatSteps::Ticks(n) => {
                    *n -= 1;
                    Some(Err(()))
                }
            }
        };
        let step_effects: &'static [Effect] = match next_step {
            Some(Ok(RepeatStep::Player(player))) => {
                if state.has_left(player) {
                    continue;
                }
                res.subject.set_event_player(Some(player));
                effects
            }
            Some(Ok(RepeatStep::Permanent {
                identity,
                controller,
                require_battlefield,
            })) => {
                if require_battlefield {
                    let Some(obj) = state.object(identity.object).filter(|o| {
                        o.zone == crate::zone::Zone::Battlefield && o.version == identity.version
                    }) else {
                        continue;
                    };
                    let power = obj.characteristics().power.unwrap_or(0);
                    res.event_object = Some(identity.object);
                    res.subject.set_event_identity(Some((identity, power)));
                    res.subject.set_event_player(Some(obj.controller));
                } else {
                    bind_remembered_object(state, res, identity.object);
                    res.subject.set_event_player(Some(controller));
                }
                effects
            }
            Some(Ok(RepeatStep::ObjectInZone {
                identity,
                zone,
                event_player,
            })) => {
                let Some(obj) = state
                    .object(identity.object)
                    .filter(|o| o.zone == zone && o.version == identity.version)
                else {
                    continue;
                };
                let player = if zone == crate::zone::Zone::Battlefield {
                    obj.controller
                } else {
                    event_player
                };
                bind_remembered_object(state, res, identity.object);
                res.subject.set_event_player(Some(player));
                effects
            }
            Some(Ok(RepeatStep::Remembered(rem))) => {
                let power = rem.lki.power.unwrap_or(0);
                res.event_object = Some(rem.reference.object);
                res.subject.set_event_identity(Some((rem.reference, power)));
                res.subject.set_event_player(Some(rem.lki.controller));
                effects
            }
            Some(Ok(RepeatStep::PlayerBranch {
                player,
                branch_index,
            })) => {
                if state.has_left(player) {
                    continue;
                }
                res.subject.set_event_player(Some(player));
                if let Effect::PlayersChooseMode(spec) = op {
                    spec.branches
                        .get(usize::from(branch_index))
                        .copied()
                        .unwrap_or(&[])
                } else {
                    effects
                }
            }
            Some(Err(())) => effects,
            None => {
                let frame = res.subject.repeat_frames.pop()?;
                res.event_object = frame.saved_event_object;
                res.subject.set_event_identity(frame.saved_event_identity);
                res.subject.set_event_player(frame.saved_event_player);
                return None;
            }
        };
        let mut pass = flatten(step_effects);
        pass.push(op);
        let at = res.pc + 1;
        res.effects.splice(at..at, pass);
        return None;
    }
}

/// Entry and step-advance handler for [`Effect::ForEachPlayer`].
pub(super) fn exec_for_each_player(
    state: &mut GameState,
    res: &mut Resolution,
    players: PlayerRel,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let resuming = res
        .subject
        .repeat_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if !resuming {
        if effects.is_empty() {
            return None;
        }
        let cap = super::immediate::REPEAT_CAP as usize;
        let mut seats = eval::players(players, state, res.controller).unwrap_or_default();
        seats.sort_by_key(|p| crate::battle::sba::apnap_distance(state, *p));
        seats.truncate(cap);
        seats.reverse();
        if seats.is_empty() {
            return None;
        }
        res.subject.repeat_frames.push(RepeatFrame {
            tail_from_end: after_len,
            steps: RepeatSteps::Items(seats.into_iter().map(RepeatStep::Player).collect()),
            saved_event_object: res.event_object,
            saved_event_identity: res.subject.event_identity(),
            saved_event_player: res.subject.event_player(),
        });
    }
    advance_repeat_frame(state, res, effects, op)
}

/// Start executing ordered `(player, branch_index)` steps for [`Effect::PlayersChooseMode`].
pub(super) fn exec_players_choose_branches(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
    mut steps: Vec<(PlayerId, u8)>,
) -> Option<Pending> {
    if steps.is_empty() {
        return None;
    }
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    steps.reverse();
    res.subject.repeat_frames.push(RepeatFrame {
        tail_from_end: after_len,
        steps: RepeatSteps::Items(
            steps
                .into_iter()
                .map(|(player, branch_index)| RepeatStep::PlayerBranch {
                    player,
                    branch_index,
                })
                .collect(),
        ),
        saved_event_object: res.event_object,
        saved_event_identity: res.subject.event_identity(),
        saved_event_player: res.subject.event_player(),
    });
    advance_repeat_frame(state, res, &[], op)
}

/// Start [`Effect::ChooseCard`].
#[allow(clippy::too_many_arguments)]
pub(super) fn choose_card_start(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
    zone: ChooseZone,
    filter: &'static Filter,
    min: u8,
    max: Amount,
    movement: ChooseMove,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let ask_player = players_of(chooser, state, you, res).first().copied()?;
    if state.has_left(ask_player) {
        return None;
    }
    let max_n = amount2(&max, state, you, res);
    if max_n == 0 {
        return None;
    }
    let options: Vec<ObjectId> = match zone {
        ChooseZone::Battlefield => state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    matches_in_resolution(filter, state, o, you, res, Some(ask_player))
                })
            })
            .collect(),
        ChooseZone::Graveyard(owner_rel) => players_of(owner_rel, state, you, res)
            .into_iter()
            .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    matches_in_resolution(filter, state, o, you, res, Some(ask_player))
                })
            })
            .collect(),
        ChooseZone::Hand(_)
        | ChooseZone::Exile(_)
        | ChooseZone::LinkedExile
        | ChooseZone::Found
        | ChooseZone::Remembered => {
            super::wave13a::collect_choose_zone_candidates(state, res, zone, filter, ask_player)
                .into_iter()
                .map(|(id, _, _)| id)
                .collect()
        }
    };
    if options.is_empty() {
        return None;
    }
    let avail = u8::try_from(options.len()).unwrap_or(u8::MAX);
    let max_clamped = u8::try_from(max_n).unwrap_or(u8::MAX).min(avail);
    let min_clamped = min.min(max_clamped);
    let prompt = super::wave13a::choose_prompt_for(zone, movement);
    res.awaiting = Some(AwaitingOp::ChooseCardOp {
        zone,
        movement,
        then,
    });
    Some(Pending::ChooseCards {
        player: ask_player,
        options,
        min: min_clamped,
        max: max_clamped,
        prompt,
        total: None,
    })
}

/// Complete [`Effect::ChooseCard`] once the player selects `chosen`.
pub(super) fn choose_card_finish(
    state: &mut GameState,
    res: &mut Resolution,
    zone: ChooseZone,
    movement: ChooseMove,
    then: &'static [Effect],
    chosen: &[ObjectId],
) -> Option<Pending> {
    if chosen.is_empty() {
        return None;
    }
    let you = res.controller;
    let valid: Vec<ObjectId> = chosen
        .iter()
        .copied()
        .filter(|&id| {
            state.object(id).is_some_and(|o| match zone {
                ChooseZone::Battlefield => o.zone == crate::zone::Zone::Battlefield,
                ChooseZone::Graveyard(_) => o.zone == crate::zone::Zone::Graveyard,
                ChooseZone::Hand(_) => o.zone == crate::zone::Zone::Hand,
                ChooseZone::Exile(_) | ChooseZone::LinkedExile => {
                    o.zone == crate::zone::Zone::Exile
                }
                ChooseZone::Found => res
                    .subject
                    .found
                    .iter()
                    .any(|r| r.object == id && r.version == o.version),
                ChooseZone::Remembered => res.subject.remembered().is_some_and(|list| {
                    list.iter()
                        .any(|r| r.reference.object == id && r.reference.version == o.version)
                }),
            })
        })
        .collect();
    if valid.is_empty() {
        return None;
    }
    if let Some(moves) = planned_choose_moves(state, you, &valid, movement)
        && let Some(pending) = ask_commander_replace(state, res, &moves)
    {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement retains owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::ChooseCardOp {
                zone,
                movement,
                then,
            }),
            chosen: valid,
            asked,
            remaining,
        });
        return Some(pending);
    }
    let mut final_objects = Vec::with_capacity(valid.len());
    for id in valid {
        let controller = state.object(id).map_or(you, |o| o.controller);
        apply_choose_move(state, res, you, id, movement);
        final_objects.push((id, controller));
    }
    let captured_ids: Vec<ObjectId> = final_objects.iter().map(|&(id, _)| id).collect();
    res.subject.note_captured_ids(&captured_ids);
    if let Some(&(first_id, _)) = final_objects.first() {
        bind_remembered_object(state, res, first_id);
    }
    if !then.is_empty() {
        schedule_chosen_then(state, res, &final_objects, then);
    }
    None
}

fn planned_choose_moves(
    state: &GameState,
    you: PlayerId,
    chosen: &[ObjectId],
    movement: ChooseMove,
) -> Option<Vec<(ObjectId, ZoneLocation)>> {
    if matches!(movement, ChooseMove::None) {
        return None;
    }
    Some(
        chosen
            .iter()
            .map(|&id| {
                let owner = state.object(id).map_or(you, |o| o.owner);
                let dest = match movement {
                    ChooseMove::None => unreachable!(),
                    ChooseMove::Hand => ZoneLocation::Hand(owner),
                    ChooseMove::Battlefield { .. } => ZoneLocation::Battlefield,
                    ChooseMove::TopOfLibrary
                    | ChooseMove::BottomOfLibrary
                    | ChooseMove::BottomOfLibraryChosenOrder
                    | ChooseMove::BottomOfLibraryRandom => ZoneLocation::Library(owner),
                    ChooseMove::Graveyard => ZoneLocation::Graveyard(owner),
                    ChooseMove::Exile | ChooseMove::ExileLinked => ZoneLocation::Exile(owner),
                };
                (id, dest)
            })
            .collect(),
    )
}

fn apply_choose_move(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    id: ObjectId,
    movement: ChooseMove,
) {
    let owner = state.object(id).map_or(you, |o| o.owner);
    match movement {
        ChooseMove::None => {}
        ChooseMove::Hand => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::Battlefield { tapped } => {
            if tapped {
                state.restore_tapped(id, true);
            }
            put_found(state, you, id, SearchDest::Battlefield);
        }
        ChooseMove::TopOfLibrary => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::BottomOfLibrary
        | ChooseMove::BottomOfLibraryChosenOrder
        | ChooseMove::BottomOfLibraryRandom => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
        }
        ChooseMove::Graveyard => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::Exile => {
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        ChooseMove::ExileLinked => exile_linked_from_choice(state, res, owner, id),
    }
}

fn exile_linked_from_choice(
    state: &mut GameState,
    res: &Resolution,
    owner: PlayerId,
    id: ObjectId,
) {
    let host_version =
        super::source_version(state, res).or_else(|| state.object(res.source).map(|o| o.version));
    let components = state.merged_components(id);
    let link = crate::object::Rider::Linked {
        host: res.source,
        until: None,
    };
    if let Some(obj) = state.object_mut(id) {
        obj.kind = ObjectKind::Card;
        obj.riders.push(link);
    }
    let _ = state.move_object(
        id,
        ZoneLocation::Exile(owner),
        ZonePosition::Top,
        Cause::Effect,
    );
    if let Some(version) = host_version {
        for component in components {
            if let Some(object) = state
                .object_mut(component)
                .filter(|o| o.zone == crate::zone::Zone::Exile)
            {
                if !object.riders.contains(&link) {
                    object.riders.push(link);
                }
                state.remember_link(
                    component,
                    baylee_core::ids::DamageSourceRef {
                        object: res.source,
                        version,
                    },
                );
            }
        }
    }
}

fn schedule_chosen_then(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[(ObjectId, PlayerId)],
    then: &'static [Effect],
) {
    if chosen.len() == 1 {
        let next = res.pc + 1;
        res.effects.splice(next..next, flatten(then));
        return;
    }
    let sentinel = Effect::RepeatEach {
        domain: RepeatDomain::Times(Amount::Fixed(0)),
        effects: then,
    };
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let mut steps: Vec<RepeatStep> = chosen
        .iter()
        .filter_map(|&(id, controller)| {
            let version = state.object_or_departed(id)?.version;
            Some(RepeatStep::Permanent {
                identity: DamageSourceRef {
                    object: id,
                    version,
                },
                controller,
                require_battlefield: false,
            })
        })
        .collect();
    if steps.is_empty() {
        return;
    }
    steps.reverse();
    res.subject.repeat_frames.push(RepeatFrame {
        tail_from_end: after_len,
        steps: RepeatSteps::Items(steps),
        saved_event_object: res.event_object,
        saved_event_identity: res.subject.event_identity(),
        saved_event_player: res.subject.event_player(),
    });
    let _ = advance_repeat_frame(state, res, then, sentinel);
}

/// Start [`Effect::PeekAndReveal`].
#[allow(clippy::too_many_arguments)]
pub(super) fn peek_and_reveal_start(
    state: &mut GameState,
    res: &mut Resolution,
    library: PlayerRel,
    count: Amount,
    reveal_all: bool,
    reveal_chosen: bool,
    filter: &'static Filter,
    min_pick: u8,
    max_pick: u8,
    picked: PeekPickDest,
    rest: PeekRestDest,
    if_picked: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let lib_owner = players_of(library, state, you, res).first().copied()?;
    let n = amount2(&count, state, you, res) as usize;
    if n == 0 {
        return None;
    }
    let top: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(lib_owner))
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect();
    if top.is_empty() {
        return None;
    }
    if reveal_all {
        state.journal.record(GameEvent::Revealed {
            player: lib_owner,
            cards: top.clone(),
        });
    }
    let context = res.rule_context();
    let options: Vec<ObjectId> = if max_pick == 0 {
        Vec::new()
    } else {
        top.iter()
            .copied()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
            })
            .collect()
    };
    if max_pick == 0 && !reveal_all && matches!(rest, PeekRestDest::StayInOrder) {
        res.awaiting = Some(AwaitingOp::InspectHand);
        return Some(Pending::ChooseCards {
            player: you,
            options: top,
            min: 0,
            max: 0,
            prompt: ChoicePrompt::LookAtLibrary,
            total: None,
        });
    }
    if max_pick == 0 || (reveal_all && options.is_empty()) {
        return peek_and_reveal_finish(
            state,
            res,
            lib_owner,
            top,
            false,
            picked,
            rest,
            if_picked,
            &[],
        );
    }
    let avail = u8::try_from(options.len()).unwrap_or(u8::MAX);
    let max = max_pick.min(avail);
    let min = if !reveal_all && !matches!(filter, Filter::Any) {
        0
    } else {
        min_pick.min(max)
    };
    res.awaiting = Some(AwaitingOp::PeekAndRevealOp {
        lib_owner,
        looked: top,
        reveal_chosen: !reveal_all && reveal_chosen,
        picked,
        rest,
        if_picked,
    });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min,
        max,
        prompt: peek_choice_prompt(picked),
        total: None,
    })
}

const fn peek_choice_prompt(picked: PeekPickDest) -> ChoicePrompt {
    match picked {
        PeekPickDest::Hand => ChoicePrompt::PutIntoHand,
        PeekPickDest::Battlefield { .. } => ChoicePrompt::PutOntoBattlefield,
        PeekPickDest::TopOfLibrary => ChoicePrompt::PutBackOnTop,
        PeekPickDest::StayOnTop | PeekPickDest::Graveyard | PeekPickDest::Exile => {
            ChoicePrompt::Generic
        }
    }
}

/// Complete [`Effect::PeekAndReveal`] after `chosen` is selected.
#[allow(clippy::too_many_arguments)]
pub(super) fn peek_and_reveal_finish(
    state: &mut GameState,
    res: &mut Resolution,
    lib_owner: PlayerId,
    looked: Vec<ObjectId>,
    reveal_chosen: bool,
    picked: PeekPickDest,
    rest: PeekRestDest,
    if_picked: &'static [Effect],
    chosen: &[ObjectId],
) -> Option<Pending> {
    if reveal_chosen && !chosen.is_empty() {
        state.journal.record(GameEvent::Revealed {
            player: lib_owner,
            cards: chosen.to_vec(),
        });
    }
    let remaining: Vec<ObjectId> = looked
        .iter()
        .copied()
        .filter(|id| !chosen.contains(id))
        .collect();
    let moves = planned_peek_moves(
        state,
        res.controller,
        lib_owner,
        chosen,
        &remaining,
        picked,
        rest,
    );
    if !moves.is_empty()
        && let Some(pending) = ask_commander_replace(state, res, &moves)
    {
        let Some(AwaitingOp::CommanderReplace {
            asked,
            remaining: owners_left,
        }) = res.awaiting.take()
        else {
            unreachable!("commander replacement retains owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::PeekAndRevealOp {
                lib_owner,
                looked,
                reveal_chosen: false,
                picked,
                rest,
                if_picked,
            }),
            chosen: chosen.to_vec(),
            asked,
            remaining: owners_left,
        });
        return Some(pending);
    }
    for &card in chosen {
        place_peek_picked(state, res.controller, lib_owner, card, picked);
    }
    if !chosen.is_empty() {
        res.subject.note_captured_ids(chosen);
    } else if remaining.len() == looked.len()
        && matches!(rest, PeekRestDest::StayInOrder | PeekRestDest::TopAnyOrder)
    {
        res.subject.note_captured_ids(&looked);
    }
    if let Some(&first) = chosen.first() {
        bind_remembered_object(state, res, first);
    }
    if !chosen.is_empty() && !if_picked.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, flatten(if_picked));
    }
    place_peek_rest(state, res, lib_owner, remaining, rest)
}

fn planned_peek_moves(
    state: &GameState,
    you: PlayerId,
    lib_owner: PlayerId,
    chosen: &[ObjectId],
    remaining: &[ObjectId],
    picked: PeekPickDest,
    rest: PeekRestDest,
) -> Vec<(ObjectId, ZoneLocation)> {
    let mut moves = Vec::new();
    for &card in chosen {
        let owner = state.object(card).map_or(lib_owner, |o| o.owner);
        match picked {
            PeekPickDest::StayOnTop | PeekPickDest::TopOfLibrary => {}
            PeekPickDest::Hand => {
                let recip = if lib_owner == you { you } else { owner };
                moves.push((card, ZoneLocation::Hand(recip)));
            }
            PeekPickDest::Battlefield { .. } => moves.push((card, ZoneLocation::Battlefield)),
            PeekPickDest::Graveyard => moves.push((card, ZoneLocation::Graveyard(owner))),
            PeekPickDest::Exile => moves.push((card, ZoneLocation::Exile(owner))),
        }
    }
    match rest {
        PeekRestDest::StayInOrder
        | PeekRestDest::TopAnyOrder
        | PeekRestDest::BottomOfLibrary { .. } => {}
        PeekRestDest::Graveyard => {
            for &card in remaining {
                let owner = state.object(card).map_or(lib_owner, |o| o.owner);
                moves.push((card, ZoneLocation::Graveyard(owner)));
            }
        }
        PeekRestDest::Exile => {
            for &card in remaining {
                let owner = state.object(card).map_or(lib_owner, |o| o.owner);
                moves.push((card, ZoneLocation::Exile(owner)));
            }
        }
    }
    moves
}

fn place_peek_picked(
    state: &mut GameState,
    you: PlayerId,
    lib_owner: PlayerId,
    card: ObjectId,
    picked: PeekPickDest,
) {
    let owner = state.object(card).map_or(lib_owner, |o| o.owner);
    match picked {
        PeekPickDest::StayOnTop => {}
        PeekPickDest::TopOfLibrary => {
            let lib = ZoneLocation::Library(lib_owner);
            if state.zones.list(lib).contains(&card) {
                state.zones.remove(card, lib);
                state.zones.insert(card, lib, ZonePosition::Top, true);
            }
        }
        PeekPickDest::Hand => {
            let recip = if lib_owner == you { you } else { owner };
            let _ = state.move_object(
                card,
                ZoneLocation::Hand(recip),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        PeekPickDest::Battlefield { tapped } => {
            if tapped {
                state.restore_tapped(card, true);
            }
            put_found(state, you, card, SearchDest::Battlefield);
        }
        PeekPickDest::Graveyard => {
            let _ = state.move_object(
                card,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        PeekPickDest::Exile => {
            let _ = state.move_object(
                card,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }
}

fn place_peek_rest(
    state: &mut GameState,
    res: &mut Resolution,
    lib_owner: PlayerId,
    mut remaining: Vec<ObjectId>,
    rest: PeekRestDest,
) -> Option<Pending> {
    if remaining.is_empty() {
        return None;
    }
    match rest {
        PeekRestDest::StayInOrder => None,
        PeekRestDest::TopAnyOrder => {
            if remaining.len() == 1 {
                let lib = ZoneLocation::Library(lib_owner);
                let card = remaining[0];
                if state.zones.list(lib).contains(&card) {
                    state.zones.remove(card, lib);
                    state.zones.insert(card, lib, ZonePosition::Top, true);
                }
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReorderTopLibrary { player: lib_owner });
            let n = u32::try_from(remaining.len()).unwrap_or(u32::MAX);
            Some(Pending::Arrange {
                player: res.controller,
                cards: remaining,
                piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, n)],
                prompt: ArrangePrompt::Order,
            })
        }
        PeekRestDest::BottomOfLibrary { random } => {
            if random {
                state.rng.shuffle(&mut remaining);
            } else if remaining.len() > 1 {
                res.awaiting = Some(AwaitingOp::PeekBottom { player: lib_owner });
                let n = u32::try_from(remaining.len()).unwrap_or(u32::MAX);
                return Some(Pending::Arrange {
                    player: res.controller,
                    cards: remaining,
                    piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
                    prompt: ArrangePrompt::Order,
                });
            }
            let lib = ZoneLocation::Library(lib_owner);
            for card in remaining {
                if state.zones.list(lib).contains(&card) {
                    state.zones.remove(card, lib);
                    state.zones.insert(card, lib, ZonePosition::Bottom, true);
                }
            }
            None
        }
        PeekRestDest::Graveyard => {
            for card in remaining {
                let owner = state.object(card).map_or(lib_owner, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        PeekRestDest::Exile => {
            for card in remaining {
                let owner = state.object(card).map_or(lib_owner, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
    }
}

/// Start [`Effect::RevealFromHand`].
pub(super) fn reveal_from_hand_start(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    filter: &'static Filter,
    min: u8,
    max: Amount,
    if_revealed: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let max_n = u8::try_from(amount2(&max, state, you, res)).unwrap_or(u8::MAX);
    if max_n == 0 {
        return None;
    }
    let mut players = players_of(who, state, you, res);
    players.sort_by_key(|p| crate::battle::sba::apnap_distance(state, *p));
    players.reverse();
    advance_hand_reveal(
        state,
        res,
        players,
        filter,
        min,
        max_n,
        false,
        None,
        if_revealed,
    )
}

/// Resume [`Effect::RevealFromHand`] after `player` chooses `chosen`.
#[allow(clippy::too_many_arguments)]
pub(super) fn reveal_from_hand_resume(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    remaining_players: Vec<PlayerId>,
    filter: &'static Filter,
    min: u8,
    max: u8,
    mut any_revealed: bool,
    mut first_revealed: Option<ObjectId>,
    if_revealed: &'static [Effect],
    chosen: &[ObjectId],
) -> Option<Pending> {
    if !chosen.is_empty() {
        any_revealed = true;
        if first_revealed.is_none() {
            first_revealed = chosen.first().copied();
        }
        state.journal.record(GameEvent::Revealed {
            player,
            cards: chosen.to_vec(),
        });
    }
    advance_hand_reveal(
        state,
        res,
        remaining_players,
        filter,
        min,
        max,
        any_revealed,
        first_revealed,
        if_revealed,
    )
}

#[allow(clippy::too_many_arguments)]
fn advance_hand_reveal(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining_players: Vec<PlayerId>,
    filter: &'static Filter,
    min: u8,
    max: u8,
    any_revealed: bool,
    first_revealed: Option<ObjectId>,
    if_revealed: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let context = res.rule_context();
    while let Some(player) = remaining_players.pop() {
        if state.has_left(player) {
            continue;
        }
        let options: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Hand(player))
            .iter()
            .copied()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
            })
            .collect();
        if options.is_empty() {
            continue;
        }
        let avail = u8::try_from(options.len()).unwrap_or(u8::MAX);
        let max_c = max.min(avail);
        // From a hidden hand, only an unrestricted `Filter::Any` reveal can force
        // `min > 0` (CR 701.20a, CR 608.2d).
        let min_c = if matches!(filter, Filter::Any) {
            min.min(max_c)
        } else {
            0
        };
        res.awaiting = Some(AwaitingOp::RevealFromHandOp {
            player,
            remaining_players,
            filter,
            min,
            max,
            any_revealed,
            first_revealed,
            if_revealed,
        });
        return Some(Pending::ChooseCards {
            player,
            options,
            min: min_c,
            max: max_c,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    if let Some(first) = first_revealed {
        bind_remembered_object(state, res, first);
    }
    if any_revealed && !if_revealed.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, flatten(if_revealed));
    }
    None
}

/// Execute [`Effect::RevealTop`] (`CR 701.20a–b`).
pub(super) fn reveal_top(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerRel,
    count: Amount,
    filter: &'static Filter,
    if_matches: &'static [Effect],
    otherwise: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let lib_owner = players_of(player, state, you, res).first().copied()?;
    let n = amount2(&count, state, you, res) as usize;
    if n == 0 {
        return None;
    }
    let cards: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(lib_owner))
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect();
    if cards.is_empty() {
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: lib_owner,
        cards: cards.clone(),
    });
    bind_remembered_object(state, res, cards[0]);
    let context = res.rule_context();
    let matched = cards.iter().any(|&id| {
        state
            .object(id)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
    });
    let branch = if matched { if_matches } else { otherwise };
    if branch.is_empty() {
        None
    } else {
        run_nested(state, res, branch)
    }
}
