//! Wave 14d resolution handlers: temporary continuous modifiers (`AddTempModifier` /
//! `AddGlobalTempModifier`), prevention-with-rider shields (`PreventNextDamageWithThen`),
//! and generalized control changes (`GainControlEx` / `GainControlAllEx`).

use baylee_cards_dsl::{
    Duration, Effect, Filter, KeywordSet, Layer, Modifier, PlayerRel, TargetSpec,
};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId, PlayerId, SeatSet};
use smallvec::SmallVec;

use super::{
    AwaitingOp, Pending, Resolution, bound_now_with_context, continuous_subjects,
    filter_reads_resolution_player, matches_in_resolution, players_of, resolving_ability,
    source_attachment_lki, source_still_tapped, source_still_yours, source_version, subjects,
    this_to_affect,
};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::Cause;
use crate::prevention::{
    ChosenSource, Shield, ShieldKind, ShieldOrigin, ShieldSourceFilter, Shielded,
};
use crate::state::GameState;
use crate::zone::Zone;

/// Check CR 611.2b source-tied duration preconditions before registering any
/// temporary continuous effect.
fn temp_duration_ready(
    state: &GameState,
    res: &Resolution,
    duration: Duration,
    you: PlayerId,
) -> bool {
    if !continuous_subjects::source_duration_starts(state, res, duration) {
        return false;
    }
    if matches!(duration, Duration::WhileYouControlSource) && !source_still_yours(state, res, you) {
        return false;
    }
    if matches!(duration, Duration::WhileSourceTapped { .. })
        && (!source_still_tapped(state, res)
            || !subjects::source(state, res).is_some_and(|s| subjects::on_battlefield(state, s)))
    {
        return false;
    }
    true
}

/// Resolve player targets / named players for a player-scoped `TargetSpec`.
fn resolve_player_spec(
    target: TargetSpec,
    state: &GameState,
    you: PlayerId,
    res: &Resolution,
) -> Option<Vec<PlayerId>> {
    match target {
        TargetSpec::Player(rel) => Some(players_of(rel, state, you, res)),
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..) => {
            let mut players: Vec<PlayerId> = res
                .target_players
                .iter()
                .filter(|p| !state.has_left(*p))
                .collect();
            if players.is_empty()
                && let Some(chosen) = res.chosen_player
                && !state.has_left(chosen)
            {
                players.push(chosen);
            }
            Some(players)
        }
        _ => None,
    }
}

/// Resolve [`Effect::AddTempModifier`].
pub(super) fn exec_add_temp_modifier(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    modifier: Modifier,
    duration: Duration,
) -> Option<Pending> {
    let you = res.controller;
    if matches!(modifier, Modifier::GainControl) && state.has_left(you) {
        return None;
    }
    if !temp_duration_ready(state, res, duration, you) {
        return None;
    }

    let mut filters: SmallVec<[EffectFilter; 4]> = SmallVec::new();
    if let Some(players) = resolve_player_spec(target, state, you, res) {
        if players.is_empty() {
            return None;
        }
        let seats: SeatSet = players.into_iter().collect();
        filters.push(EffectFilter::DslControlledBy(&Filter::Any, seats));
    } else {
        match target {
            TargetSpec::ThisObject => {
                let src =
                    subjects::source(state, res).filter(|r| subjects::is_current(state, *r))?;
                filters.push(EffectFilter::ObjectIs(src.object, src.version));
            }
            TargetSpec::EventObject => {
                let ev = subjects::event(state, res).filter(|r| subjects::is_current(state, *r))?;
                filters.push(EffectFilter::ObjectIs(ev.object, ev.version));
            }
            TargetSpec::AnyTarget
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_) => {
                for &id in &res.targets {
                    if let Some(r) = continuous_subjects::target(state, res, id) {
                        filters.push(EffectFilter::ObjectIs(r.object, r.version));
                    }
                }
                let players: Vec<PlayerId> = res
                    .target_players
                    .iter()
                    .filter(|p| !state.has_left(*p))
                    .collect();
                if !players.is_empty() {
                    let seats: SeatSet = players.into_iter().collect();
                    filters.push(EffectFilter::DslControlledBy(&Filter::Any, seats));
                }
            }
            _ => {
                if !res.targets.is_empty() {
                    for &id in &res.targets {
                        if let Some(r) = continuous_subjects::target(state, res, id) {
                            filters.push(EffectFilter::ObjectIs(r.object, r.version));
                        }
                    }
                } else if !res.targeted {
                    if matches!(target, TargetSpec::Object(f) if *f == Filter::This) {
                        if let Some(src) =
                            subjects::source(state, res).filter(|r| subjects::is_current(state, *r))
                        {
                            filters.push(EffectFilter::ObjectIs(src.object, src.version));
                        }
                    } else if matches!(target, TargetSpec::Object(f) if *f == Filter::AttachedToBySource)
                        && let Some(host) = crate::eval::attached_for_ability(
                            state,
                            res.source,
                            source_version(state, res),
                            source_attachment_lki(state, res.on_stack),
                        )
                    {
                        filters.push(EffectFilter::ObjectIs(host.id, host.version));
                    }
                }
            }
        }
    }

    if filters.is_empty() {
        return None;
    }

    register_temp_effects(state, res, you, modifier, duration, &filters);
    None
}

/// Resolve [`Effect::AddGlobalTempModifier`].
pub(super) fn exec_add_global_temp_modifier(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    modifier: Modifier,
    duration: Duration,
) -> Option<Pending> {
    let you = res.controller;
    if matches!(modifier, Modifier::GainControl) && state.has_left(you) {
        return None;
    }
    if !temp_duration_ready(state, res, duration, you) {
        return None;
    }

    let bound_seats = super::immediate::resolution_bound_seats(state, res, you, &modifier);
    if matches!(bound_seats.as_deref(), Some([])) {
        return None;
    }

    let filters: SmallVec<[EffectFilter; 4]> = if matches!(filter, Filter::This) {
        let this = this_to_affect(state, res)?;
        smallvec::smallvec![EffectFilter::object(state, this)]
    } else if filter_reads_resolution_player(filter) {
        let seats = bound_seats.as_deref();
        if crate::state::filter_reaches_other_zones(filter) {
            state
                .arena
                .iter()
                .filter(|(_, o)| {
                    !o.status.contains(crate::object::Status::PHASED_OUT)
                        && seats.is_none_or(|s| s.contains(&o.controller))
                        && matches_in_resolution(filter, state, o, you, res, None)
                })
                .map(|(id, _)| EffectFilter::object(state, id))
                .collect()
        } else {
            state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        !o.status.contains(crate::object::Status::PHASED_OUT)
                            && seats.is_none_or(|s| s.contains(&o.controller))
                            && matches_in_resolution(filter, state, o, you, res, None)
                    })
                })
                .map(|id| EffectFilter::object(state, id))
                .collect()
        }
    } else {
        bound_now_with_context(
            state,
            filter,
            &modifier,
            you,
            res.rule_context(),
            bound_seats.as_deref(),
        )
    };

    register_temp_effects(state, res, you, modifier, duration, &filters);
    None
}

fn register_temp_effects(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    modifier: Modifier,
    duration: Duration,
    filters: &[EffectFilter],
) {
    let timestamp = state.next_timestamp();
    let layer = modifier.layer();
    let mut registered_any = false;
    for &filter in filters {
        if !crate::permanent_guards::permits_resolution_effect(state, filter, modifier, you)
            || !state.source_duration_holds(duration, res.source, you, filter)
        {
            continue;
        }
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: EffectOrigin::Resolution,
            layer,
            timestamp,
            duration,
            filter,
            modifier,
        });
        registered_any = true;
    }
    if registered_any {
        if matches!(modifier, Modifier::GrantStatic { .. }) {
            state.refresh_characteristics();
            crate::effects::sync_granted_statics(state);
        } else if matches!(modifier, Modifier::GainControl) {
            state.refresh_characteristics();
        }
    }
}

/// Resolve [`Effect::PreventNextDamageWithThen`].
pub(super) fn exec_prevent_next_damage_with_then(
    state: &mut GameState,
    res: &Resolution,
    sources: TargetSpec,
    recipients: TargetSpec,
    combat_only: bool,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let context = res.rule_context();
    let chosen_color = res
        .subject
        .chosen_color
        .or_else(|| state.object(res.source).and_then(|o| o.chosen_color));
    let chosen_subtype = context
        .resolved_subtype
        .or_else(|| state.object(res.source).and_then(|o| o.chosen_subtype));

    let (resolved_sources, consumed_first_targets) = resolve_shield_sources(
        state,
        res,
        sources,
        you,
        context,
        chosen_color,
        chosen_subtype,
    )?;
    if resolved_sources.is_empty() {
        return None;
    }

    let shielded = resolve_shield_recipients(state, res, recipients, you, consumed_first_targets);
    if shielded.is_empty() {
        return None;
    }

    let shield_origin = ShieldOrigin {
        source: res.source,
        ability: resolving_ability(state, res),
    };
    let origin_version =
        source_version(state, res).or_else(|| state.object(res.source).map(|o| o.version));
    for source in resolved_sources {
        for &protects in &shielded {
            state.shields.push_from(
                Shield {
                    protects,
                    kind: ShieldKind::NextWithThen {
                        source,
                        combat_only,
                        then,
                        origin_source: res.source,
                        origin_version,
                        text: context.text,
                        chosen_subtype,
                        chosen_color,
                    },
                    controller: you,
                },
                Some(shield_origin),
            );
        }
    }
    None
}

fn resolve_shield_sources(
    state: &GameState,
    res: &Resolution,
    sources: TargetSpec,
    you: PlayerId,
    context: crate::text_changes::RuleContext,
    chosen_color: Option<baylee_core::mana::ManaColor>,
    chosen_subtype: Option<baylee_core::ids::SubtypeId>,
) -> Option<(SmallVec<[ShieldSourceFilter; 2]>, bool)> {
    let mut consumed_first_targets = false;
    let mut resolved: SmallVec<[ShieldSourceFilter; 2]> = SmallVec::new();
    match sources {
        TargetSpec::ThisObject => {
            let reference =
                subjects::source(state, res).or_else(|| state.source_identity(res.source))?;
            let chosen = ChosenSource::new_with_choices(
                state,
                reference,
                &Filter::Any,
                you,
                context,
                chosen_color,
                chosen_subtype,
            )?;
            resolved.push(ShieldSourceFilter::Exact(chosen));
        }
        TargetSpec::EventObject => {
            let reference = subjects::event(state, res)?;
            let chosen = ChosenSource::new_with_choices(
                state,
                reference,
                &Filter::Any,
                you,
                context,
                chosen_color,
                chosen_subtype,
            )?;
            resolved.push(ShieldSourceFilter::Exact(chosen));
        }
        TargetSpec::Object(filter)
        | TargetSpec::Spell(filter)
        | TargetSpec::StackOrBattlefield(filter)
        | TargetSpec::SpellOrAbility(filter) => {
            if !res.targets.is_empty() {
                consumed_first_targets = true;
                for &id in &res.targets {
                    if let Some(reference) =
                        subjects::target(state, res, id).or_else(|| state.source_identity(id))
                        && let Some(chosen) = ChosenSource::new_with_choices(
                            state,
                            reference,
                            &Filter::Any,
                            you,
                            context,
                            chosen_color,
                            chosen_subtype,
                        )
                    {
                        resolved.push(ShieldSourceFilter::Exact(chosen));
                    }
                }
            } else if res.targeted && res.target_players.is_empty() {
                // Targeted source was lost before resolution; create no shield.
                return None;
            } else {
                resolved.push(ShieldSourceFilter::Matching {
                    sources: filter,
                    chosen_color: crate::eval::filter_reads_chosen_color(filter)
                        .then_some(chosen_color)
                        .flatten(),
                    chosen_subtype: crate::eval::filter_reads_chosen_subtype(filter)
                        .then_some(chosen_subtype)
                        .flatten(),
                    text: context.text,
                    this: context.source,
                });
            }
        }
        _ => {
            resolved.push(ShieldSourceFilter::Matching {
                sources: &Filter::Any,
                chosen_color: None,
                chosen_subtype: None,
                text: context.text,
                this: context.source,
            });
        }
    }
    Some((resolved, consumed_first_targets))
}

fn resolve_shield_recipients(
    state: &GameState,
    res: &Resolution,
    recipients: TargetSpec,
    you: PlayerId,
    consumed_first_targets: bool,
) -> SmallVec<[Shielded; 2]> {
    let mut shielded: SmallVec<[Shielded; 2]> = SmallVec::new();
    match recipients {
        TargetSpec::AnyTarget => shielded.push(Shielded::Everything),
        TargetSpec::ThisObject => {
            if let Some(src) =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                shielded.push(Shielded::Object(src.object, src.version));
            }
        }
        TargetSpec::EventObject => {
            if let Some(ev) =
                subjects::event(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                shielded.push(Shielded::Object(ev.object, ev.version));
            }
        }
        TargetSpec::Player(rel) => {
            for p in players_of(rel, state, you, res) {
                shielded.push(Shielded::Player(p));
            }
        }
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..) => {
            for p in res.target_players.iter().filter(|p| !state.has_left(*p)) {
                shielded.push(Shielded::Player(p));
            }
            if shielded.is_empty()
                && let Some(chosen) = res.chosen_player
                && !state.has_left(chosen)
            {
                shielded.push(Shielded::Player(chosen));
            }
        }
        TargetSpec::OpponentOrObject(_) | TargetSpec::PlayerOrObject(_) => {
            let list = if consumed_first_targets {
                res.second_targets.as_slice()
            } else {
                res.targets.as_slice()
            };
            for &id in list {
                if let Some(r) = continuous_subjects::target(state, res, id)
                    && state
                        .object(r.object)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                {
                    shielded.push(Shielded::Object(r.object, r.version));
                }
            }
            for p in res.target_players.iter().filter(|p| !state.has_left(*p)) {
                shielded.push(Shielded::Player(p));
            }
        }
        TargetSpec::Object(filter) => {
            let list = if consumed_first_targets {
                res.second_targets.as_slice()
            } else {
                res.targets.as_slice()
            };
            if !list.is_empty() {
                for &id in list {
                    if let Some(r) = continuous_subjects::target(state, res, id)
                        && state
                            .object(r.object)
                            .is_some_and(|o| o.zone == Zone::Battlefield)
                    {
                        shielded.push(Shielded::Object(r.object, r.version));
                    }
                }
            } else if *filter == Filter::This {
                if let Some(src) =
                    subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
                {
                    shielded.push(Shielded::Object(src.object, src.version));
                }
            } else if *filter == Filter::AttachedToBySource {
                if let Some(host) = crate::eval::attached_for_ability(
                    state,
                    res.source,
                    source_version(state, res),
                    source_attachment_lki(state, res.on_stack),
                ) {
                    shielded.push(Shielded::Object(host.id, host.version));
                }
            } else if *filter == Filter::Any && !res.targeted {
                shielded.push(Shielded::Everything);
            }
        }
        _ => {}
    }
    shielded
}

/// Resolve [`Effect::GainControlEx`].
pub(super) fn exec_gain_control_ex(
    state: &mut GameState,
    res: &mut Resolution,
    target: TargetSpec,
    to: PlayerRel,
    duration: Duration,
    untap: bool,
    haste_until_eot: bool,
) -> Option<Pending> {
    let you = res.controller;
    if !temp_duration_ready(state, res, duration, you) {
        return None;
    }

    let mut subjects_list: Vec<DamageSourceRef> = Vec::new();
    match target {
        TargetSpec::ThisObject => {
            if let Some(src) =
                subjects::source(state, res).filter(|s| subjects::on_battlefield(state, *s))
            {
                subjects_list.push(src);
            }
        }
        TargetSpec::EventObject => {
            if let Some(ev) =
                subjects::event(state, res).filter(|s| subjects::on_battlefield(state, *s))
            {
                subjects_list.push(ev);
            }
        }
        _ => {
            if !res.targets.is_empty() {
                for &id in &res.targets {
                    if let Some(r) = continuous_subjects::target(state, res, id)
                        && state
                            .object(r.object)
                            .is_some_and(|o| o.zone == Zone::Battlefield)
                    {
                        subjects_list.push(r);
                    }
                }
            } else if !res.targeted {
                if matches!(target, TargetSpec::Object(f) if *f == Filter::This) {
                    if let Some(src) =
                        subjects::source(state, res).filter(|s| subjects::on_battlefield(state, *s))
                    {
                        subjects_list.push(src);
                    }
                } else if matches!(target, TargetSpec::Object(f) if *f == Filter::AttachedToBySource)
                    && let Some(host) = crate::eval::attached_for_ability(
                        state,
                        res.source,
                        source_version(state, res),
                        source_attachment_lki(state, res.on_stack),
                    )
                    && host.zone == Zone::Battlefield
                {
                    subjects_list.push(DamageSourceRef {
                        object: host.id,
                        version: host.version,
                    });
                }
            }
        }
    }

    if subjects_list.is_empty() {
        return None;
    }

    dispatch_gain_control_ex(
        state,
        res,
        subjects_list,
        to,
        duration,
        untap,
        haste_until_eot,
    )
}

/// Resolve [`Effect::GainControlAllEx`].
pub(super) fn exec_gain_control_all_ex(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    to: PlayerRel,
    duration: Duration,
    untap: bool,
    haste_until_eot: bool,
) -> Option<Pending> {
    let you = res.controller;
    if !temp_duration_ready(state, res, duration, you) {
        return None;
    }

    let subjects_list: Vec<DamageSourceRef> = if matches!(filter, Filter::This) {
        subjects::source(state, res)
            .filter(|s| subjects::on_battlefield(state, *s))
            .into_iter()
            .collect()
    } else if matches!(filter, Filter::AttachedToBySource) {
        crate::eval::attached_for_ability(
            state,
            res.source,
            source_version(state, res),
            source_attachment_lki(state, res.on_stack),
        )
        .filter(|o| o.zone == Zone::Battlefield)
        .map(|o| DamageSourceRef {
            object: o.id,
            version: o.version,
        })
        .into_iter()
        .collect()
    } else {
        state
            .battlefield_seen()
            .filter_map(|id| {
                let obj = state.object(id)?;
                matches_in_resolution(filter, state, obj, you, res, None).then_some(
                    DamageSourceRef {
                        object: obj.id,
                        version: obj.version,
                    },
                )
            })
            .collect()
    };

    if subjects_list.is_empty() {
        return None;
    }

    dispatch_gain_control_ex(
        state,
        res,
        subjects_list,
        to,
        duration,
        untap,
        haste_until_eot,
    )
}

fn dispatch_gain_control_ex(
    state: &mut GameState,
    res: &mut Resolution,
    subjects_list: Vec<DamageSourceRef>,
    to: PlayerRel,
    duration: Duration,
    untap: bool,
    haste_until_eot: bool,
) -> Option<Pending> {
    let you = res.controller;
    let options = players_of(to, state, you, res);
    match options.as_slice() {
        [] => None,
        [only] => {
            apply_gain_control_ex(
                state,
                res,
                &subjects_list,
                *only,
                duration,
                untap,
                haste_until_eot,
            );
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::GainControlExRecipient {
                subjects: subjects_list,
                duration,
                untap,
                haste_until_eot,
            });
            Some(Pending::ChoosePlayer {
                player: you,
                options,
            })
        }
    }
}

/// Apply control change + optional untap + optional haste until end of turn
/// for every surviving permanent incarnation in `subjects_list`.
pub(super) fn apply_gain_control_ex(
    state: &mut GameState,
    res: &Resolution,
    subjects_list: &[DamageSourceRef],
    new_controller: PlayerId,
    duration: Duration,
    untap: bool,
    haste_until_eot: bool,
) {
    if state.has_left(new_controller) {
        return;
    }
    let you = res.controller;
    if !temp_duration_ready(state, res, duration, you) {
        return;
    }

    let source_tied = matches!(
        duration,
        Duration::WhileSourceOnBattlefield
            | Duration::WhileSourceInZone(_)
            | Duration::WhileYouControlSource
            | Duration::WhileSourceTapped { .. }
    );
    let effect_source = source_tied.then_some(res.source);
    let timestamp = state.next_timestamp();
    let mut live_ids: SmallVec<[ObjectId; 4]> = SmallVec::new();

    for &subject in subjects_list {
        if !subjects::on_battlefield(state, subject) {
            continue;
        }
        let id = subject.object;
        live_ids.push(id);
        let filter = EffectFilter::ObjectIs(id, subject.version);
        if crate::permanent_guards::can_gain_control(state, id, new_controller)
            && state.source_duration_holds(duration, res.source, new_controller, filter)
        {
            let Some(obj) = state.object(id) else {
                continue;
            };
            let skip_noop_permanent = matches!(duration, Duration::Indefinitely)
                && obj.base_controller == new_controller
                && !state.effects.iter().any(|fx| {
                    fx.modifier == Modifier::GainControl
                        && crate::effects::applies_to(state, fx, obj)
                });
            if !skip_noop_permanent {
                state.effects.register(ContinuousEffect {
                    id: EffectId::new(0),
                    source: effect_source,
                    controller: new_controller,
                    origin: EffectOrigin::Resolution,
                    layer: Layer::Control,
                    timestamp,
                    duration,
                    filter,
                    modifier: Modifier::GainControl,
                });
            }
        }
    }

    state.refresh_characteristics();

    if untap || haste_until_eot {
        let haste_ts = haste_until_eot.then(|| state.next_timestamp());
        for &id in &live_ids {
            if !state
                .object(id)
                .is_some_and(|o| o.zone == Zone::Battlefield)
            {
                continue;
            }
            if untap && state.set_tapped(id, false) {
                state.record_tap_event(id, false, Cause::Effect);
            }
            if let Some(ts) = haste_ts {
                let filter = EffectFilter::object(state, id);
                state.effects.register(ContinuousEffect {
                    id: EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: EffectOrigin::Resolution,
                    layer: Layer::Ability,
                    timestamp: ts,
                    duration: Duration::UntilEndOfTurn,
                    filter,
                    modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                });
            }
        }
        if haste_until_eot {
            state.refresh_characteristics();
        }
    }
}
