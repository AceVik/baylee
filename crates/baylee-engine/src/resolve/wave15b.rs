//! Engine Wave 15b resolution operations:
//! - Entering the battlefield attached (`Effect::PutOntoBattlefieldAttached`,
//!   `Effect::CreateTokenAttached`, CR 303.4f–g, CR 301.5c, CR 301.6)
//! - Entering the battlefield attacking (`Effect::PutOntoBattlefieldAttacking`,
//!   CR 508.4, CR 506.3b, CR 508.3a)
//! - Deterministic random card choice & distinct-name choice
//!   (`Effect::ChooseCardConstrained`, `Effect::SearchLibraryDistinctNames`,
//!   CR 701.23)
//! - Optional/mandatory untargeted attachment (`Effect::AttachChosen`,
//!   CR 701.3a–b)
//! - Discarding remembered/found/defined cards (`Effect::DiscardDefined`,
//!   CR 701.9a)

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::Status;
use crate::text_changes::TextChangeMap;
use crate::zone::Zone;
use baylee_cards_dsl::{
    AbilityDef, AttachHost, ChooseMove, ChooseZone, DefinedObjects, Filter, TokenDef,
};
use baylee_core::generated::subtypes;
use baylee_core::ids::{DamageSourceRef, NameRef, TargetRef};
use baylee_core::types::TypeSet;

/// Suspended state for [`Effect::PutOntoBattlefieldAttached`] while asking for
/// a host permanent or player (CR 303.4f).
#[derive(Clone, Debug, Hash)]
pub struct PutAttachedState {
    /// Current card waiting for its host choice.
    pub current: DamageSourceRef,
    /// Remaining cards to process after `current`.
    pub remaining: Vec<DamageSourceRef>,
    /// Host selector shared across the batch.
    pub host: AttachHost,
    /// `true` puts each under its owner's control; `false` under `res.controller`.
    pub owner_control: bool,
    /// Whether each enters tapped.
    pub tapped: bool,
    /// Incarnations that entered the battlefield so far.
    pub entered: Vec<DamageSourceRef>,
}

/// Suspended state for [`Effect::ChooseCardConstrained`] when selecting cards
/// with distinct names (`distinct_names: true`) or awaiting a single pick.
#[derive(Clone, Debug, Hash)]
pub struct ChooseConstrainedState {
    /// Choosing player seat.
    pub ask_player: PlayerId,
    /// Snapshotted eligible candidates `(id, version, zone)` in deterministic order.
    pub candidates: Vec<(ObjectId, u32, Zone)>,
    /// Minimum cards required in total (clamped to legal distinct picks).
    pub min: u8,
    /// Maximum cards allowed in total.
    pub max: u8,
    /// Whether distinct names must be enforced across picks.
    pub distinct_names: bool,
    /// Whether chosen cards are explicitly revealed (`GameEvent::Revealed`).
    pub reveal_chosen: bool,
    /// Movement applied to chosen objects.
    pub movement: ChooseMove,
    /// Follow-up instructions for chosen objects.
    pub then: &'static [Effect],
    /// Originating effect instruction.
    pub op: Effect,
    /// Cards already picked in earlier distinct-name steps.
    pub picked: Vec<ObjectId>,
    /// Names already claimed by `picked` (enforces `distinct_names`).
    pub picked_names: Vec<NameRef>,
}

// -----------------------------------------------------------------------------
// Shared object / host resolution helpers
// -----------------------------------------------------------------------------

pub(super) fn resolve_defined_objects(
    state: &GameState,
    res: &Resolution,
    defined: DefinedObjects,
) -> Vec<DamageSourceRef> {
    let mut out = Vec::new();
    let push_unique = |out: &mut Vec<DamageSourceRef>, r: DamageSourceRef| {
        if !out.iter().any(|existing| existing.object == r.object) {
            out.push(r);
        }
    };
    match defined {
        DefinedObjects::Spec(spec) => match spec {
            TargetSpec::ThisObject => {
                if let Some(r) = subjects::moving_source(state, res) {
                    push_unique(&mut out, r);
                }
            }
            TargetSpec::EventObject => {
                if let Some(r) =
                    subjects::event(state, res).filter(|r| subjects::is_current(state, *r))
                {
                    push_unique(&mut out, r);
                }
            }
            other => {
                for id in zones::moving_objects(state, res, other) {
                    if let Some(r) =
                        subjects::target(state, res, id).or_else(|| state.source_identity(id))
                    {
                        push_unique(&mut out, r);
                    }
                }
            }
        },
        DefinedObjects::Found => {
            for &r in &res.subject.found {
                if state
                    .object(r.object)
                    .is_some_and(|o| o.version == r.version)
                {
                    push_unique(&mut out, r);
                }
            }
        }
        DefinedObjects::AllInZone { zone, filter } => {
            for id in super::reanimation::zone_sel_candidates(state, res, zone, filter) {
                if let Some(r) = state.source_identity(id) {
                    push_unique(&mut out, r);
                }
            }
        }
    }
    out
}

pub(super) fn resolve_spec_host(
    state: &GameState,
    res: &Resolution,
    spec: TargetSpec,
) -> Option<TargetRef> {
    let obj_ref = match spec {
        TargetSpec::ThisObject => {
            subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
        }
        TargetSpec::EventObject => {
            subjects::event(state, res).filter(|r| subjects::on_battlefield(state, *r))
        }
        _ => zones::spec_object(state, res, spec)
            .and_then(|id| subjects::target(state, res, id).or_else(|| state.source_identity(id)))
            .filter(|r| subjects::on_battlefield(state, *r)),
    };
    if let Some(r) = obj_ref {
        return Some(TargetRef::Object(r));
    }
    let player = match spec {
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..)
        | TargetSpec::AnyTarget
        | TargetSpec::PlayerOrObject(_)
        | TargetSpec::OpponentOrObject(_) => res.target_players.iter().next().or(res.chosen_player),
        TargetSpec::Player(rel) => {
            if res.targeted {
                res.target_players.iter().next().or(res.chosen_player)
            } else {
                players_of(rel, state, res.controller, res)
                    .into_iter()
                    .next()
            }
        }
        _ => None,
    };
    player
        .filter(|&p| !state.has_left(p))
        .map(TargetRef::Player)
}

fn has_reconfigure<'a>(mut abilities: impl Iterator<Item = &'a AbilityDef>) -> bool {
    abilities.any(|ability| {
        matches!(
            ability,
            AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. }
                if effects.contains(&Effect::ReconfigureAttach)
        )
    })
}

/// Check whether card `id` (which may currently be outside the battlefield) can
/// legally become attached to `host` when controlled by `controller`
/// (CR 303.4f–g, CR 301.5c, CR 301.6, CR 702.16c–d).
fn card_can_attach_as(
    state: &mut GameState,
    id: ObjectId,
    controller: PlayerId,
    host: TargetRef,
) -> bool {
    let Some((old_ctrl, old_base, is_aura, is_eq, is_fort, is_creature, is_battle, reconf)) =
        state.object(id).map(|obj| {
            let chars = obj.characteristics();
            (
                obj.controller,
                obj.base_controller,
                chars.subtypes.contains(subtypes::enchantment::AURA),
                chars.subtypes.contains(subtypes::artifact::EQUIPMENT),
                chars.subtypes.contains(subtypes::artifact::FORTIFICATION),
                chars.types.contains(TypeSet::CREATURE),
                chars.types.contains(TypeSet::BATTLE),
                has_reconfigure(obj.abilities(state.bases.as_ref()).iter()),
            )
        })
    else {
        return false;
    };
    if is_battle {
        return false;
    }
    if let Some(obj) = state.object_mut(id) {
        obj.controller = controller;
        obj.base_controller = controller;
    }
    let legal = if is_aura {
        match host {
            TargetRef::Object(h) => {
                subjects::on_battlefield(state, h)
                    && crate::aura_bindings::can_attach(state, id, h.object)
            }
            TargetRef::Player(p) => {
                !state.has_left(p) && crate::aura_bindings::can_attach_player(state, id, p)
            }
        }
    } else if is_eq || is_fort {
        match host {
            TargetRef::Object(h) if h.object != id && subjects::on_battlefield(state, h) => {
                let required = if is_eq {
                    TypeSet::CREATURE
                } else {
                    TypeSet::LAND
                };
                let creature_ok = !is_creature || (is_eq && reconf);
                creature_ok
                    && state.object(h.object).is_some_and(|recipient| {
                        recipient.characteristics().types.contains(required)
                    })
                    && !eval::protected_from(state, h.object, id)
            }
            _ => false,
        }
    } else {
        false
    };
    if let Some(obj) = state.object_mut(id) {
        obj.controller = old_ctrl;
        obj.base_controller = old_base;
    }
    legal
}

/// Check whether a prospective `TokenDef` created under `controller` can
/// legally enter attached to `host` without inserting anything into the arena
/// (CR 303.4f–g, CR 301.5c, CR 301.6, CR 702.16c–d).
fn token_can_attach(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static TokenDef,
    text: TextChangeMap,
    host: TargetRef,
) -> bool {
    let mut base = state.token_base_pt(token, None);
    if text != TextChangeMap::IDENTITY {
        let changed = std::sync::Arc::make_mut(&mut base);
        changed.colors = text.color_words(changed.colors);
        changed.subtypes = text.land_types(changed.subtypes);
        changed.keywords = text.keywords(changed.keywords);
    }
    let mut prospective =
        GameObject::new_bare(ObjectId::NO_SOURCE, controller, ObjectKind::Permanent, base);
    prospective.token = Some(token);
    if text != TextChangeMap::IDENTITY {
        let rules = crate::object::AbilityList::from_static(token.abilities, None, None)
            .with_base_text(text);
        prospective.take_abilities(rules);
    }
    let chars = prospective.characteristics();
    if chars.types.contains(TypeSet::BATTLE) {
        return false;
    }
    let is_aura = chars.subtypes.contains(subtypes::enchantment::AURA);
    if is_aura {
        return match host {
            TargetRef::Object(h) => {
                subjects::on_battlefield(state, h)
                    && crate::aura_bindings::can_attach_prospective_object(
                        state,
                        &prospective,
                        h.object,
                    )
            }
            TargetRef::Player(p) => {
                crate::aura_bindings::can_attach_prospective_player(state, &prospective, p)
            }
        };
    }
    let is_eq = chars.subtypes.contains(subtypes::artifact::EQUIPMENT);
    let is_fort = chars.subtypes.contains(subtypes::artifact::FORTIFICATION);
    if !(is_eq || is_fort) {
        return false;
    }
    let TargetRef::Object(h) = host else {
        return false;
    };
    if !subjects::on_battlefield(state, h) {
        return false;
    }
    let required = if is_eq {
        TypeSet::CREATURE
    } else {
        TypeSet::LAND
    };
    let creature_ok = !chars.types.contains(TypeSet::CREATURE)
        || (is_eq && has_reconfigure(token.abilities.iter()));
    creature_ok
        && state
            .object(h.object)
            .is_some_and(|recipient| recipient.characteristics().types.contains(required))
        && !eval::protected_from_object(state, h.object, &prospective)
}

// -----------------------------------------------------------------------------
// 1. Entering the battlefield attached
// -----------------------------------------------------------------------------

pub(super) fn put_onto_battlefield_attached(
    state: &mut GameState,
    res: &mut Resolution,
    subject: DefinedObjects,
    host: AttachHost,
    owner_control: bool,
    tapped: bool,
) -> Option<Pending> {
    let candidates = resolve_defined_objects(state, res, subject);
    let mut queue = Vec::new();
    for r in candidates {
        if state.object(r.object).is_some_and(|o| {
            o.version == r.version
                && o.is_card()
                && o.zone != Zone::Battlefield
                && o.characteristics().types.is_permanent()
        }) {
            queue.push(r);
        }
    }
    if queue.is_empty() {
        res.subject.found.clear();
        return None;
    }
    drive_put_attached(
        state,
        res,
        queue,
        host,
        owner_control,
        tapped,
        Vec::new(),
        None,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn drive_put_attached(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining: Vec<DamageSourceRef>,
    host: AttachHost,
    owner_control: bool,
    tapped: bool,
    mut entered: Vec<DamageSourceRef>,
    mut deferred_attach: Option<(ObjectId, TargetRef)>,
) -> Option<Pending> {
    while !remaining.is_empty() {
        let card = remaining.remove(0);
        let Some((entering_ctrl, is_aura)) = state
            .object(card.object)
            .filter(|o| {
                o.version == card.version
                    && o.is_card()
                    && o.zone != Zone::Battlefield
                    && o.characteristics().types.is_permanent()
            })
            .map(|o| {
                (
                    if owner_control {
                        o.owner
                    } else {
                        res.controller
                    },
                    o.characteristics()
                        .subtypes
                        .contains(subtypes::enchantment::AURA),
                )
            })
        else {
            continue;
        };
        if state.has_left(entering_ctrl) {
            continue;
        }
        match host {
            AttachHost::Spec(spec) => {
                let target_host = resolve_spec_host(state, res, spec);
                if let Some((ent_ref, def)) =
                    enter_card_attached(state, card, entering_ctrl, is_aura, tapped, target_host)
                {
                    entered.push(ent_ref);
                    if let Some(d) = def {
                        deferred_attach = Some(d);
                    }
                }
            }
            AttachHost::ChosenPermanent(filter) => {
                let candidates: Vec<ObjectId> = state.battlefield_seen().collect();
                let mut options = Vec::new();
                for id in candidates {
                    let Some(host_ref) = state.source_identity(id) else {
                        continue;
                    };
                    let filter_ok = state.object(id).is_some_and(|obj| {
                        matches_in_resolution(
                            filter,
                            state,
                            obj,
                            res.controller,
                            res,
                            Some(entering_ctrl),
                        )
                    });
                    if filter_ok
                        && card_can_attach_as(
                            state,
                            card.object,
                            entering_ctrl,
                            TargetRef::Object(host_ref),
                        )
                    {
                        options.push(id);
                    }
                }
                if options.is_empty() {
                    if !is_aura
                        && let Some((ent_ref, _)) =
                            enter_card_attached(state, card, entering_ctrl, false, tapped, None)
                    {
                        entered.push(ent_ref);
                    }
                    continue;
                }
                let prompt = if is_aura {
                    ChoicePrompt::NewHost { aura: card.object }
                } else {
                    ChoicePrompt::Generic
                };
                res.awaiting = Some(AwaitingOp::PutAttachedHost(Box::new(PutAttachedState {
                    current: card,
                    remaining,
                    host,
                    owner_control,
                    tapped,
                    entered,
                })));
                return Some(Pending::ChooseCards {
                    player: entering_ctrl,
                    options,
                    min: 1,
                    max: 1,
                    prompt,
                    total: None,
                });
            }
            AttachHost::ChosenPlayer(rel) => {
                let seats = players_of(rel, state, res.controller, res);
                let options: Vec<PlayerId> = seats
                    .into_iter()
                    .filter(|&p| {
                        !state.has_left(p)
                            && card_can_attach_as(
                                state,
                                card.object,
                                entering_ctrl,
                                TargetRef::Player(p),
                            )
                    })
                    .collect();
                if options.is_empty() {
                    if !is_aura
                        && let Some((ent_ref, _)) =
                            enter_card_attached(state, card, entering_ctrl, false, tapped, None)
                    {
                        entered.push(ent_ref);
                    }
                    continue;
                }
                res.awaiting = Some(AwaitingOp::PutAttachedHostPlayer(Box::new(
                    PutAttachedState {
                        current: card,
                        remaining,
                        host,
                        owner_control,
                        tapped,
                        entered,
                    },
                )));
                return Some(Pending::ChoosePlayer {
                    player: entering_ctrl,
                    options,
                });
            }
        }
    }
    res.subject.found = entered;
    if let Some((object, host_ref)) = deferred_attach
        && (state.entries.contains(object) || state.entries.has_after())
    {
        res.awaiting = Some(AwaitingOp::Entry {
            next: None,
            after: Some(super::after_entry::AfterEntry::AttachEntered {
                object,
                host: host_ref,
            }),
        });
    }
    None
}

pub(super) fn resume_put_attached_card(
    state: &mut GameState,
    res: &mut Resolution,
    mut pending_state: PutAttachedState,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let chosen_host = chosen
        .first()
        .and_then(|&id| state.source_identity(id))
        .filter(|r| subjects::on_battlefield(state, *r))
        .map(TargetRef::Object);
    let deferred = finish_current_put_attached(state, res, &mut pending_state, chosen_host);
    drive_put_attached(
        state,
        res,
        pending_state.remaining,
        pending_state.host,
        pending_state.owner_control,
        pending_state.tapped,
        pending_state.entered,
        deferred,
    )
}

pub(super) fn resume_put_attached_player(
    state: &mut GameState,
    res: &mut Resolution,
    mut pending_state: PutAttachedState,
    chosen: PlayerId,
) -> Option<Pending> {
    let chosen_host = (!state.has_left(chosen)).then_some(TargetRef::Player(chosen));
    let deferred = finish_current_put_attached(state, res, &mut pending_state, chosen_host);
    drive_put_attached(
        state,
        res,
        pending_state.remaining,
        pending_state.host,
        pending_state.owner_control,
        pending_state.tapped,
        pending_state.entered,
        deferred,
    )
}

fn finish_current_put_attached(
    state: &mut GameState,
    res: &Resolution,
    pending_state: &mut PutAttachedState,
    chosen_host: Option<TargetRef>,
) -> Option<(ObjectId, TargetRef)> {
    let card = pending_state.current;
    let (entering_ctrl, is_aura) = state
        .object(card.object)
        .filter(|o| {
            o.version == card.version
                && o.is_card()
                && o.zone != Zone::Battlefield
                && o.characteristics().types.is_permanent()
        })
        .map(|o| {
            (
                if pending_state.owner_control {
                    o.owner
                } else {
                    res.controller
                },
                o.characteristics()
                    .subtypes
                    .contains(subtypes::enchantment::AURA),
            )
        })?;
    if state.has_left(entering_ctrl) {
        return None;
    }
    if let Some((ent_ref, deferred)) = enter_card_attached(
        state,
        card,
        entering_ctrl,
        is_aura,
        pending_state.tapped,
        chosen_host,
    ) {
        pending_state.entered.push(ent_ref);
        return deferred;
    }
    None
}

/// Move one card onto the battlefield attached to `host`.
///
/// CR 303.4f–g: an Aura without a legal host never leaves its prior zone.
/// CR 301.5c / CR 301.6: Equipment and Fortifications enter even without a
/// legal host, and attach once on the battlefield if `host` is legal.
fn enter_card_attached(
    state: &mut GameState,
    card: DamageSourceRef,
    entering_ctrl: PlayerId,
    is_aura: bool,
    tapped: bool,
    host: Option<TargetRef>,
) -> Option<(DamageSourceRef, Option<(ObjectId, TargetRef)>)> {
    let id = card.object;
    if is_aura {
        let host_ref = host?;
        if !card_can_attach_as(state, id, entering_ctrl, host_ref) {
            return None;
        }
    }
    let prior = state.object(id).map(|o| {
        (
            o.zone,
            o.version,
            o.controller,
            o.base_controller,
            o.kind,
            o.status.contains(Status::TAPPED),
        )
    })?;
    let expected_ver = prior.1.wrapping_add(1);
    if let Some(obj) = state.object_mut(id) {
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(entering_ctrl);
        if is_aura {
            match host {
                Some(TargetRef::Object(h)) => {
                    obj.set_attached_player(None);
                    obj.attached_to = Some(h.object);
                }
                Some(TargetRef::Player(p)) => {
                    obj.set_attached_player(Some(p));
                }
                None => {}
            }
        }
    }
    state.set_tapped(id, tapped);
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    let entered = state.object(id).is_some_and(|o| {
        (o.zone == Zone::Battlefield && o.version == expected_ver) || state.entries.contains(id)
    });
    if !entered {
        if state
            .object(id)
            .is_some_and(|o| o.zone == prior.0 && o.version == prior.1)
        {
            state.restore_tapped(id, prior.5);
            if let Some(obj) = state.object_mut(id) {
                obj.attached_to = None;
                obj.set_attached_player(None);
                obj.kind = prior.4;
                obj.controller = prior.2;
                obj.base_controller = prior.3;
            }
        }
        return None;
    }
    let ent_ref = crate::entry::battlefield_reference(state, id)?;
    let mut deferred = None;
    if !is_aura && let Some(host_ref) = host {
        if state.entries.contains(id) {
            deferred = Some((id, host_ref));
        } else if super::attachments::can_attach_to_recipient(state, ent_ref, host_ref) {
            super::attachments::attach_selected(state, &[ent_ref], host_ref);
        }
    }
    Some((ent_ref, deferred))
}

pub(super) fn after_entry_attach(state: &mut GameState, object: ObjectId, host: TargetRef) {
    if let Some(att_ref) = state.source_identity(object)
        && super::attachments::can_attach_to_recipient(state, att_ref, host)
    {
        super::attachments::attach_selected(state, &[att_ref], host);
    }
}

pub(super) fn create_token_attached(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    amount: Amount,
    host: AttachHost,
) -> Option<Pending> {
    let you = res.controller;
    let count = amount2(&amount, state, you, res);
    if count == 0 || state.has_left(you) {
        res.subject.found.clear();
        return None;
    }
    let is_aura = token.subtypes.contains(&subtypes::enchantment::AURA);
    match host {
        AttachHost::Spec(spec) => {
            let target_host = resolve_spec_host(state, res, spec);
            finish_create_token_attached(state, res, token, count, is_aura, target_host);
            None
        }
        AttachHost::ChosenPermanent(filter) => {
            let candidates: Vec<ObjectId> = state.battlefield_seen().collect();
            let mut options = Vec::new();
            for id in candidates {
                let Some(host_ref) = state.source_identity(id) else {
                    continue;
                };
                let filter_ok = state.object(id).is_some_and(|obj| {
                    matches_in_resolution(filter, state, obj, you, res, Some(you))
                });
                if filter_ok
                    && token_can_attach(state, you, token, res.text, TargetRef::Object(host_ref))
                {
                    options.push(id);
                }
            }
            if options.is_empty() {
                finish_create_token_attached(state, res, token, count, is_aura, None);
                return None;
            }
            res.awaiting = Some(AwaitingOp::CreateTokenAttached {
                token,
                count,
                is_aura,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        AttachHost::ChosenPlayer(rel) => {
            let seats = players_of(rel, state, you, res);
            let mut options = Vec::new();
            for p in seats {
                if !state.has_left(p)
                    && token_can_attach(state, you, token, res.text, TargetRef::Player(p))
                {
                    options.push(p);
                }
            }
            if options.is_empty() {
                finish_create_token_attached(state, res, token, count, is_aura, None);
                return None;
            }
            res.awaiting = Some(AwaitingOp::CreateTokenAttachedPlayer {
                token,
                count,
                is_aura,
            });
            Some(Pending::ChoosePlayer {
                player: you,
                options,
            })
        }
    }
}

pub(super) fn resume_create_token_attached_card(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    count: u32,
    is_aura: bool,
    chosen: &[ObjectId],
) {
    let chosen_host = chosen
        .first()
        .and_then(|&id| state.source_identity(id))
        .filter(|r| subjects::on_battlefield(state, *r))
        .map(TargetRef::Object);
    finish_create_token_attached(state, res, token, count, is_aura, chosen_host);
}

pub(super) fn resume_create_token_attached_player(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    count: u32,
    is_aura: bool,
    chosen: PlayerId,
) {
    let chosen_host = (!state.has_left(chosen)).then_some(TargetRef::Player(chosen));
    finish_create_token_attached(state, res, token, count, is_aura, chosen_host);
}

fn finish_create_token_attached(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    count: u32,
    is_aura: bool,
    host: Option<TargetRef>,
) {
    let you = res.controller;
    if count == 0 || state.has_left(you) {
        res.subject.found.clear();
        return;
    }
    let legal_host = host.filter(|&h| token_can_attach(state, you, token, res.text, h));
    if is_aura && legal_host.is_none() {
        // CR 303.4g: an Aura token with nothing legal to enchant is not created.
        res.subject.found.clear();
        return;
    }
    let made =
        super::tokens::create_tokens_attached(state, you, token, count, res.text, legal_host);
    super::tokens::remember_created_tokens(state, res, &made);
}

// -----------------------------------------------------------------------------
// 2. Entering the battlefield tapped and attacking (CR 508.4, CR 506.3b)
// -----------------------------------------------------------------------------

pub(super) fn put_onto_battlefield_attacking(
    state: &mut GameState,
    res: &mut Resolution,
    subject: DefinedObjects,
    owner_control: bool,
    tapped: bool,
) -> Option<Pending> {
    let candidates = resolve_defined_objects(state, res, subject);
    let mut entered_ids = Vec::new();
    let mut entered_refs = Vec::new();
    for card in candidates {
        let id = card.object;
        let Some((prior_zone, prior_ver, prior_ctrl, prior_base, prior_kind, prior_tapped, owner)) =
            state
                .object(id)
                .filter(|o| {
                    o.version == card.version
                        && o.is_card()
                        && o.zone != Zone::Battlefield
                        && o.characteristics().types.is_permanent()
                })
                .map(|o| {
                    (
                        o.zone,
                        o.version,
                        o.controller,
                        o.base_controller,
                        o.kind,
                        o.status.contains(Status::TAPPED),
                        o.owner,
                    )
                })
        else {
            continue;
        };
        let entering_ctrl = if owner_control { owner } else { res.controller };
        if state.has_left(entering_ctrl) {
            continue;
        }
        let expected_ver = prior_ver.wrapping_add(1);
        if let Some(obj) = state.object_mut(id) {
            obj.kind = ObjectKind::Permanent;
            obj.set_controller(entering_ctrl);
        }
        state.set_tapped(id, tapped);
        let _ = state.move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
        let entered = state.object(id).is_some_and(|o| {
            (o.zone == Zone::Battlefield && o.version == expected_ver) || state.entries.contains(id)
        });
        if !entered {
            if state
                .object(id)
                .is_some_and(|o| o.zone == prior_zone && o.version == prior_ver)
            {
                state.restore_tapped(id, prior_tapped);
                if let Some(obj) = state.object_mut(id) {
                    obj.kind = prior_kind;
                    obj.controller = prior_ctrl;
                    obj.base_controller = prior_base;
                }
            }
            continue;
        }
        entered_ids.push(id);
        if let Some(r) = crate::entry::battlefield_reference(state, id) {
            entered_refs.push(r);
        }
    }
    res.subject.found = entered_refs;
    if entered_ids.is_empty() {
        return None;
    }
    if entered_ids.iter().any(|id| state.entries.contains(*id)) || state.entries.has_after() {
        res.awaiting = Some(AwaitingOp::Entry {
            next: None,
            after: Some(super::after_entry::AfterEntry::FinishAttackingFound),
        });
        return None;
    }
    super::attacking_tokens::start_for_active(state, res, entered_ids)
}

pub(super) fn after_entry_attacking_found(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let ids: Vec<ObjectId> = res
        .subject
        .found
        .iter()
        .filter(|r| subjects::on_battlefield(state, **r))
        .map(|r| r.object)
        .collect();
    super::attacking_tokens::start_for_active(state, res, ids)
}

// -----------------------------------------------------------------------------
// 3 & 4. Deterministic random choice & distinct-name choice / search
// -----------------------------------------------------------------------------

#[allow(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines
)]
pub(super) fn choose_card_constrained(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
    zone: ChooseZone,
    filter: &'static Filter,
    min: u8,
    max: Amount,
    at_random: bool,
    distinct_names: bool,
    reveal_chosen: bool,
    movement: ChooseMove,
    then: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let you = res.controller;
    let Some(ask_player) =
        super::wave12a::apnap_seats_of(state, players_of(chooser, state, you, res))
            .into_iter()
            .find(|&p| !state.has_left(p))
    else {
        clear_found_and_event(res);
        return None;
    };
    let candidates =
        super::wave13a::collect_choose_zone_candidates(state, res, zone, filter, ask_player);
    if candidates.is_empty() {
        clear_found_and_event(res);
        return None;
    }
    let max_n = amount2(&max, state, you, res);
    let max_u8 = u8::try_from(max_n).unwrap_or(u8::MAX);
    if max_u8 == 0 {
        return finish_constrained(
            state,
            res,
            ask_player,
            candidates,
            reveal_chosen,
            movement,
            then,
            op,
            &[],
        );
    }
    if at_random {
        // Pure deterministic selection using `state.rng` ONLY.
        let mut pool: Vec<ObjectId> = candidates.iter().map(|&(id, _, _)| id).collect();
        state.rng.shuffle(&mut pool);
        let mut picked = Vec::new();
        let mut seen_names: Vec<NameRef> = Vec::new();
        for id in pool {
            if picked.len() >= usize::from(max_u8) {
                break;
            }
            if distinct_names {
                let Some(name) = state.object(id).map(|o| o.characteristics().name) else {
                    continue;
                };
                if seen_names.contains(&name) {
                    continue;
                }
                seen_names.push(name);
            }
            picked.push(id);
        }
        return finish_constrained(
            state,
            res,
            ask_player,
            candidates,
            reveal_chosen,
            movement,
            then,
            op,
            &picked,
        );
    }

    // Count distinct names so `min`/`max` never ask for more distinct names than exist.
    let mut unique_names: Vec<NameRef> = Vec::new();
    let mut has_duplicate_name = false;
    for &(id, _, _) in &candidates {
        if let Some(name) = state.object(id).map(|o| o.characteristics().name) {
            if unique_names.contains(&name) {
                has_duplicate_name = true;
            } else {
                unique_names.push(name);
            }
        }
    }
    let cap = if distinct_names {
        u8::try_from(unique_names.len()).unwrap_or(u8::MAX)
    } else {
        u8::try_from(candidates.len()).unwrap_or(u8::MAX)
    };
    let max_clamped = max_u8.min(cap);
    let min_clamped = min.min(max_clamped);
    let prompt = super::wave13a::choose_prompt_for(zone, movement);

    // When `distinct_names` is true, `max_clamped > 1`, and duplicate names exist
    // among the candidates, step through one card at a time so the player may
    // choose any specific copy while already-claimed names are excluded from all
    // subsequent steps.
    if distinct_names && max_clamped > 1 && has_duplicate_name {
        let options: Vec<ObjectId> = candidates.iter().map(|&(id, _, _)| id).collect();
        let step_min = u8::from(min_clamped > 0);
        res.awaiting = Some(AwaitingOp::ChooseCardConstrained(Box::new(
            ChooseConstrainedState {
                ask_player,
                candidates,
                min: min_clamped,
                max: max_clamped,
                distinct_names: true,
                reveal_chosen,
                movement,
                then,
                op,
                picked: Vec::new(),
                picked_names: Vec::new(),
            },
        )));
        return Some(Pending::ChooseCards {
            player: ask_player,
            options,
            min: step_min,
            max: 1,
            prompt,
            total: None,
        });
    }

    let options: Vec<ObjectId> = candidates.iter().map(|&(id, _, _)| id).collect();
    res.awaiting = Some(AwaitingOp::ChooseCardConstrained(Box::new(
        ChooseConstrainedState {
            ask_player,
            candidates,
            min: min_clamped,
            max: max_clamped,
            distinct_names: false,
            reveal_chosen,
            movement,
            then,
            op,
            picked: Vec::new(),
            picked_names: Vec::new(),
        },
    )));
    Some(Pending::ChooseCards {
        player: ask_player,
        options,
        min: min_clamped,
        max: max_clamped,
        prompt,
        total: None,
    })
}

pub(super) fn resume_choose_card_constrained(
    state: &mut GameState,
    res: &mut Resolution,
    mut spec: ChooseConstrainedState,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if !spec.distinct_names {
        return finish_constrained(
            state,
            res,
            spec.ask_player,
            spec.candidates,
            spec.reveal_chosen,
            spec.movement,
            spec.then,
            spec.op,
            chosen,
        );
    }
    // Step-by-step distinct-name mode: empty answer means player stopped early.
    if let Some(&id) = chosen.first() {
        if !spec.picked.contains(&id)
            && let Some(name) = state.object(id).map(|o| o.characteristics().name)
            && !spec.picked_names.contains(&name)
        {
            spec.picked.push(id);
            spec.picked_names.push(name);
        }
    } else {
        return finish_constrained(
            state,
            res,
            spec.ask_player,
            spec.candidates,
            spec.reveal_chosen,
            spec.movement,
            spec.then,
            spec.op,
            &spec.picked,
        );
    }

    if spec.picked.len() >= usize::from(spec.max) {
        return finish_constrained(
            state,
            res,
            spec.ask_player,
            spec.candidates,
            spec.reveal_chosen,
            spec.movement,
            spec.then,
            spec.op,
            &spec.picked,
        );
    }

    let remaining_options: Vec<ObjectId> = spec
        .candidates
        .iter()
        .filter_map(|&(id, ver, z)| {
            let obj = state.object(id)?;
            if obj.version != ver || obj.zone != z || spec.picked.contains(&id) {
                return None;
            }
            let name = obj.characteristics().name;
            (!spec.picked_names.contains(&name)).then_some(id)
        })
        .collect();
    if remaining_options.is_empty() {
        return finish_constrained(
            state,
            res,
            spec.ask_player,
            spec.candidates,
            spec.reveal_chosen,
            spec.movement,
            spec.then,
            spec.op,
            &spec.picked,
        );
    }

    let step_min = u8::from(spec.picked.len() < usize::from(spec.min));
    let prompt = match spec.op {
        Effect::ChooseCardConstrained { zone, movement, .. } => {
            super::wave13a::choose_prompt_for(zone, movement)
        }
        _ => ChoicePrompt::Generic,
    };
    let ask_player = spec.ask_player;
    res.awaiting = Some(AwaitingOp::ChooseCardConstrained(Box::new(spec)));
    Some(Pending::ChooseCards {
        player: ask_player,
        options: remaining_options,
        min: step_min,
        max: 1,
        prompt,
        total: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn finish_constrained(
    state: &mut GameState,
    res: &mut Resolution,
    ask_player: PlayerId,
    candidates: Vec<(ObjectId, u32, Zone)>,
    reveal_chosen: bool,
    movement: ChooseMove,
    then: &'static [Effect],
    op: Effect,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let delegate = super::wave13a::ChooseCardExState {
        ask_player,
        candidates,
        reveal_chosen,
        movement,
        not_chosen_movement: ChooseMove::None,
        then,
        for_unchosen: &[],
        op,
    };
    super::wave13a::choose_card_ex_finish(state, res, delegate, chosen)
}

fn clear_found_and_event(res: &mut Resolution) {
    res.subject.found.clear();
    res.event_object = None;
    res.subject.set_event_identity(None);
}

pub(super) fn search_library_distinct_names(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    count: Amount,
    find: &'static baylee_cards_dsl::Find,
) -> Option<Pending> {
    let you = res.controller;
    let count = u8::try_from(amount2(&count, state, you, res)).unwrap_or(u8::MAX);
    super::search::begin_search(
        state,
        res,
        super::search::Search {
            library: you,
            searcher: you,
            filter,
            bound: None,
            finds: core::slice::from_ref(find),
            count: Some(count),
            distinct_names: true,
            split: None,
            optional: true,
        },
    )
}

// -----------------------------------------------------------------------------
// 5a. Optional / mandatory attach chosen (CR 701.3a–b)
// -----------------------------------------------------------------------------

pub(super) fn attach_chosen(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    to: TargetSpec,
    max: Amount,
    optional: bool,
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let host = resolve_spec_host(state, res, to)?;
    let max_n = amount2(&max, state, you, res);
    if max_n == 0 {
        return None;
    }
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            let Some(att_ref) = state.source_identity(id) else {
                return false;
            };
            super::attachments::can_attach_to_recipient(state, att_ref, host)
                && state.object(id).is_some_and(|obj| {
                    matches_in_resolution(filter, state, obj, you, res, Some(you))
                })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    let avail = u8::try_from(options.len()).unwrap_or(u8::MAX);
    let max_clamped = u8::try_from(max_n).unwrap_or(u8::MAX).min(avail);
    let min = if optional { 0 } else { max_clamped };
    res.awaiting = Some(AwaitingOp::AttachChosen { host });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min,
        max: max_clamped,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

pub(super) fn resume_attach_chosen(state: &mut GameState, host: TargetRef, chosen: &[ObjectId]) {
    let refs: Vec<DamageSourceRef> = chosen
        .iter()
        .filter_map(|&id| state.source_identity(id))
        .collect();
    super::attachments::attach_selected(state, &refs, host);
}

// -----------------------------------------------------------------------------
// 5b. Discard defined / found / remembered cards (CR 701.9a)
// -----------------------------------------------------------------------------

pub(super) fn discard_defined(
    state: &mut GameState,
    res: &mut Resolution,
    cards: DefinedObjects,
) -> Option<Pending> {
    let resolved = resolve_defined_objects(state, res, cards);
    let discards: Vec<(ObjectId, PlayerId)> = resolved
        .into_iter()
        .filter_map(|r| {
            let obj = state.object(r.object)?;
            if obj.version != r.version || obj.zone != Zone::Hand {
                return None;
            }
            let holder = obj.zone_owner.unwrap_or(obj.owner);
            (!state.has_left(holder)).then_some((r.object, holder))
        })
        .collect();
    if discards.is_empty() {
        return None;
    }
    super::discard::discard_or_ask(state, res, &discards)
}
