//! Generalized delayed-trigger registration and remembered-object LKI capture
//! (`Effect::RegisterDelayedTrigger`, `Effect::ReflexiveTrigger`, CR 603.7, CR 603.12).

use baylee_cards_dsl::{Condition, DelayedTriggerWhen, Effect, TargetReq, TriggerRemember};
use baylee_core::ids::{DamageSourceRef, ObjectId};
use smallvec::{SmallVec, smallvec};

use super::Resolution;
use crate::event::{Departure, GameEvent};
use crate::object::Rider;
use crate::state::{
    DelayedAction, DelayedTrigger, DelayedTriggerSpec, DelayedWhen, GameState, RememberedContext,
};
use crate::text_changes::RuleContext;
use crate::zone::Zone;

/// Capture the current or last-known characteristics of `object` for a
/// delayed or reflexive trigger's `RememberedContext`.
pub(super) fn context_from_object(
    state: &GameState,
    object: ObjectId,
    version_override: Option<u32>,
) -> RememberedContext {
    let Some(obj) = state.object_or_departed(object) else {
        return RememberedContext {
            object: Some(object),
            identity: version_override.map(|v| (v, 0)),
            ..RememberedContext::default()
        };
    };
    let chars = state
        .last_known_characteristics(object)
        .unwrap_or_else(|| obj.characteristics());
    let version = version_override.unwrap_or(obj.version);
    let power = chars.power.unwrap_or(0);
    let toughness = chars.toughness.unwrap_or(0);
    let ctrl = state
        .last_known_controller(object)
        .unwrap_or(obj.controller);
    RememberedContext {
        object: Some(object),
        identity: Some((version, power)),
        departure: (obj.zone != Zone::Battlefield).then_some((ctrl, toughness)),
        mana_value: Some(chars.mana_value()),
        player: Some(ctrl),
        amount: None,
        subject: None,
    }
}

/// Capture the post-departure incarnation and pre-departure battlefield LKI
/// from a journaled [`Departure`].
pub(super) fn context_from_departure(
    state: &GameState,
    object: ObjectId,
    dep: &Departure,
) -> RememberedContext {
    let version = state
        .object(object)
        .map_or_else(|| dep.version.saturating_add(1), |o| o.version);
    let mana_value = state
        .damage_source(object, Some(dep.version))
        .map(|o| o.characteristics().mana_value())
        .or_else(|| {
            state
                .last_known_characteristics(object)
                .map(crate::object::Characteristics::mana_value)
        })
        .or_else(|| {
            state
                .object_or_departed(object)
                .map(|o| o.characteristics().mana_value())
        });
    RememberedContext {
        object: Some(object),
        identity: Some((version, dep.power)),
        departure: Some((dep.controller, dep.toughness)),
        mana_value,
        player: Some(dep.controller),
        amount: None,
        subject: None,
    }
}

/// Resolve a [`TriggerRemember`] specification against the current [`Resolution`].
pub(super) fn resolve_remembered(
    state: &GameState,
    res: &Resolution,
    remember: TriggerRemember,
) -> SmallVec<[RememberedContext; 2]> {
    match remember {
        TriggerRemember::None => smallvec![RememberedContext::default()],
        TriggerRemember::Source => {
            let ver = super::subjects::source(state, res).map(|s| s.version);
            smallvec![context_from_object(state, res.source, ver)]
        }
        TriggerRemember::FirstTarget => {
            if let Some(&target) = res.targets.first() {
                if state.object(target).is_some() {
                    smallvec![context_from_object(state, target, None)]
                } else {
                    SmallVec::new()
                }
            } else if let Some(player) = res.target_players.iter().next() {
                smallvec![RememberedContext {
                    player: Some(player),
                    ..RememberedContext::default()
                }]
            } else {
                SmallVec::new()
            }
        }
        TriggerRemember::EachTarget => {
            if res.targets.is_empty() {
                res.target_players
                    .iter()
                    .map(|player| RememberedContext {
                        player: Some(player),
                        ..RememberedContext::default()
                    })
                    .collect()
            } else {
                res.targets
                    .iter()
                    .copied()
                    .filter(|&target| state.object(target).is_some())
                    .map(|target| context_from_object(state, target, None))
                    .collect()
            }
        }
        TriggerRemember::EventObject => event_object_context(state, res),
        TriggerRemember::EventSubject => {
            let Some(subject) = state.object(res.on_stack).and_then(|o| {
                o.riders.iter().find_map(|r| match *r {
                    Rider::EventSubject(s) => Some(s),
                    _ => None,
                })
            }) else {
                return SmallVec::new();
            };
            let mut ctx = context_from_object(state, subject.object, Some(subject.version));
            ctx.subject = Some(subject);
            smallvec![ctx]
        }
        TriggerRemember::EachCreatedToken => super::this_way::created_tokens(state, res)
            .into_iter()
            .map(|token| context_from_object(state, token, None))
            .collect(),
        TriggerRemember::SacrificedObject => sacrificed_contexts(state, res),
        TriggerRemember::EachExiledThisWay => exiled_this_way_contexts(state, res),
        TriggerRemember::TargetAndSacrificed => {
            let mut out = SmallVec::new();
            if let Some(&target) = res.targets.first()
                && state.object(target).is_some()
            {
                out.push(context_from_object(state, target, None));
            }
            for ctx in sacrificed_contexts(state, res) {
                if let Some(id) = ctx.object
                    && let Some(obj) = state
                        .object(id)
                        .filter(|o| o.zone == crate::zone::Zone::Graveyard)
                {
                    out.push(context_from_object(state, id, Some(obj.version)));
                }
            }
            out
        }
    }
}

fn event_object_context(state: &GameState, res: &Resolution) -> SmallVec<[RememberedContext; 2]> {
    let stack_obj = state.object(res.on_stack);
    let identity = super::event_object_identity(state, res);
    let departure = stack_obj.and_then(|o| {
        o.riders.iter().find_map(|r| match *r {
            Rider::EventDeparture(ctrl, tough) => Some((ctrl, tough)),
            _ => None,
        })
    });
    let mana_value = stack_obj.and_then(|o| {
        o.riders.iter().find_map(|r| match *r {
            Rider::EventManaValue(mv) => Some(mv),
            _ => None,
        })
    });
    let player = stack_obj.and_then(|o| {
        o.riders.iter().find_map(|r| match *r {
            Rider::EventPlayer(p) => Some(p),
            _ => None,
        })
    });
    let amount = stack_obj.and_then(|o| {
        o.riders.iter().find_map(|r| match *r {
            Rider::EventAmount(a) => Some(a),
            _ => None,
        })
    });
    let subject = stack_obj.and_then(|o| {
        o.riders.iter().find_map(|r| match *r {
            Rider::EventSubject(s) => Some(s),
            _ => None,
        })
    });
    if let Some(object) = res.event_object {
        let mut base = context_from_object(state, object, identity.map(|(v, _)| v));
        if let Some(id) = identity {
            base.identity = Some(id);
        }
        if departure.is_some() {
            base.departure = departure;
        }
        if mana_value.is_some() {
            base.mana_value = mana_value;
        }
        if player.is_some() {
            base.player = player;
        }
        base.amount = amount;
        base.subject = subject;
        smallvec![base]
    } else if player.is_some() || amount.is_some() {
        smallvec![RememberedContext {
            player,
            amount,
            subject,
            ..RememberedContext::default()
        }]
    } else {
        SmallVec::new()
    }
}

fn sacrificed_contexts(state: &GameState, res: &Resolution) -> SmallVec<[RememberedContext; 2]> {
    let mut out = SmallVec::new();
    for entry in super::this_way::entries(state, res) {
        if let GameEvent::ZoneChanged {
            object,
            from: Zone::Battlefield,
            ..
        } = entry.event
            && let Some(dep) = entry.departure.as_deref()
            && dep.sacrificed
            && dep.controller == res.controller
        {
            out.push(context_from_departure(state, object, dep));
        }
    }
    if out.is_empty()
        && let Some(paid) = state.object(res.on_stack).and_then(|o| o.paid.as_ref())
    {
        if !paid.sacrificed_many.is_empty() {
            for &(id, ver, power) in &paid.sacrificed_many {
                let mut ctx = context_from_object(state, id, Some(ver));
                let clamped = i16::try_from(power).unwrap_or(i16::MAX);
                ctx.identity = Some((ver, clamped));
                out.push(ctx);
            }
        } else if let Some((id, ver)) = paid.sacrificed {
            let mut ctx = context_from_object(state, id, Some(ver));
            if let Some(power) = state
                .object(res.on_stack)
                .and_then(|o| o.source_power_lki)
                .filter(|_| id == res.source)
            {
                ctx.identity = Some((ver, power));
            }
            out.push(ctx);
        }
    }
    out
}

fn exiled_this_way_contexts(
    state: &GameState,
    res: &Resolution,
) -> SmallVec<[RememberedContext; 2]> {
    let mut seen: SmallVec<[ObjectId; 2]> = SmallVec::new();
    let mut out = SmallVec::new();
    for entry in super::this_way::entries(state, res) {
        if let GameEvent::ZoneChanged {
            object,
            to: Zone::Exile,
            cause: crate::event::Cause::Effect,
            ..
        } = entry.event
            && !seen.contains(&object)
            && state.object(object).is_some_and(|o| o.zone == Zone::Exile)
        {
            seen.push(object);
            out.push(context_from_object(state, object, None));
        }
    }
    out
}

/// Register one or more delayed triggers from [`Effect::RegisterDelayedTrigger`]
/// (CR 603.7a–h).
#[allow(clippy::too_many_arguments)] // Direct unpack of Effect::RegisterDelayedTrigger fields.
pub(super) fn register(
    state: &mut GameState,
    res: &Resolution,
    when: DelayedTriggerWhen,
    condition: Option<&'static Condition>,
    targets: Option<TargetReq>,
    remember: TriggerRemember,
    effects: &'static [Effect],
) {
    let source_version = super::subjects::moving_source(state, res)
        .or_else(|| super::subjects::source(state, res))
        .map(|s| s.version)
        .or_else(|| state.object_or_departed(res.source).map(|o| o.version))
        .unwrap_or(0);
    let after = state.journal.last_seq();

    let build_spec = |remembered: Option<RememberedContext>| {
        DelayedAction::GeneralTrigger(Box::new(DelayedTriggerSpec {
            source: res.source,
            source_version,
            effects,
            text: res.text,
            condition,
            targets,
            remembered,
            related_objects: Vec::new(),
        }))
    };

    match when {
        DelayedTriggerWhen::AtStep {
            step,
            whose,
            this_turn_only,
            recurring,
        } => register_at_step(
            state,
            res,
            step,
            whose,
            this_turn_only,
            recurring,
            remember,
            &build_spec,
        ),
        DelayedTriggerWhen::OnTrigger {
            trigger,
            watch,
            duration,
        } => register_on_trigger(
            state,
            res,
            trigger,
            watch,
            duration,
            remember,
            after,
            &build_spec,
        ),
        DelayedTriggerWhen::OnObservation {
            watch,
            observation,
            this_turn_only,
        } => register_on_observation(
            state,
            res,
            watch,
            observation,
            this_turn_only,
            remember,
            after,
            &build_spec,
        ),
    }
}

#[allow(clippy::too_many_arguments)] // Direct unpack of Effect::RegisterDelayedTrigger fields.
fn register_at_step(
    state: &mut GameState,
    res: &Resolution,
    step: baylee_cards_dsl::DelayedStep,
    whose: Option<baylee_cards_dsl::PlayerRel>,
    this_turn_only: bool,
    recurring: bool,
    remember: TriggerRemember,
    build_spec: &impl Fn(Option<RememberedContext>) -> DelayedAction,
) {
    let players = match whose {
        None => None,
        Some(rel) => {
            let Some(seats) = crate::eval::players(rel, state, res.controller) else {
                return;
            };
            if seats.is_empty() {
                return;
            }
            let mut set = baylee_core::ids::SeatSet::new();
            for seat in seats {
                set.insert(seat);
            }
            Some(set)
        }
    };
    let delayed_when = DelayedWhen::AtStep {
        step,
        players,
        this_turn_only,
        recurring,
    };
    if remember == TriggerRemember::None {
        state.delayed.push(DelayedTrigger {
            controller: res.controller,
            when: delayed_when,
            action: build_spec(None),
        });
    } else if remember == TriggerRemember::TargetAndSacrificed {
        let objects: Vec<DamageSourceRef> = resolve_remembered(state, res, remember)
            .into_iter()
            .filter_map(|ctx| {
                let obj = ctx.object?;
                let (ver, _) = ctx.identity?;
                Some(DamageSourceRef {
                    object: obj,
                    version: ver,
                })
            })
            .collect();
        if !objects.is_empty() {
            let mut action = build_spec(None);
            if let DelayedAction::GeneralTrigger(spec) = &mut action {
                spec.related_objects = objects;
            }
            state.delayed.push(DelayedTrigger {
                controller: res.controller,
                when: delayed_when,
                action,
            });
        }
    } else {
        let contexts = resolve_remembered(state, res, remember);
        if contexts.is_empty()
            && matches!(
                remember,
                TriggerRemember::EachTarget
                    | TriggerRemember::EachCreatedToken
                    | TriggerRemember::EachExiledThisWay
                    | TriggerRemember::SacrificedObject
                    | TriggerRemember::FirstTarget
            )
        {
            return;
        }
        let list = if contexts.is_empty() {
            smallvec![RememberedContext::default()]
        } else {
            contexts
        };
        for ctx in list {
            state.delayed.push(DelayedTrigger {
                controller: res.controller,
                when: delayed_when,
                action: build_spec(Some(ctx)),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)] // Direct unpack of Effect::RegisterDelayedTrigger fields.
fn register_on_trigger(
    state: &mut GameState,
    res: &Resolution,
    trigger: &'static baylee_cards_dsl::Trigger,
    watch: TriggerRemember,
    duration: baylee_cards_dsl::DelayedDuration,
    remember: TriggerRemember,
    after: u64,
    build_spec: &impl Fn(Option<RememberedContext>) -> DelayedAction,
) {
    if watch == TriggerRemember::None {
        let context = RuleContext {
            source: res.source,
            text: res.text,
            resolved_name: res.subject.named.map(|(_, name)| name),
            resolved_subtype: res.subject.chosen_subtype,
            reference: res.event_object,
        };
        let delayed_when = DelayedWhen::OnEvent {
            trigger,
            watch: None,
            watch_battlefield: false,
            duration,
            context,
            after,
        };
        if remember == TriggerRemember::None {
            state.delayed.push(DelayedTrigger {
                controller: res.controller,
                when: delayed_when,
                action: build_spec(None),
            });
        } else {
            for ctx in resolve_remembered(state, res, remember) {
                state.delayed.push(DelayedTrigger {
                    controller: res.controller,
                    when: delayed_when,
                    action: build_spec(Some(ctx)),
                });
            }
        }
        return;
    }

    let watched_list = resolve_remembered(state, res, watch);
    let explicit_rem = (remember != TriggerRemember::None && remember != watch)
        .then(|| resolve_remembered(state, res, remember));
    for watched in watched_list {
        let Some(w_obj) = watched.object else {
            continue;
        };
        let Some((w_ver, _)) = watched.identity else {
            continue;
        };
        let on_bf = state
            .object(w_obj)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.version == w_ver);
        let watch_ref = DamageSourceRef {
            object: w_obj,
            version: w_ver,
        };
        let context = RuleContext {
            source: w_obj,
            text: res.text,
            resolved_name: res.subject.named.map(|(_, name)| name),
            resolved_subtype: res.subject.chosen_subtype,
            reference: res.event_object,
        };
        let delayed_when = DelayedWhen::OnEvent {
            trigger,
            watch: Some(watch_ref),
            watch_battlefield: on_bf,
            duration,
            context,
            after,
        };
        let rem_to_store = if remember == TriggerRemember::None {
            None
        } else if remember == watch {
            Some(watched)
        } else {
            explicit_rem.as_ref().and_then(|list| list.first().copied())
        };
        state.delayed.push(DelayedTrigger {
            controller: res.controller,
            when: delayed_when,
            action: build_spec(rem_to_store),
        });
    }
}

#[allow(clippy::too_many_arguments)] // Direct unpack of Effect::RegisterDelayedTrigger fields.
fn register_on_observation(
    state: &mut GameState,
    res: &Resolution,
    watch: TriggerRemember,
    observation: baylee_cards_dsl::DelayedObservation,
    this_turn_only: bool,
    remember: TriggerRemember,
    after: u64,
    build_spec: &impl Fn(Option<RememberedContext>) -> DelayedAction,
) {
    let watched_list = resolve_remembered(state, res, watch);
    let explicit_rem = (remember != TriggerRemember::None && remember != watch)
        .then(|| resolve_remembered(state, res, remember));
    for watched in watched_list {
        let Some(w_obj) = watched.object else {
            continue;
        };
        let Some((w_ver, _)) = watched.identity else {
            continue;
        };
        if !super::subjects::on_battlefield(
            state,
            DamageSourceRef {
                object: w_obj,
                version: w_ver,
            },
        ) {
            continue;
        }
        let watch_ref = DamageSourceRef {
            object: w_obj,
            version: w_ver,
        };
        let rem_to_store = if remember == TriggerRemember::None || remember == watch {
            Some(watched)
        } else {
            explicit_rem.as_ref().and_then(|list| list.first().copied())
        };
        state.delayed.push(DelayedTrigger {
            controller: res.controller,
            when: DelayedWhen::ObservedGeneral {
                watch: watch_ref,
                when: observation,
                this_turn_only,
                after,
            },
            action: build_spec(rem_to_store),
        });
    }
}
