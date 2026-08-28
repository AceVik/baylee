//! Effect resolution: the op interpreter.
//!
//! Spells and abilities resolve by running their [`Effect`] list through a
//! small continuation machine: operations that need a player choice
//! (searches, scry) suspend into a `Pending::ChooseCards` and resume on the
//! answer. Everything runs through the normal event pipeline, so the
//! journal stays complete.

use crate::choice::{ChoicePrompt, Pending, YesNoPrompt};
use crate::eval;
use crate::event::{Cause, DamageTarget, GameEvent};
use crate::object::{Characteristics, GameObject, ObjectKind, Status};
use crate::sba;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Amount, Effect, PlayerRel, SearchDest, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::SubtypeSet;
use smallvec::SmallVec;

/// A running effect resolution (continuation).
#[derive(Clone, Debug)]
pub struct Resolution {
    /// The source permanent/spell of the effect.
    pub source: ObjectId,
    /// The stack object being resolved (spell or ability).
    pub on_stack: ObjectId,
    /// Controlling player.
    pub controller: PlayerId,
    /// Flattened effect operations.
    pub effects: Vec<Effect>,
    /// Program counter.
    pub pc: usize,
    /// Targets chosen at cast/activation.
    pub targets: SmallVec<[ObjectId; 2]>,
    /// X value, if any.
    pub x: Option<u32>,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// The suspended choice, if any.
    pub awaiting: Option<AwaitingOp>,
}

/// An operation suspended on a player choice.
#[derive(Clone, Copy, Debug)]
pub enum AwaitingOp {
    /// A library search: chosen card goes to `dest`.
    SearchLibrary {
        /// Destination.
        dest: SearchDest,
        /// Enters tapped (battlefield only).
        tapped: bool,
        /// Shuffle afterwards.
        shuffle: bool,
    },
    /// Scry: chosen cards go to the bottom, the rest stays on top.
    Scry {
        /// How many cards were looked at.
        looked: u8,
    },
    /// Chosen hand cards go on top of the library in chosen order.
    PutBackOnTop,
    /// A mana color choice (choice-restricted mana abilities).
    ManaChoice {
        /// Allowed colors.
        colors: &'static [ManaColor],
        /// Mana still to choose (combination lands pick per mana).
        remaining: u16,
        /// Mana added per pick (1 for combination, all for single-choice).
        per_pick: u16,
    },
    /// "You may pay N life; if you don't, this enters tapped".
    PayLifeOrTapSelf {
        /// Life to pay.
        amount: u16,
    },
}

/// Amount evaluation with target context ([`Amount::TargetPower`]).
fn amount2(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
    x: Option<u32>,
    targets: &[ObjectId],
) -> u32 {
    match amount {
        Amount::TargetPower => targets
            .first()
            .and_then(|t| state.object(*t))
            .and_then(|o| o.characteristics().power)
            .map_or(0, |p| p.max(0) as u32),
        other => eval::amount(other, state, you, this, x),
    }
}

/// Flattens nested `Sequence`s into one flat op list.
#[must_use]
pub fn flatten(effects: &'static [Effect]) -> Vec<Effect> {
    fn go(e: &Effect, out: &mut Vec<Effect>) {
        match e {
            Effect::Sequence(parts) => {
                for p in *parts {
                    go(p, out);
                }
            }
            other => out.push(*other),
        }
    }
    let mut out = Vec::new();
    for e in effects {
        go(e, &mut out);
    }
    out
}

/// What the resolution machine produced.
#[derive(Debug)]
pub enum Flow {
    /// All operations are done.
    Complete,
    /// Suspended: a choice is required (pending is set by the caller).
    Wait(Pending),
}

/// Runs a resolution until it completes or suspends on a choice.
#[must_use]
pub fn run(state: &mut GameState, res: &mut Resolution) -> Flow {
    while res.pc < res.effects.len() {
        let op = res.effects[res.pc];
        if let Some(pending) = exec(state, res, op) {
            return Flow::Wait(pending);
        }
        res.pc += 1;
    }
    Flow::Complete
}

/// Resumes a color choice suspended on [`AwaitingOp::ManaChoice`].
///
/// # Panics
/// When the suspended operation is not a mana choice.
#[must_use]
pub fn resume_with_color(state: &mut GameState, res: &mut Resolution, color: ManaColor) -> Flow {
    let AwaitingOp::ManaChoice {
        colors,
        remaining,
        per_pick,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_with_color on non-mana choice");
    };
    debug_assert!(colors.contains(&color));
    state.players[res.controller.get() as usize]
        .mana_pool
        .add(color, per_pick);
    state.journal.record(GameEvent::ManaProduced {
        player: res.controller,
        color,
        amount: per_pick,
        source: Some(res.source),
    });
    if remaining > 1 {
        res.awaiting = Some(AwaitingOp::ManaChoice {
            colors,
            remaining: remaining - 1,
            per_pick,
        });
        return Flow::Wait(Pending::ChooseColor {
            player: res.controller,
            options: colors.to_vec(),
        });
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes a yes/no choice (shockland payment and friends).
///
/// # Panics
/// When the suspended operation is not a yes/no choice.
#[must_use]
pub fn resume_yes_no(state: &mut GameState, res: &mut Resolution, answer: bool) -> Flow {
    let AwaitingOp::PayLifeOrTapSelf { amount } =
        res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_yes_no on non-yes/no choice");
    };
    if answer {
        let p = &mut state.players[res.controller.get() as usize];
        let old = p.life;
        p.life -= i32::from(amount);
        let new = p.life;
        state.journal.record(GameEvent::LifeChanged {
            player: res.controller,
            old,
            new,
            cause: Cause::Effect,
        });
    } else if let Some(obj) = state.object_mut(res.source) {
        obj.status.insert(Status::TAPPED);
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes a suspended resolution with the chosen cards.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
pub fn resume(state: &mut GameState, res: &mut Resolution, chosen: &[ObjectId]) -> Flow {
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    match awaiting {
        AwaitingOp::SearchLibrary {
            dest,
            tapped,
            shuffle,
        } => {
            for &card in chosen {
                match dest {
                    SearchDest::Hand => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Hand(res.controller),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::TopOfLibrary => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Library(res.controller),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::Battlefield => {
                        if let Some(obj) = state.object_mut(card) {
                            obj.kind = ObjectKind::Permanent;
                            if tapped {
                                obj.status.insert(Status::TAPPED);
                            }
                        }
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                }
            }
            if shuffle {
                state.shuffle_library(res.controller);
            }
        }
        AwaitingOp::Scry { .. } => {
            // Chosen cards go to the bottom in chosen order; the rest stays
            // on top in its original relative order (scry approximation).
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::PutBackOnTop => {
            // Chosen cards go on top in chosen order (last chosen = top).
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::ManaChoice { .. } | AwaitingOp::PayLifeOrTapSelf { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
    }
    res.pc += 1;
    run(state, res)
}

/// Executes one operation; returns `Some(pending)` when it suspends.
fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::SearchLibrary { .. } | Effect::Scry { .. } | Effect::PutFromHandOnTop { .. } => {
            exec_choice(state, res, op)
        }
        _ => exec_immediate(state, res, op),
    }
}

/// Operations that suspend on a player choice.
#[allow(clippy::too_many_lines)] // the choice-op dispatch table is naturally flat
fn exec_choice(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::SearchLibrary {
            filter,
            dest,
            tapped,
            shuffle,
            optional,
        } => {
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
                .collect();
            if options.is_empty() {
                // Hidden zone: failing to find is always legal (CR 701.19).
                if shuffle {
                    state.shuffle_library(you);
                }
                return None;
            }
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                dest,
                tapped,
                shuffle,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: u8::from(!optional),
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            })
        }
        Effect::Scry { amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Scry {
                looked: looked.len() as u8,
            });
            Some(Pending::ChooseCards {
                player: you,
                options: looked,
                min: 0,
                max: n as u8,
                prompt: ChoicePrompt::ScryBottom,
            })
        }
        Effect::PutFromHandOnTop { count } => {
            let hand = state.zones.list(ZoneLocation::Hand(you)).clone();
            let n = (count as usize).min(hand.len());
            if n == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::PutBackOnTop);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: n as u8,
                max: n as u8,
                prompt: ChoicePrompt::PutBackOnTop,
            })
        }
        Effect::AddManaChoice {
            colors,
            amount,
            combination,
        } => {
            let per_pick = if combination { 1 } else { amount };
            res.awaiting = Some(AwaitingOp::ManaChoice {
                colors,
                remaining: amount,
                per_pick,
            });
            Some(Pending::ChooseColor {
                player: you,
                options: colors.to_vec(),
            })
        }
        Effect::PayLifeOrEnterTapped { amount } => {
            // Not payable at all → no choice, enters tapped (CR 614.1c).
            if state.players[you.get() as usize].life <= i32::from(amount) {
                if let Some(obj) = state.object_mut(res.source) {
                    obj.status.insert(Status::TAPPED);
                }
                return None;
            }
            res.awaiting = Some(AwaitingOp::PayLifeOrTapSelf { amount });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
            })
        }
        _ => unreachable!("not a choice op"),
    }
}

/// Operations that complete immediately.
#[allow(clippy::too_many_lines)] // the op vocabulary is naturally one flat dispatch table
fn exec_immediate(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::Sequence(_) => unreachable!("sequences are flattened"),
        Effect::GainLife { amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::GainLifeFor { amount, who } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            let players = match who {
                PlayerRel::ControllerOfTarget => res
                    .targets
                    .first()
                    .and_then(|t| state.object(*t))
                    .map_or_else(Vec::new, |o| vec![o.controller]),
                other => eval::players(other, state, you),
            };
            for player in players {
                gain_life(state, player, n);
            }
            None
        }
        Effect::Exile { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
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
            None
        }
        Effect::AddCounter { kind, amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            let target_id = res.targets.first().copied().unwrap_or(res.source);
            // Counter-placement replacements (Doubling Season, CR 614.2).
            let mut n_total = n;
            if let Some(target_obj) = state.object(target_id) {
                for entry in &state.replacement_rules {
                    if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement {
                        object_filter,
                    } = entry.rule
                        && eval::matches(
                            object_filter,
                            state,
                            target_obj,
                            entry.controller,
                            entry.source,
                        )
                    {
                        n_total = n_total.saturating_mul(2);
                    }
                }
            }
            if let Some(obj) = state.object_mut(target_id) {
                let old = obj.counters.get(kind);
                let new = obj.counters.add(kind, n_total);
                state.journal.record(GameEvent::CounterChanged {
                    object: target_id,
                    kind,
                    old,
                    new,
                });
            }
            None
        }
        Effect::ReturnToHand { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnAllToHand {
            filter,
            opponents_only,
        } => {
            let all: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        (!opponents_only || o.controller != you)
                            && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
                .collect();
            for id in all {
                let owner = state.object(id).map_or(you, |o| o.owner);
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
            None
        }
        Effect::DestroyAll { filter } => {
            let all: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
                .collect();
            for id in all {
                sba::destroy(state, id);
            }
            None
        }
        Effect::ExileGraveyard { player } => {
            for player in eval::players(player, state, you) {
                let cards: Vec<ObjectId> =
                    state.zones.list(ZoneLocation::Graveyard(player)).clone();
                for card in cards {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::GraveyardToTop { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Library(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::GraveyardToBattlefield { .. } => {
            if let Some(&target_id) = res.targets.first() {
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    obj.controller = you;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::CreateContinuousEffect {
            layer,
            filter,
            modifier,
            duration,
        } => {
            let filter = if matches!(filter, baylee_cards_dsl::Filter::This) {
                crate::effects::EffectFilter::ObjectIs(
                    res.targets.first().copied().unwrap_or(res.source),
                )
            } else {
                crate::effects::EffectFilter::Dsl(filter)
            };
            let timestamp = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer,
                timestamp,
                duration,
                filter,
                modifier,
            });
            None
        }
        Effect::CreateToken { token } => {
            // Token-creation replacements (Doubling Season, CR 614.1).
            let mut count = 1u32;
            if let Some(source_obj) = state.object(res.source) {
                for entry in &state.replacement_rules {
                    if let baylee_cards_dsl::ReplacementRule::DoubleTokenCreation {
                        controller_filter,
                    } = entry.rule
                        && eval::matches(
                            controller_filter,
                            state,
                            source_obj,
                            res.controller,
                            entry.source,
                        )
                    {
                        count *= 2;
                    }
                }
            }
            for _ in 0..count {
                create_one_token(state, res.controller, token);
            }
            None
        }
        Effect::ChangeController { new_controller } => {
            if let Some(&target_id) = res.targets.first() {
                // Control-change ops always favor the effect's controller
                // (Gilded Drake-style exchanges get a dedicated op in S7).
                let _ = new_controller;
                change_controller(state, target_id, you);
            }
            None
        }
        Effect::PhaseOut { target } => {
            let target_id = match target {
                Some(_) => res.targets.first().copied(),
                None => Some(res.source),
            };
            if let Some(id) = target_id {
                if let Some(obj) = state.object_mut(id) {
                    obj.status.insert(Status::PHASED_OUT);
                }
                state.journal.record(GameEvent::PhaseChanged {
                    object: id,
                    phased_out: true,
                });
            }
            None
        }
        Effect::ExileLinked { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                    obj.riders
                        .push(crate::object::Rider::Linked { host: res.source });
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::SacrificeSelf => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::PumpFilter {
            filter,
            power,
            toughness,
            duration,
        } => {
            let signed = |a: &Amount| -> i16 {
                let v = amount2(a, state, you, res.source, res.x, &res.targets) as i16;
                if matches!(a, Amount::NegX) { -v } else { v }
            };
            let p = signed(&power);
            let t = signed(&toughness);
            let ts = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer: baylee_cards_dsl::Layer::PtModify,
                timestamp: ts,
                duration,
                filter: crate::effects::EffectFilter::Dsl(filter),
                modifier: baylee_cards_dsl::Modifier::ModifyPT(p, t),
            });
            None
        }
        Effect::CreateTokenFromLinked { token } => {
            // The exiled card's owner creates the token; its power and
            // toughness are the exiled card's mana value.
            let mut owner = None;
            let mut cmc = 0;
            'scan: for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in state.zones.list(ZoneLocation::Exile(p)) {
                    if state.object(card).is_some_and(|o| {
                        o.riders
                            .iter()
                            .any(|r| matches!(r, crate::object::Rider::Linked { host } if *host == res.source))
                    }) {
                        owner = Some(p);
                        cmc = state
                            .object(card)
                            .map_or(0, |o| o.characteristics().mana_cost.cmc());
                        break 'scan;
                    }
                }
            }
            if let Some(owner) = owner {
                let mut def = *token;
                def.power = Some(cmc as i16);
                def.toughness = Some(cmc as i16);
                create_one_token(state, owner, &def);
            }
            None
        }
        Effect::ReturnLinkedToBattlefield => {
            // Everything exiled with a link to the source returns under its
            // owner's control (Skyclave Apparition & co.).
            let mut returning = Vec::new();
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in state.zones.list(ZoneLocation::Exile(p)) {
                    if state.object(card).is_some_and(|o| {
                        o.riders
                            .iter()
                            .any(|r| matches!(r, crate::object::Rider::Linked { host } if *host == res.source))
                    }) {
                        returning.push(card);
                    }
                }
            }
            for card in returning {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.riders
                        .retain(|r| !matches!(r, crate::object::Rider::Linked { host } if *host == res.source));
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::LoseLife { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            for player in eval::players(target, state, you) {
                let p = &mut state.players[player.get() as usize];
                let old = p.life;
                p.life -= n;
                let new = p.life;
                state.journal.record(GameEvent::LifeChanged {
                    player,
                    old,
                    new,
                    cause: Cause::Effect,
                });
            }
            None
        }
        Effect::DrawCards { amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as usize;
            state.draw_cards(you, n);
            None
        }
        Effect::DrawCardsFor { amount, who } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as usize;
            let players = match who {
                PlayerRel::Chosen => res.chosen_player.into_iter().collect(),
                other => eval::players(other, state, you),
            };
            for player in players {
                state.draw_cards(player, n);
            }
            None
        }
        Effect::ExileTargetsCreateTokens { token } => {
            let targets = res.targets.clone();
            for target_id in targets {
                let Some(obj) = state.object(target_id) else {
                    continue;
                };
                let owner = obj.owner;
                let controller = obj.controller;
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                // Token replacement applies per token created (CR 614.1
                // applies to the total, per controller).
                create_one_token(state, controller, token);
            }
            None
        }
        Effect::DealDamage { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i16;
            match target {
                TargetSpec::Player(rel) => {
                    for player in eval::players(rel, state, you) {
                        deal_to_player(state, res.source, player, n);
                    }
                }
                _ => {
                    if let Some(&target_id) = res.targets.first() {
                        if let Some(obj) = state.object_mut(target_id) {
                            obj.damage = obj.damage.saturating_add(n.max(0) as u16);
                        }
                        state.journal.record(GameEvent::DamageDealt {
                            source: Some(res.source),
                            target: DamageTarget::Object(target_id),
                            amount: n.max(0) as u16,
                            is_combat: false,
                        });
                    }
                }
            }
            None
        }
        Effect::Destroy { .. } => {
            if let Some(&target_id) = res.targets.first() {
                sba::destroy(state, target_id);
            }
            None
        }
        Effect::CounterTargetSpell => {
            if let Some(&target_id) = res.targets.first() {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::Mill { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as usize;
            for player in eval::players(target, state, you) {
                let top: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .iter()
                    .rev()
                    .take(n)
                    .copied()
                    .collect();
                for card in top {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::AddMana { color, amount } => {
            state.players[you.get() as usize]
                .mana_pool
                .add(color, amount);
            state.journal.record(GameEvent::ManaProduced {
                player: you,
                color,
                amount,
                source: Some(res.source),
            });
            None
        }
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::AddManaChoice { .. }
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
    }
}

fn create_one_token(
    state: &mut GameState,
    controller: PlayerId,
    token: &baylee_cards_dsl::TokenDef,
) {
    let name = state.names.intern(token.name);
    let base = Characteristics {
        name,
        mana_cost: ManaCost::ZERO,
        colors: token.colors,
        types: token.types,
        supertypes: token.supertypes,
        subtypes: SubtypeSet::from_slice(token.subtypes),
        keywords: token.keywords,
        power: token.power,
        toughness: token.toughness,
        loyalty: None,
    };
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        let mut obj = GameObject::new_bare(id, controller, ObjectKind::Permanent, base);
        obj.timestamp = ts;
        obj
    });
    state
        .zones
        .insert(id, ZoneLocation::Battlefield, ZonePosition::Top);
    if let Some(obj) = state.object_mut(id) {
        obj.zone = crate::zone::Zone::Battlefield;
    }
}

fn change_controller(state: &mut GameState, target: ObjectId, new_controller: PlayerId) {
    let Some(obj) = state.object(target) else {
        return;
    };
    let old = obj.controller;
    if old == new_controller {
        return;
    }
    let ts = state.next_timestamp();
    {
        let obj = state.object_mut(target).expect("checked above");
        obj.controller = new_controller;
        // Control changes restart summoning sickness (CR 302.6).
        obj.timestamp = ts;
    }
    state.journal.record(GameEvent::ControllerChanged {
        object: target,
        old,
        new: new_controller,
    });
}

fn gain_life(state: &mut GameState, player: PlayerId, n: i32) {
    if n <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life += n;
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
}

fn deal_to_player(state: &mut GameState, source: ObjectId, player: PlayerId, n: i16) {
    if n <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life -= i32::from(n);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount: n as u16,
        is_combat: false,
    });
}
