//! Wave 4 combat & keyword resolution helpers:
//! - Ninjutsu & commander ninjutsu (`Effect::PutNinjutsuOntoBattlefield`,
//!   CR 702.49a–d, CR 508.4a, CR 508.4c, CR 508.4d)
//! - Myriad (`Effect::Myriad`, CR 702.116a–b, CR 508.4)
//! - Exert (`Effect::ExertSource` and shared `exert_permanent`, CR 701.43a–d)

use baylee_cards_dsl::{CopyCleanup, Effect, TargetSpec};
use baylee_core::ids::{Defender, EffectId, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

use super::after_entry::AfterEntry;
use super::{AwaitingOp, Flow, Resolution, resolving_ability, source_version, subjects};
use crate::choice::{ChoicePrompt, Pending, YesNoPrompt};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::{Cause, GameEvent};
use crate::object::{ObjectKind, Rider, Status};
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use crate::turn::{Phase, Step};
use crate::zone::{Zone, ZoneLocation, ZonePosition};

static EXILE_EVENT_OBJECT: &[Effect] = &[Effect::Exile {
    target: TargetSpec::EventObject,
}];

/// Interactive per-opponent state for `Effect::Myriad` (CR 702.116a).
#[derive(Clone, Debug, Hash)]
pub struct MyriadPlan {
    /// Surviving opponents other than the defending player, in APNAP turn order.
    pub opponents: Vec<PlayerId>,
    /// Next opponent index in `opponents` to ask about.
    pub index: usize,
    /// Set when the controller answered "yes" for this opponent and must now
    /// choose between attacking that opponent or one of their planeswalkers.
    pub awaiting_walker_for: Option<PlayerId>,
    /// Defenders chosen so far for accepted myriad copies, with optional
    /// planeswalker incarnation version (CR 400.7, CR 508.4a).
    pub chosen_defenders: Vec<(Defender, Option<u32>)>,
}

/// Exert `permanent` on behalf of `player` if `permanent` is currently on the
/// battlefield and phased in (CR 701.43a–c).
///
/// Registers the `DoesNotUntap` continuous effect lasting until `player`'s next
/// untap step and records [`GameEvent::PermanentExerted`] for
/// `Trigger::Exerted` and `ReflexiveEvent::ExertedThis`.
pub(crate) fn exert_permanent(
    state: &mut GameState,
    permanent: ObjectId,
    player: PlayerId,
    cause: Cause,
) -> bool {
    let Some(version) = state
        .object(permanent)
        .filter(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT))
        .map(|o| o.version)
    else {
        return false;
    };
    let filter = EffectFilter::DslAbout(
        &baylee_cards_dsl::Filter::And(&[
            baylee_cards_dsl::Filter::Reference,
            baylee_cards_dsl::Filter::ControlledByYou,
        ]),
        permanent,
        version,
    );
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(permanent),
        controller: player,
        origin: EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Modifier::DoesNotUntap.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilYourNextUntapStep,
        filter,
        modifier: baylee_cards_dsl::Modifier::DoesNotUntap,
    });
    state.journal.record(GameEvent::PermanentExerted {
        object: permanent,
        player,
        cause,
    });
    true
}

/// Executes `Effect::ExertSource` (CR 701.43a–c).
pub(super) fn exert_source(state: &mut GameState, res: &Resolution) {
    if let Some(source) =
        subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
    {
        exert_permanent(state, source.object, res.controller, Cause::Effect);
    }
}

fn in_attacking_combat(state: &GameState, controller: PlayerId) -> bool {
    state.turn.phase == Phase::Combat
        && state.turn.active == controller
        && matches!(
            state.turn.step,
            Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::CombatDamageFirst
                | Step::CombatDamage
                | Step::CombatEnd
        )
}

fn defender_still_legal(
    state: &GameState,
    controller: PlayerId,
    defender: Defender,
    target_version: Option<u32>,
) -> bool {
    if !crate::combat::defender_options(state, controller).contains(&defender) {
        return false;
    }
    match defender {
        Defender::Player(_) => true,
        Defender::Planeswalker(id) | Defender::Battle(id) => {
            target_version.is_none_or(|ver| state.object(id).is_some_and(|o| o.version == ver))
        }
    }
}

/// Executes `Effect::PutNinjutsuOntoBattlefield` (CR 702.49a, CR 702.49c, CR 702.49d).
pub(super) fn put_ninjutsu_onto_battlefield(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let subject = subjects::source(state, res).filter(|r| subjects::is_current(state, *r))?;
    let id = subject.object;
    let obj = state.object(id)?;
    let in_ninjutsu_zone = (obj.zone == Zone::Hand && obj.zone_owner == Some(you))
        || (obj.zone == Zone::Command && obj.owner == you && !obj.is_merged_component());
    if !obj.is_card() || !in_ninjutsu_zone || !obj.characteristics().types.is_permanent() {
        return None;
    }
    let defender = state.object(res.on_stack).and_then(|stack_obj| {
        stack_obj.riders.iter().find_map(|r| match r {
            Rider::NinjutsuDefender(def, ver) => Some((*def, *ver)),
            _ => None,
        })
    });
    if let Some(object) = state.object_mut(id) {
        object.kind = ObjectKind::Permanent;
        object.set_controller(you);
    }
    state.set_tapped(id, true);
    let _ = state.move_object(
        id,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    if state.entries.contains(id) || state.entries.has_after() {
        res.awaiting = Some(AwaitingOp::Entry {
            next: None,
            after: Some(AfterEntry::EnterAttacking {
                creature: id,
                defender,
            }),
        });
        return None;
    }
    finish_enter_attacking(state, you, id, defender);
    None
}

/// Places a single entered permanent into combat attacking `defender` when
/// `defender` is still a legal combat defender (CR 702.49c, CR 508.4a, CR 508.4d).
pub(super) fn finish_enter_attacking(
    state: &mut GameState,
    controller: PlayerId,
    creature: ObjectId,
    defender: Option<(Defender, Option<u32>)>,
) {
    if !in_attacking_combat(state, controller) {
        return;
    }
    let Some((def, target_ver)) = defender else {
        return;
    };
    if !defender_still_legal(state, controller, def, target_ver) {
        return;
    }
    let is_eligible_creature = state.object(creature).is_some_and(|o| {
        o.zone == Zone::Battlefield
            && !o.status.contains(Status::PHASED_OUT)
            && o.controller == controller
            && o.characteristics().types.contains(TypeSet::CREATURE)
            && !o.characteristics().types.contains(TypeSet::BATTLE)
    });
    if !is_eligible_creature {
        return;
    }
    state
        .combat
        .declare_attackers([crate::combat::AttackerInfo {
            creature,
            defending: def,
            blocked: false,
            band: None,
        }]);
    crate::combat::remember_attack(state, creature, def);
    state.board_state_changed();
}

/// Finishes entering tapped and attacking for a permanent spell whose sneak
/// cost was paid (`CR 702.190b`, `CR 506.3a`). Consumes `Rider::Sneaked` and
/// `Rider::NinjutsuDefender` so deferred and synchronous entry paths are
/// idempotent.
pub(crate) fn finish_sneak_enter_attacking(state: &mut GameState, permanent: ObjectId) {
    let Some((controller, defender)) = state.object_mut(permanent).and_then(|obj| {
        if obj.zone != Zone::Battlefield || !obj.riders.contains(&Rider::Sneaked) {
            return None;
        }
        let mut defender = None;
        obj.riders.retain(|r| match r {
            Rider::Sneaked => false,
            Rider::NinjutsuDefender(def, ver) => {
                if defender.is_none() {
                    defender = Some((*def, *ver));
                }
                false
            }
            _ => true,
        });
        Some((obj.controller, defender))
    }) else {
        return;
    };
    finish_enter_attacking(state, controller, permanent, defender);
}

/// Starts `Effect::Myriad` (CR 702.116a).
pub(super) fn start_myriad(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let defending_player = res.subject.event_player().or_else(|| {
        state.object(res.on_stack).and_then(|o| {
            o.riders.iter().find_map(|r| match r {
                Rider::EventPlayer(seat) => Some(*seat),
                _ => None,
            })
        })
    });
    let seat_count = state.players.len();
    let active = usize::from(state.turn.active.get());
    let mut opponents = Vec::new();
    for offset in 0..seat_count {
        let seat = PlayerId::new(((active + offset) % seat_count) as u8);
        if !state.has_left(seat) && state.is_opponent(seat, you) && Some(seat) != defending_player {
            opponents.push(seat);
        }
    }
    if opponents.is_empty() {
        return None;
    }
    advance_myriad(
        state,
        res,
        MyriadPlan {
            opponents,
            index: 0,
            awaiting_walker_for: None,
            chosen_defenders: Vec::new(),
        },
    )
}

/// Resumes `AwaitingOp::Myriad` from a `Pending::YesNo` answer.
pub(super) fn resume_myriad_yes_no(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if !matches!(res.awaiting, Some(AwaitingOp::Myriad(_))) {
        return None;
    }
    let Some(AwaitingOp::Myriad(mut plan)) = res.awaiting.take() else {
        unreachable!()
    };
    let you = res.controller;
    let opp = plan.opponents[plan.index];
    if !answer {
        plan.index += 1;
        let since = state.journal.last_seq();
        if let Some(pending) = advance_myriad(state, res, *plan) {
            return Some(super::next_choice(state, res, since, pending));
        }
        return Some(super::finish_choice(state, res, since));
    }
    if in_attacking_combat(state, you) && !state.has_left(opp) && state.is_opponent(opp, you) {
        let walkers = opponent_planeswalkers(state, opp);
        if !walkers.is_empty() {
            plan.awaiting_walker_for = Some(opp);
            let since = state.journal.last_seq();
            res.awaiting = Some(AwaitingOp::Myriad(plan));
            return Some(super::next_choice(
                state,
                res,
                since,
                Pending::ChooseCards {
                    player: you,
                    options: walkers,
                    min: 0,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                },
            ));
        }
    }
    plan.chosen_defenders.push((Defender::Player(opp), None));
    plan.index += 1;
    let since = state.journal.last_seq();
    if let Some(pending) = advance_myriad(state, res, *plan) {
        return Some(super::next_choice(state, res, since, pending));
    }
    Some(super::finish_choice(state, res, since))
}

/// Resumes `AwaitingOp::Myriad` from a planeswalker `Pending::ChooseCards` answer.
pub(super) fn resume_myriad_walker(
    state: &mut GameState,
    res: &mut Resolution,
    mut plan: MyriadPlan,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let opp = plan.awaiting_walker_for.take().unwrap_or(res.controller);
    let defender = match chosen.first() {
        Some(&pw) => {
            let ver = state.object(pw).map(|o| o.version);
            (Defender::Planeswalker(pw), ver)
        }
        None => (Defender::Player(opp), None),
    };
    plan.chosen_defenders.push(defender);
    plan.index += 1;
    advance_myriad(state, res, plan)
}

fn opponent_planeswalkers(state: &GameState, opponent: PlayerId) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == opponent
                    && o.characteristics().types.contains(TypeSet::PLANESWALKER)
                    && !o.characteristics().types.contains(TypeSet::BATTLE)
            })
        })
        .collect()
}

fn advance_myriad(
    state: &mut GameState,
    res: &mut Resolution,
    mut plan: MyriadPlan,
) -> Option<Pending> {
    let you = res.controller;
    while plan.index < plan.opponents.len() {
        let opp = plan.opponents[plan.index];
        if state.has_left(opp) || !state.is_opponent(opp, you) {
            plan.index += 1;
            continue;
        }
        let source = resolving_ability(state, res);
        res.awaiting = Some(AwaitingOp::Myriad(Box::new(plan)));
        return Some(Pending::YesNo {
            player: you,
            prompt: YesNoPrompt::MayDo,
            source,
        });
    }
    commit_myriad_tokens(state, res, &plan.chosen_defenders);
    None
}

fn commit_myriad_tokens(
    state: &mut GameState,
    res: &mut Resolution,
    chosen_defenders: &[(Defender, Option<u32>)],
) {
    if chosen_defenders.is_empty() {
        return;
    }
    let you = res.controller;
    let src_version = source_version(state, res)
        .or_else(|| state.object_or_departed(res.source).map(|o| o.version))
        .unwrap_or(0);
    let mut created_pairs: Vec<(ObjectId, (Defender, Option<u32>))> = Vec::new();
    for &def in chosen_defenders {
        let made = create_one_myriad_copy_batch(state, res, you);
        for &id in &made {
            state.set_tapped(id, true);
            let version = state.object(id).map_or(0, |o| o.version);
            let _ = CopyCleanup::ExileAtEndOfCombat;
            state.delayed.push(DelayedTrigger {
                controller: you,
                when: DelayedWhen::EndOfCombatThisTurn,
                action: DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: src_version,
                    effects: EXILE_EVENT_OBJECT,
                    text: res.text,
                    object: id,
                    version,
                    subject: None,
                },
            });
            created_pairs.push((id, def));
        }
    }
    if created_pairs
        .iter()
        .any(|(id, _)| state.entries.contains(*id))
        || state.entries.has_after()
    {
        for &(id, (def, ver)) in &created_pairs {
            if let Some(obj) = state.object_mut(id) {
                obj.riders.push(Rider::NinjutsuDefender(def, ver));
            }
        }
        res.awaiting = Some(AwaitingOp::Entry {
            next: None,
            after: Some(AfterEntry::FinishMyriadCombat),
        });
        return;
    }
    join_myriad_combat(state, you, &created_pairs);
}

fn create_one_myriad_copy_batch(
    state: &mut GameState,
    res: &mut Resolution,
    controller: PlayerId,
) -> Vec<ObjectId> {
    let made =
        super::tokens::create_modified_token_copies_for(state, res, controller, res.source, &[], 1);
    if !made.is_empty() {
        return made;
    }
    let Some(departed) = state.object_or_departed(res.source).map(|o| o.base.clone()) else {
        return Vec::new();
    };
    super::tokens::create_token_copies(state, controller, res.source, &departed, 1)
}

/// Finishes placing deferred myriad tokens into combat once entry choices settle.
pub(super) fn after_myriad_entry(state: &mut GameState, res: &Resolution) {
    let mut pairs = Vec::new();
    for id in state.battlefield_view() {
        let Some(obj) = state.object_mut(id) else {
            continue;
        };
        let mut found = None;
        obj.riders.retain(|r| match r {
            Rider::NinjutsuDefender(def, ver) if found.is_none() => {
                found = Some((*def, *ver));
                false
            }
            _ => true,
        });
        if let Some(def) = found {
            pairs.push((id, def));
        }
    }
    join_myriad_combat(state, res.controller, &pairs);
}

fn join_myriad_combat(
    state: &mut GameState,
    controller: PlayerId,
    pairs: &[(ObjectId, (Defender, Option<u32>))],
) {
    if !in_attacking_combat(state, controller) {
        return;
    }
    let mut infos = Vec::new();
    for &(token, (def, target_ver)) in pairs {
        if !defender_still_legal(state, controller, def, target_ver) {
            continue;
        }
        let ok = state.object(token).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(Status::PHASED_OUT)
                && o.controller == controller
                && o.characteristics().types.contains(TypeSet::CREATURE)
                && !o.characteristics().types.contains(TypeSet::BATTLE)
        });
        if !ok {
            continue;
        }
        infos.push(crate::combat::AttackerInfo {
            creature: token,
            defending: def,
            blocked: false,
            band: None,
        });
        crate::combat::remember_attack(state, token, def);
    }
    if !infos.is_empty() {
        state.combat.declare_attackers(infos);
        state.board_state_changed();
    }
}

/// Starts `Effect::Bolster(amount)` (CR 701.39a): finds the creature(s)
/// controlled by `res.controller` with the least toughness (unreduced by marked
/// damage per CR 120.6, untargeted so shroud/hexproof never interfere), places
/// `count` +1/+1 counters immediately if unique, or asks `Pending::ChooseCards`
/// on a tie.
pub(super) fn start_bolster(
    state: &mut GameState,
    res: &mut Resolution,
    amount: baylee_cards_dsl::Amount,
) -> Option<Pending> {
    let count =
        u16::try_from(super::amount2(&amount, state, res.controller, res)).unwrap_or(u16::MAX);
    if count == 0 {
        return None;
    }
    let mut least: Option<i16> = None;
    let mut options: Vec<ObjectId> = Vec::new();
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        if object.controller != res.controller
            || !object.characteristics().types.contains(TypeSet::CREATURE)
        {
            continue;
        }
        let toughness = object.characteristics().toughness.unwrap_or(0);
        if least.is_none_or(|min| toughness < min) {
            least = Some(toughness);
            options.clear();
        }
        if least == Some(toughness) {
            options.push(id);
        }
    }
    match options.as_slice() {
        [] => None,
        [only] => {
            crate::replacement::put_counters_by(
                state,
                res.controller,
                *only,
                baylee_cards_dsl::CounterKind::P1P1,
                count,
            );
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::Bolster { count });
            Some(Pending::ChooseCards {
                player: res.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
    }
}

/// Resumes `Effect::Bolster` once a tied least-toughness creature is chosen.
pub(super) fn resume_bolster(
    state: &mut GameState,
    res: &Resolution,
    count: u16,
    chosen: &[ObjectId],
) {
    if let Some(&card) = chosen.first()
        && state
            .object(card)
            .is_some_and(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT))
    {
        crate::replacement::put_counters_by(
            state,
            res.controller,
            card,
            baylee_cards_dsl::CounterKind::P1P1,
            count,
        );
    }
}

/// Executes `Effect::BecomeRenowned(count)` (CR 702.112a–c): if this ability's
/// exact source permanent is still on the battlefield, unphased, and not yet
/// renowned, places `count` +1/+1 counters on it and gives it the `Renowned`
/// designation until it leaves the battlefield.
pub(super) fn become_renowned(state: &mut GameState, res: &Resolution, count: u8) {
    let Some(subject) = subjects::source(state, res).filter(|r| subjects::is_current(state, *r))
    else {
        return;
    };
    let eligible = state.object(subject.object).is_some_and(|o| {
        o.zone == Zone::Battlefield
            && o.version == subject.version
            && !o.status.contains(Status::PHASED_OUT)
            && !o.riders.contains(&Rider::Renowned)
    });
    if !eligible {
        return;
    }
    if count > 0 {
        crate::replacement::put_counters_by(
            state,
            res.controller,
            subject.object,
            baylee_cards_dsl::CounterKind::P1P1,
            u16::from(count),
        );
    }
    if let Some(obj) = state.object_mut(subject.object)
        && obj.zone == Zone::Battlefield
        && obj.version == subject.version
        && !obj.status.contains(Status::PHASED_OUT)
        && !obj.riders.contains(&Rider::Renowned)
    {
        obj.riders.push(Rider::Renowned);
        state.board_state_changed();
    }
}

/// Starts `Effect::Exploit` (`CR 702.110a–b`): offers an optional untargeted
/// sacrifice of any creature controlled by `res.controller` (including
/// `res.source` itself if still a controlled creature on the battlefield).
pub(super) fn start_exploit(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == you
                    && state.can_be_sacrificed(id, Some(you))
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::Exploit);
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::SacrificeFor { player: you },
        total: None,
    })
}

/// Resumes `Effect::Exploit` once the controller picks `0` or `1` creature to
/// sacrifice, recording `GameEvent::CreatureExploited` with pre-sacrifice LKI.
pub(super) fn resume_exploit(state: &mut GameState, res: &Resolution, chosen: &[ObjectId]) {
    let Some(&sac_id) = chosen.first() else {
        return;
    };
    let you = res.controller;
    let legal = state.object(sac_id).is_some_and(|o| {
        o.zone == Zone::Battlefield
            && !o.status.contains(Status::PHASED_OUT)
            && o.controller == you
            && state.can_be_sacrificed(sac_id, Some(you))
            && o.characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
    });
    if !legal {
        return;
    }
    let exploiter =
        subjects::source(state, res).unwrap_or_else(|| baylee_core::ids::DamageSourceRef {
            object: res.source,
            version: state.object(res.source).map_or(0, |o| o.version),
        });
    let before_version = state.object(sac_id).map(|o| o.version);
    let departure = state.departure_snapshot(sac_id);
    let mut departure_event = departure.as_ref().map(|d| d.event.clone());
    if let Some(dep) = &mut departure_event {
        dep.sacrificed = true;
    }
    if let Some(obj) = state.object_mut(sac_id)
        && obj.is_card()
    {
        obj.kind = ObjectKind::Card;
    }
    let _ = state.sacrifice_object_with_departure(sac_id, Cause::Effect, departure);
    if state
        .object(sac_id)
        .is_none_or(|o| o.zone != Zone::Battlefield || Some(o.version) != before_version)
    {
        state.journal.record_departure(
            GameEvent::CreatureExploited {
                exploiter,
                exploited: sac_id,
                player: you,
            },
            departure_event,
        );
    }
}

/// Executes `Effect::Monstrosity(amount)` (`CR 701.37a–c`): if this ability's
/// exact source permanent is still on the battlefield, unphased, and not yet
/// monstrous, puts `N` `+1/+1` counters on it, attaches `Rider::Monstrous`,
/// and emits `GameEvent::BecameMonstrous { object, count: N }`.
pub(super) fn become_monstrous(
    state: &mut GameState,
    res: &Resolution,
    amount: baylee_cards_dsl::Amount,
) {
    let n = super::amount2(&amount, state, res.controller, res);
    let Some(subject) = subjects::source(state, res).filter(|r| subjects::is_current(state, *r))
    else {
        return;
    };
    let eligible = state.object(subject.object).is_some_and(|o| {
        o.zone == Zone::Battlefield
            && o.version == subject.version
            && !o.status.contains(Status::PHASED_OUT)
            && !o.riders.contains(&Rider::Monstrous)
    });
    if !eligible {
        return;
    }
    if n > 0 {
        crate::replacement::put_counters_by(
            state,
            res.controller,
            subject.object,
            baylee_cards_dsl::CounterKind::P1P1,
            u16::try_from(n).unwrap_or(u16::MAX),
        );
    }
    if let Some(obj) = state.object_mut(subject.object)
        && obj.zone == Zone::Battlefield
        && obj.version == subject.version
        && !obj.status.contains(Status::PHASED_OUT)
        && !obj.riders.contains(&Rider::Monstrous)
    {
        obj.riders.push(Rider::Monstrous);
        state.invalidate_projections();
        state
            .journal
            .record(crate::event::GameEvent::BecameMonstrous {
                object: subject.object,
                count: n,
            });
    }
}

/// Executes `Effect::Suspect` / `Effect::Unsuspect` (`CR 701.60a–d`).
pub(super) fn resolve_suspect(
    state: &mut GameState,
    res: &Resolution,
    target: baylee_cards_dsl::TargetSpec,
    suspect: bool,
) {
    use baylee_cards_dsl::{Filter, TargetSpec};
    let ids: Vec<ObjectId> = match target {
        TargetSpec::ThisObject => subjects::source(state, res)
            .filter(|r| subjects::is_current(state, *r))
            .map(|r| r.object)
            .into_iter()
            .collect(),
        TargetSpec::EventObject => subjects::event(state, res)
            .filter(|r| subjects::is_current(state, *r))
            .map(|r| r.object)
            .into_iter()
            .collect(),
        TargetSpec::ObjectOfFirstTargetsPlayer(_) | TargetSpec::CardInFirstTargetsGraveyard(_) => {
            res.second_targets.to_vec()
        }
        TargetSpec::Object(Filter::AttachedToBySource)
            if !res.targeted && res.targets.is_empty() =>
        {
            super::attached_host(state, res)
                .map(|(id, _)| id)
                .into_iter()
                .collect()
        }
        TargetSpec::Object(filter) if !res.targeted && res.targets.is_empty() => state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    crate::eval::matches_with_context(
                        filter,
                        state,
                        o,
                        res.controller,
                        res.rule_context(),
                    )
                })
            })
            .collect(),
        _ => res
            .targets
            .iter()
            .copied()
            .filter(|&id| {
                state.object(id).is_some_and(|obj| {
                    res.target_lki.as_ref().is_none_or(|all| {
                        all.iter()
                            .find(|lki| lki.id == id)
                            .is_none_or(|lki| lki.version == obj.version)
                    })
                })
            })
            .collect(),
    };
    let mut changed = false;
    for id in ids {
        let can_apply = state.object(id).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(Status::PHASED_OUT)
                && (!suspect || o.characteristics().types.contains(TypeSet::CREATURE))
        });
        if !can_apply {
            continue;
        }
        if let Some(obj) = state.object_mut(id) {
            if suspect {
                if !obj.riders.contains(&Rider::Suspected) {
                    obj.riders.push(Rider::Suspected);
                    changed = true;
                }
            } else if obj.riders.contains(&Rider::Suspected) {
                obj.riders.retain(|r| *r != Rider::Suspected);
                changed = true;
            }
        }
    }
    if changed {
        state.invalidate_projections();
    }
}

pub(super) fn resolve_ring_tempts_you(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let seat = usize::from(you.get());
    let count = state.players[seat].ring_temptations.saturating_add(1);
    state.players[seat].ring_temptations = count;
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == you
                    && !o.status.contains(Status::PHASED_OUT)
                    && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect();
    if options.is_empty() {
        let ring_bearer = state.players[seat].ring_bearer;
        state.journal.record(crate::event::GameEvent::RingTempted {
            player: you,
            ring_bearer,
            count,
        });
        return None;
    }
    res.awaiting = Some(AwaitingOp::ChooseRingBearer);
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::ChooseRingBearer,
        total: None,
    })
}

pub(super) fn finish_choose_ring_bearer(
    state: &mut GameState,
    res: &Resolution,
    chosen: &[ObjectId],
) {
    let you = res.controller;
    if state.has_left(you) {
        return;
    }
    let seat = usize::from(you.get());
    let picked = chosen.first().copied().filter(|&id| {
        state.object(id).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && o.controller == you
                && !o.status.contains(Status::PHASED_OUT)
                && o.characteristics().types.contains(TypeSet::CREATURE)
        })
    });
    if let Some(bearer) = picked {
        state.players[seat].ring_bearer = Some(bearer);
        state.invalidate_projections();
        state.refresh_characteristics();
    }
    let count = state.players[seat].ring_temptations;
    let ring_bearer = state.players[seat].ring_bearer;
    state.journal.record(crate::event::GameEvent::RingTempted {
        player: you,
        ring_bearer,
        count,
    });
}

/// Detains `id` on behalf of `controller` (`CR 701.35a`): until `controller`'s
/// next turn, that exact permanent incarnation can't attack or block and its
/// activated abilities can't be activated (including mana abilities).
pub(super) fn detain_object(
    state: &mut GameState,
    controller: PlayerId,
    source: ObjectId,
    id: ObjectId,
) {
    if state.has_left(controller) {
        return;
    }
    if !state
        .object(id)
        .is_some_and(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT))
    {
        return;
    }
    let filter = crate::effects::EffectFilter::object(state, id);
    let duration = baylee_cards_dsl::Duration::UntilYourNextTurn;
    for (layer, modifier) in [
        (
            baylee_cards_dsl::Layer::Ability,
            baylee_cards_dsl::Modifier::AddKeyword(
                baylee_cards_dsl::KeywordSet::CANT_ATTACK
                    .union(baylee_cards_dsl::KeywordSet::CANT_BLOCK),
            ),
        ),
        (
            baylee_cards_dsl::Layer::Text,
            baylee_cards_dsl::Modifier::CantActivate,
        ),
    ] {
        if !crate::permanent_guards::permits_resolution_effect(state, filter, modifier, controller)
        {
            continue;
        }
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(source),
            controller,
            origin: crate::effects::EffectOrigin::Resolution,
            layer,
            timestamp,
            duration,
            filter,
            modifier,
        });
    }
}

/// Resolves `Effect::Detain { target }` (`CR 701.35a`).
pub(super) fn detain_target(
    state: &mut GameState,
    res: &Resolution,
    target: baylee_cards_dsl::TargetSpec,
) {
    let you = res.controller;
    match target {
        baylee_cards_dsl::TargetSpec::ThisObject => {
            if let Some(src) = subjects::source(state, res)
                && subjects::on_battlefield(state, src)
            {
                detain_object(state, you, res.source, src.object);
            }
        }
        baylee_cards_dsl::TargetSpec::EventObject => {
            if let Some(id) = res.event_object {
                let expected = state.object(res.on_stack).and_then(|o| {
                    o.riders.iter().find_map(|r| match r {
                        Rider::EventObjectIdentity(v, _) => Some(*v),
                        _ => None,
                    })
                });
                if state
                    .object(id)
                    .is_some_and(|o| expected.is_none_or(|v| o.version == v))
                {
                    detain_object(state, you, res.source, id);
                }
            }
        }
        _ => {
            for &id in &res.targets {
                let still_current = state.object(id).is_some_and(|obj| {
                    res.target_lki.as_ref().is_none_or(|all| {
                        all.iter()
                            .find(|lki| lki.id == id)
                            .is_none_or(|lki| lki.version == obj.version)
                    })
                });
                if still_current {
                    detain_object(state, you, res.source, id);
                }
            }
        }
    }
}

/// Resolves `Effect::DetainFilter { filter }` (`CR 701.35a`).
pub(super) fn detain_filter(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static baylee_cards_dsl::Filter,
) {
    let you = res.controller;
    let context = res.rule_context();
    let targets: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|obj| {
                crate::eval::matches_with_context(filter, state, obj, you, context)
            })
        })
        .collect();
    for id in targets {
        detain_object(state, you, res.source, id);
    }
}
