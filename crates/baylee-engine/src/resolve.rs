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
use crate::mana_pay;
use crate::object::{Characteristics, GameObject, ObjectKind, Status};
use crate::sba;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Amount, Effect, PlayerRel, SearchDest, TargetSpec};
use baylee_core::color::ColorSet;
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
    /// The object the triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// The suspended choice, if any.
    pub awaiting: Option<AwaitingOp>,
}

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
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
    /// A player decides whether to pay for a tax effect.
    PlayerMayPay {
        /// The player deciding.
        player: PlayerId,
        /// Generic mana to pay.
        mana: u16,
        /// The effect to run when they don't pay.
        effect: &'static Effect,
    },
    /// Top-of-library reorder (Sensei's Divining Top).
    ReorderTopLibrary,
    /// A relative player bottoms a card from their hand (Vendilion Clique).
    BottomFromHand {
        /// Whose hand.
        player: PlayerId,
    },
    /// After `RedirectTarget`: set the spell's target to the chosen one.
    RedirectNewTarget {
        /// The spell on the stack whose target changes.
        spell: ObjectId,
    },
    /// After `SacrificeFilter`: sacrifice the chosen permanent, then ask
    /// the next remaining player.
    SacrificeFilter {
        /// What may be sacrificed.
        filter: &'static baylee_cards_dsl::Filter,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `LookAtTopPick`: chosen go to hand, the rest to the bottom.
    DigRest {
        /// The looked-at cards not chosen.
        rest: Vec<ObjectId>,
    },
    /// Chosen hand cards go on top of the library in chosen order.
    PutBackOnTop,
    /// One mana of a color in your commander's color identity
    /// (Command Tower).
    CommanderMana,
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
        Amount::TargetCmc => targets
            .first()
            .and_then(|t| state.object(*t))
            .map_or(0, |o| o.characteristics().mana_cost.cmc()),
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

/// Resumes a color choice suspended on [`AwaitingOp::ManaChoice`] or
/// [`AwaitingOp::CommanderMana`].
///
/// # Panics
/// When the suspended operation is not a mana choice.
#[must_use]
pub fn resume_with_color(state: &mut GameState, res: &mut Resolution, color: ManaColor) -> Flow {
    if matches!(res.awaiting, Some(AwaitingOp::CommanderMana)) {
        res.awaiting = None;
        state.players[res.controller.get() as usize]
            .mana_pool
            .add(color, 1);
        state.journal.record(GameEvent::ManaProduced {
            player: res.controller,
            color,
            amount: 1,
            source: Some(res.source),
        });
        res.pc += 1;
        return run(state, res);
    }
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

/// Resumes a tax choice (Rhystic Study & co.): `paid` means the player
/// chose to pay the mana.
///
/// # Panics
/// When the suspended operation is not a tax choice.
#[must_use]
pub fn resume_tax_choice(state: &mut GameState, res: &mut Resolution, paid: bool) -> Flow {
    let AwaitingOp::PlayerMayPay {
        player,
        mana,
        effect,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_tax_choice on non-tax choice");
    };
    if paid {
        debug_assert!(mana_pay::pay(
            &mut state.players[player.get() as usize].mana_pool,
            &baylee_core::mana::ManaCost::parse(&format!("{{{mana}}}")),
        ));
        res.pc += 1;
        return run(state, res);
    }
    // Not paid: run the fallback effect inline, then continue.
    let fallback = Resolution {
        source: res.source,
        on_stack: res.on_stack,
        controller: res.controller,
        effects: flatten(std::slice::from_ref(effect)),
        pc: 0,
        targets: res.targets.clone(),
        event_object: res.event_object,
        x: res.x,
        chosen_player: res.chosen_player,
        awaiting: None,
    };
    let mut fallback = fallback;
    match run(state, &mut fallback) {
        Flow::Complete => {}
        Flow::Wait(pending) => {
            res.awaiting = fallback.awaiting;
            res.effects.splice(res.pc..res.pc, fallback.effects);
            return Flow::Wait(pending);
        }
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes a suspended resolution with the chosen cards.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
#[allow(clippy::too_many_lines)]
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
        AwaitingOp::DigRest { rest } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for card in rest {
                if !chosen.contains(&card) {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(res.controller),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
            }
        }
        AwaitingOp::BottomFromHand { player } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(player),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::RedirectNewTarget { spell } => {
            if let Some(&new_target) = chosen.first()
                && let Some(obj) = state.object_mut(spell)
            {
                obj.targets.clear();
                obj.targets.push(new_target);
            }
        }
        AwaitingOp::SacrificeFilter { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                let owner = state.object(victim).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(victim) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    victim,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // Ask the next player who still has a legal sacrifice.
            let mut remaining = remaining;
            while let Some(player) = remaining.first().copied() {
                remaining.remove(0);
                let options: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .filter(|id| {
                        state.object(**id).is_some_and(|o| {
                            o.controller == player
                                && eval::matches(filter, state, o, res.controller, res.source)
                        })
                    })
                    .copied()
                    .collect();
                if !options.is_empty() {
                    res.awaiting = Some(AwaitingOp::SacrificeFilter { filter, remaining });
                    return Flow::Wait(Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                    });
                }
            }
        }
        AwaitingOp::ReorderTopLibrary => {
            // chosen[0] becomes the topmost card (end of the library vec).
            for &card in chosen.iter().rev() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::ManaChoice { .. }
        | AwaitingOp::CommanderMana
        | AwaitingOp::PayLifeOrTapSelf { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
        AwaitingOp::PlayerMayPay { .. } => {
            unreachable!("tax choices resume via resume_tax_choice")
        }
    }
    res.pc += 1;
    run(state, res)
}

/// Executes one operation; returns `Some(pending)` when it suspends.
fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::AddManaChoice { .. }
        | Effect::AddManaCommanderIdentity
        | Effect::PayLifeOrEnterTapped { .. } => exec_choice(state, res, op),
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
            // Ashiok, Dream Render: opponents can't search libraries.
            if state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                    && fx.controller != you
            }) {
                return None;
            }
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
        Effect::ScryFor { player, amount } => {
            let players = match player {
                PlayerRel::Chosen => res.chosen_player.into_iter().collect::<Vec<_>>(),
                other => eval::players(other, state, you),
            };
            let player = players.first().copied()?;
            let n = eval::amount(&amount, state, player, res.source, res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
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
                player,
                options: looked,
                min: 0,
                max: n as u8,
                prompt: ChoicePrompt::ScryBottom,
            })
        }
        Effect::Scry { amount } => {
            let n = eval::amount(&amount, state, you, res.source, res.x) as usize;
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
        Effect::PlayerMayPayOr {
            player,
            mana,
            effect,
        } => {
            let player = eval::players(player, state, you).first().copied()?;
            // If they can't pay, the fallback fires immediately.
            let can_pay = state.players[player.get() as usize].mana_pool.total() >= u32::from(mana);
            if !can_pay {
                return exec_immediate(state, res, *effect);
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                mana,
                effect,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
            })
        }
        Effect::ReorderTopLibrary { count } => {
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReorderTopLibrary);
            Some(Pending::OrderObjects {
                player: you,
                objects: options,
            })
        }
        Effect::OptionalBasicLandSearchFor { player } => {
            // Ashiok, Dream Render: opponents can't search libraries.
            if state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                    && fx.controller != you
            }) {
                return None;
            }
            let player = eval::players(player, state, you).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::LAND)
                            && o.characteristics()
                                .supertypes
                                .contains(baylee_core::types::SupertypeSet::BASIC)
                    })
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                dest: SearchDest::Battlefield,
                tapped: true,
                shuffle: true,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            })
        }
        Effect::AddManaCommanderIdentity => {
            // Union the color identities of your command-zone cards.
            let mut colors = ColorSet::EMPTY;
            for id in state.zones.list(ZoneLocation::Command(you)) {
                if let Some(obj) = state.object(*id) {
                    colors = colors.union(obj.characteristics().color_identity);
                }
            }
            let options: Vec<ManaColor> = [
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ]
            .into_iter()
            .filter(|c| {
                colors.contains(match c {
                    ManaColor::White => baylee_core::color::Color::White,
                    ManaColor::Blue => baylee_core::color::Color::Blue,
                    ManaColor::Black => baylee_core::color::Color::Black,
                    ManaColor::Red => baylee_core::color::Color::Red,
                    ManaColor::Green => baylee_core::color::Color::Green,
                    ManaColor::Colorless => return false,
                })
            })
            .collect();
            if options.is_empty() {
                // No commander (non-commander game): colorless.
                state.players[you.get() as usize]
                    .mana_pool
                    .add(ManaColor::Colorless, 1);
                return None;
            }
            res.awaiting = Some(AwaitingOp::CommanderMana);
            Some(Pending::ChooseColor {
                player: you,
                options,
            })
        }
        Effect::AddManaChoice {
            colors,
            amount,
            combination,
        } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            if n == 0 {
                return None;
            }
            let per_pick = if combination { 1 } else { n };
            res.awaiting = Some(AwaitingOp::ManaChoice {
                colors,
                remaining: n,
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
        Effect::Blink { .. } => {
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
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
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
        Effect::AddCounterFilter {
            filter,
            kind,
            amount,
        } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            let objects: Vec<ObjectId> = state
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
            for id in objects {
                if let Some(obj) = state.object_mut(id) {
                    let old = obj.counters.get(kind);
                    let new = obj.counters.add(kind, n);
                    state.journal.record(GameEvent::CounterChanged {
                        object: id,
                        kind,
                        old,
                        new,
                    });
                }
            }
            None
        }
        Effect::AddManaDynamic { color, amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            state.players[you.get() as usize].mana_pool.add(color, n);
            state.journal.record(GameEvent::ManaProduced {
                player: you,
                color,
                amount: n,
                source: Some(res.source),
            });
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
        Effect::GraveyardToHand { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
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
        Effect::CreateTokenForTargetController { token } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                create_one_token(state, controller, token);
            }
            None
        }
        Effect::Amass { subtype, amount } => {
            // Find an Army you control; if none, create a 0/0 Army token.
            static ARMY: baylee_core::ids::SubtypeId = baylee_core::ids::SubtypeId::new(0);
            let _ = ARMY;
            let army = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.controller == you
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                            && o.characteristics().subtypes.contains(subtype)
                    })
                });
            let target_id = if let Some(id) = army {
                id
            } else {
                // Create the 0/0 Army token, then put counters on it.
                let name = state.names.intern("Army");
                let base = Characteristics {
                    name,
                    mana_cost: ManaCost::ZERO,
                    colors: ColorSet::EMPTY,
                    types: baylee_core::types::TypeSet::CREATURE,
                    supertypes: baylee_core::types::SupertypeSet::EMPTY,
                    subtypes: SubtypeSet::from_slice(&[subtype]),
                    keywords: baylee_cards_dsl::KeywordSet::EMPTY,
                    power: Some(0),
                    toughness: Some(0),
                    loyalty: None,
                    color_identity: ColorSet::EMPTY,
                    produced_colors: ColorSet::EMPTY,
                    produced_colorless: false,
                };
                let ts = state.next_timestamp();
                let id = state.arena.insert_with(|id| {
                    let mut obj = GameObject::new_bare(id, you, ObjectKind::Permanent, base);
                    obj.timestamp = ts;
                    obj
                });
                state
                    .zones
                    .insert(id, ZoneLocation::Battlefield, ZonePosition::Top);
                if let Some(obj) = state.object_mut(id) {
                    obj.zone = crate::zone::Zone::Battlefield;
                }
                id
            };
            if let Some(obj) = state.object_mut(target_id) {
                let old = obj.counters.get(baylee_cards_dsl::CounterKind::P1P1);
                let new = obj
                    .counters
                    .add(baylee_cards_dsl::CounterKind::P1P1, amount);
                state.journal.record(GameEvent::CounterChanged {
                    object: target_id,
                    kind: baylee_cards_dsl::CounterKind::P1P1,
                    old,
                    new,
                });
            }
            None
        }
        Effect::PutSourceOnTopOfLibrary => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            let _ = state.move_object(
                res.source,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::CreateTokenCopyOf {
            target,
            kicked_bonus,
        } => {
            let target_id = match target {
                Some(_) => res.targets.first().copied(),
                None => Some(res.source),
            };
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(id) = target_id
                && let Some(base) = state.object(id).map(|o| o.base.clone())
            {
                for _ in 0..count {
                    let base = base.clone();
                    let ts = state.next_timestamp();
                    let new_id = state.arena.insert_with(|oid| {
                        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
                        obj.timestamp = ts;
                        obj
                    });
                    state
                        .zones
                        .insert(new_id, ZoneLocation::Battlefield, ZonePosition::Top);
                    if let Some(obj) = state.object_mut(new_id) {
                        obj.zone = crate::zone::Zone::Battlefield;
                    }
                }
            }
            None
        }
        Effect::CreateTokenCopyOfFirstToken => {
            let token = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.card.is_none()
                            && o.controller == you
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                });
            if let Some(id) = token
                && let Some(base) = state.object(id).map(|o| o.base.clone())
            {
                let ts = state.next_timestamp();
                let new_id = state.arena.insert_with(|oid| {
                    let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
                    obj.timestamp = ts;
                    obj
                });
                state
                    .zones
                    .insert(new_id, ZoneLocation::Battlefield, ZonePosition::Top);
                if let Some(obj) = state.object_mut(new_id) {
                    obj.zone = crate::zone::Zone::Battlefield;
                }
            }
            None
        }
        Effect::BottomCardFromHand { player, filter } => {
            let player = eval::players(player, state, you).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, player, res.source))
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::BottomFromHand { player });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::CreateTokenCopyOfEquipped { kicked_bonus } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(equipped) = state.object(res.source).and_then(|o| o.attached_to)
                && let Some(base) = state.object(equipped).map(|o| o.base.clone())
            {
                for _ in 0..count {
                    let base = base.clone();
                    let ts = state.next_timestamp();
                    let id = state.arena.insert_with(|oid| {
                        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
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
            }
            None
        }
        Effect::CopyTargetSpell => {
            // Copy the spell on the stack under your control (same targets;
            // target re-choice is a protocol M3 item).
            if let Some(&target_id) = res.targets.first() {
                let (card, base, targets) = {
                    let obj = state.object(target_id)?;
                    (obj.card, obj.base.clone(), obj.targets.clone())
                };
                let name = base.name;
                let ts = state.next_timestamp();
                let id = state.arena.insert_with(|oid| {
                    let mut obj = GameObject::new_bare(oid, you, ObjectKind::Spell, base);
                    obj.timestamp = ts;
                    obj
                });
                {
                    let obj = state.object_mut(id).expect("fresh copy");
                    obj.card = card;
                    obj.targets = targets;
                    obj.zone = crate::zone::Zone::Stack;
                }
                state
                    .zones
                    .insert(id, ZoneLocation::Stack, ZonePosition::Top);
                state.journal.record(GameEvent::SpellCast {
                    object: id,
                    player: you,
                });
                let _ = name;
            }
            None
        }
        Effect::AttachSelf { .. } => {
            if let Some(&target_id) = res.targets.first()
                && let Some(obj) = state.object_mut(res.source)
            {
                obj.attached_to = Some(target_id);
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let mut count = amount2(&amount, state, you, res.source, res.x, &res.targets);
            // Token-creation replacements double the total (CR 614.1).
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
                create_one_token(state, you, token);
            }
            None
        }
        Effect::BecomeMonarch => {
            state.set_monarch(you);
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
        Effect::GainLifeDoubleX => {
            let n = res.x.unwrap_or(0).saturating_mul(2) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::DrainAllCountersIntoSelf => {
            let mut drained: u16 = 0;
            for id in state.zones.list(ZoneLocation::Battlefield).clone() {
                if let Some(obj) = state.object_mut(id) {
                    let held: Vec<_> = obj.counters.iter().collect();
                    for (kind, n) in held {
                        if n > 0 {
                            obj.counters.set(kind, 0);
                            drained = drained.saturating_add(n);
                        }
                    }
                }
            }
            if drained > 0
                && let Some(src) = state.object_mut(res.source)
            {
                src.counters
                    .add(baylee_cards_dsl::CounterKind::P1P1, drained);
            }
            None
        }
        Effect::IfControlGreatestCmc { filter, then } => {
            // Greatest cmc among filter-matching permanents; condition
            // holds when you control one of them (Padeem).
            let mut greatest = 0u32;
            let mut holds = false;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                let Some(obj) = state.object(*id) else {
                    continue;
                };
                if !eval::matches(filter, state, obj, you, res.source) {
                    continue;
                }
                let cmc = obj.characteristics().mana_cost.cmc();
                if cmc > greatest {
                    greatest = cmc;
                    holds = obj.controller == you;
                } else if cmc == greatest && obj.controller == you {
                    holds = true;
                }
            }
            if holds {
                let mut nested = Resolution {
                    source: res.source,
                    on_stack: res.on_stack,
                    controller: res.controller,
                    effects: flatten(then),
                    pc: 0,
                    targets: res.targets.clone(),
                    event_object: res.event_object,
                    x: res.x,
                    chosen_player: res.chosen_player,
                    awaiting: None,
                };
                match run(state, &mut nested) {
                    Flow::Complete => {}
                    Flow::Wait(pending) => {
                        res.awaiting = nested.awaiting;
                        res.effects.splice(res.pc..res.pc, nested.effects);
                        return Some(pending);
                    }
                }
            }
            None
        }
        Effect::IfEventPowerAtLeast { n, then, otherwise } => {
            let power = res
                .event_object
                .and_then(|id| state.object(id))
                .and_then(|o| o.characteristics().power)
                .unwrap_or(0);
            let branch = if power >= n { then } else { otherwise };
            let targets: SmallVec<[ObjectId; 2]> = res.event_object.into_iter().collect();
            let mut nested = Resolution {
                source: res.source,
                on_stack: res.on_stack,
                controller: res.controller,
                effects: flatten(branch),
                pc: 0,
                targets,
                event_object: res.event_object,
                x: res.x,
                chosen_player: res.chosen_player,
                awaiting: None,
            };
            match run(state, &mut nested) {
                Flow::Complete => {}
                Flow::Wait(pending) => {
                    res.awaiting = nested.awaiting;
                    res.effects.splice(res.pc..res.pc, nested.effects);
                    return Some(pending);
                }
            }
            None
        }
        Effect::ExchangeControlOrSacrifice => {
            let exchange = res.targets.first().copied().filter(|t| {
                state.object(*t).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.controller != you
                })
            });
            if let Some(target) = exchange {
                let their_controller = state.object(target).map_or(you, |o| o.controller);
                change_controller(state, target, you);
                change_controller(state, res.source, their_controller);
            } else {
                // No exchange: sacrifice the source (Gilded Drake).
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
        Effect::AllCreaturesToOwner => {
            let creatures: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                })
                .copied()
                .collect();
            for id in creatures {
                let owner = state.object(id).map_or(you, |o| o.owner);
                change_controller(state, id, owner);
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
        Effect::PayCostOrLoseLater { cost } => {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextUpkeep,
                action: crate::state::DelayedAction::PayCostOrLose { cost },
            });
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
        Effect::SetPTFilter {
            filter,
            power,
            toughness,
            duration,
        } => {
            let signed = |a: &Amount| -> i16 {
                let v = amount2(a, state, you, res.source, res.x, &res.targets) as i16;
                if matches!(a, Amount::NegX | Amount::NegXFixed(_)) {
                    -v
                } else {
                    v
                }
            };
            let p = signed(&power);
            let t = signed(&toughness);
            let ts = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer: baylee_cards_dsl::Layer::PtSet,
                timestamp: ts,
                duration,
                filter: crate::effects::EffectFilter::Dsl(filter),
                modifier: baylee_cards_dsl::Modifier::SetPT(p, t),
            });
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
                if matches!(a, Amount::NegX | Amount::NegXFixed(_)) {
                    -v
                } else {
                    v
                }
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
                // Everybody Lives: the controller can't lose life this turn.
                let cant = state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantLoseLife)
                        && fx.controller == you
                });
                if cant {
                    continue;
                }
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
                        deal_to_object_with_loyalty(state, target_id, n, res.source);
                    }
                }
            }
            None
        }
        Effect::DealDamageToTargetController { amount } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i16;
                deal_to_player(state, res.source, controller, n);
            }
            None
        }
        Effect::Destroy { .. } => {
            if let Some(&target_id) = res.targets.first() {
                sba::destroy(state, target_id);
            }
            None
        }
        Effect::CounterTargetAbility => {
            if let Some(&target_id) = res.targets.first() {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                // Abilities on the stack cease to exist when countered.
                state.zones.remove(target_id, ZoneLocation::Stack);
                let _ = state.arena.remove(target_id);
            }
            None
        }
        Effect::TargetSourceLosesAbilities => {
            if let Some(&target_id) = res.targets.first() {
                let source = state
                    .object(target_id)
                    .and_then(|o| o.ability.map(|a| a.source));
                if let Some(src) = source {
                    let ts = state.next_timestamp();
                    state.effects.register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(res.source),
                        controller: you,
                        layer: baylee_cards_dsl::Layer::Ability,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                        filter: crate::effects::EffectFilter::ObjectIs(src),
                        modifier: baylee_cards_dsl::Modifier::LoseKeywords,
                    });
                }
            }
            None
        }
        Effect::DelayedManaAtNextFirstMain { color } => {
            let cmc = res
                .targets
                .first()
                .and_then(|t| state.object(*t))
                .map_or(0, |o| o.characteristics().mana_cost.cmc());
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextFirstMain,
                action: crate::state::DelayedAction::AddMana {
                    color,
                    amount: cmc as u16,
                },
            });
            None
        }
        Effect::SacrificeFilter { who, filter } => {
            let mut players = eval::players(who, state, you);
            players.retain(|p| {
                state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .any(|id| {
                        state.object(*id).is_some_and(|o| {
                            o.controller == *p && eval::matches(filter, state, o, you, res.source)
                        })
                    })
            });
            let player = players.first().copied()?;
            players.remove(0);
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == player && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
                .collect();
            res.awaiting = Some(AwaitingOp::SacrificeFilter {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::AddManaLandColor { mine } => {
            // Union the producible mana of all lands of the chosen side.
            let mut colors = ColorSet::EMPTY;
            let mut colorless = false;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                let Some(obj) = state.object(*id) else {
                    continue;
                };
                if !obj
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
                {
                    continue;
                }
                let side_matches = if mine {
                    obj.controller == you
                } else {
                    obj.controller != you
                };
                if side_matches {
                    let c = obj.characteristics();
                    colors = colors.union(c.produced_colors);
                    colorless |= c.produced_colorless;
                }
            }
            let mut options: Vec<ManaColor> = [
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ]
            .into_iter()
            .filter(|c| {
                colors.contains(match c {
                    ManaColor::White => baylee_core::color::Color::White,
                    ManaColor::Blue => baylee_core::color::Color::Blue,
                    ManaColor::Black => baylee_core::color::Color::Black,
                    ManaColor::Red => baylee_core::color::Color::Red,
                    ManaColor::Green => baylee_core::color::Color::Green,
                    ManaColor::Colorless => return false,
                })
            })
            .collect();
            if colorless {
                options.push(ManaColor::Colorless);
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::CommanderMana);
            Some(Pending::ChooseColor {
                player: you,
                options,
            })
        }
        Effect::LookAtTopPick { count, pick } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::DigRest { rest: top.clone() });
            Some(Pending::ChooseCards {
                player: you,
                options: top,
                min: pick,
                max: pick,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::RedirectTarget { new_filter } => {
            // The new target is chosen at resolution (CR 115.7): ask the
            // controller for any object matching the filter.
            if let Some(&spell_id) = res.targets.first() {
                let options: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .filter(|id| {
                        state
                            .object(**id)
                            .is_some_and(|o| eval::matches(new_filter, state, o, you, res.source))
                    })
                    .copied()
                    .collect();
                if options.is_empty() {
                    return None;
                }
                res.awaiting = Some(AwaitingOp::RedirectNewTarget { spell: spell_id });
                return Some(Pending::ChooseTargets {
                    player: you,
                    options,
                    min: 1,
                    max: 1,
                });
            }
            None
        }
        Effect::PutTargetOnBottomOfLibrary => {
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target,
                    ZoneLocation::Library(owner),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::TakeExtraTurn => {
            state.extra_turns.push_back(you);
            None
        }
        Effect::CreateEmblem { abilities } => {
            let name = state
                .object(res.source)
                .map(|o| o.base.name)
                .unwrap_or_else(|| state.names.intern("emblem"));
            let id = state.create_bare(you, ObjectKind::Emblem, name, ZoneLocation::Command(you));
            if let Some(obj) = state.object_mut(id) {
                obj.emblem_abilities = Some(abilities);
            }
            None
        }
        Effect::GrantFlashback => {
            if let Some(&target) = res.targets.first() {
                let ts = state.next_timestamp();
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    layer: baylee_cards_dsl::Layer::Text,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::ObjectIs(target),
                    modifier: baylee_cards_dsl::Modifier::GrantsFlashback,
                });
            }
            None
        }
        Effect::ExileSource => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::UntapTarget => {
            for &target in &res.targets {
                if let Some(obj) = state.object_mut(target) {
                    obj.status.remove(crate::object::Status::TAPPED);
                }
            }
            None
        }
        Effect::ExileAndReturnAtEndStep => {
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                );
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::NextEndStep,
                    action: crate::state::DelayedAction::ReturnToBattlefield { card: target },
                });
            }
            None
        }
        Effect::CounterTargetSpellOrAbility => {
            if let Some(&target_id) = res.targets.first() {
                let kind = state.object(target_id).map(|o| o.kind);
                if kind == Some(ObjectKind::AbilityOnStack) {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    state.zones.remove(target_id, ZoneLocation::Stack);
                    let _ = state.arena.remove(target_id);
                } else {
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
        Effect::ExileLibraryAndShuffleHand { player } => {
            for player in eval::players(player, state, you) {
                let lib: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(player)).clone();
                for card in lib {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
                state.shuffle_library(player);
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
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::AddManaChoice { .. }
        | Effect::AddManaCommanderIdentity
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
        color_identity: ColorSet::EMPTY,
        produced_colors: ColorSet::EMPTY,
        produced_colorless: false,
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

fn deal_to_object_with_loyalty(state: &mut GameState, target: ObjectId, n: i16, source: ObjectId) {
    if n <= 0 {
        return;
    }
    // Protection (CR 702.16b): matching sources deal no damage.
    if eval::protected_from(state, target, source) {
        return;
    }
    let is_walker = state.object(target).is_some_and(|o| {
        o.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::PLANESWALKER)
    });
    if is_walker {
        // Damage to a planeswalker removes loyalty counters (CR 306.8).
        let old = state.object(target).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        let new = old.saturating_sub(n as u16);
        if let Some(obj) = state.object_mut(target) {
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        state.journal.record(GameEvent::CounterChanged {
            object: target,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
    } else if let Some(obj) = state.object_mut(target) {
        obj.damage = obj.damage.saturating_add(n as u16);
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: n as u16,
        is_combat: false,
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
