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
use baylee_core::mana::ManaColor;
use smallvec::SmallVec;

mod counters;
mod life;
mod mana;
mod tokens;
mod zones;

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
    /// Players chosen as *targets* ("any target", CR 115.4).
    ///
    /// Distinct from `chosen_player`, which is the single player a
    /// `TargetSpec::AnyPlayer` names: this list rides alongside `targets`
    /// as the other half of one mixed choice, so a spell that hits two
    /// any-targets can hit two players.
    pub target_players: baylee_core::ids::SeatSet,
    /// The object the triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// The suspended choice, if any.
    pub awaiting: Option<AwaitingOp>,
    /// Whether this is a mana ability resolving off the stack (CR 605.3b).
    ///
    /// It changes what happens when the resolution *finishes*: there is no
    /// stack object to finalize, and its controller keeps priority.
    pub mana_ability: bool,
}

/// The one destination Path to Exile's basic-land search uses.
static ONTO_BATTLEFIELD_TAPPED: &[baylee_cards_dsl::effect::Find] =
    &[baylee_cards_dsl::effect::Find::BATTLEFIELD_TAPPED];

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
pub enum AwaitingOp {
    /// A library search: chosen cards go to `finds`, positionally.
    ///
    /// The library is always shuffled afterwards. Of the 1014 printed cards
    /// that search your library, three do not say "then shuffle", and all
    /// three empty the library instead — so a flag here could only ever be a
    /// way to leak the library order by accident.
    SearchLibrary {
        /// Where each found card goes, in order.
        finds: &'static [baylee_cards_dsl::effect::Find],
        /// Whether the found cards are shown to everyone first.
        ///
        /// Derived, not declared: a search narrower than "a card" that
        /// ends somewhere hidden reveals what it found. Every printed
        /// card that reveals matches that rule, and none that keeps its
        /// find secret does — so a card cannot get it wrong.
        reveal: bool,
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
    /// After `WishToHand`: the chosen card, if any, goes to its owner's hand.
    WishToHand,
    /// After `CopyTargetSpell`: the copy's controller may choose new targets
    /// for it (CR 707.10c). Picking the same objects again is how they
    /// decline, so there is no separate "keep them" answer.
    CopyNewTargets {
        /// The copy on the stack whose targets change.
        copy: ObjectId,
    },
    /// After `DigRest`: the unpicked cards go to the bottom in the
    /// player's chosen order.
    DigBottom,
    /// After a taken-over search: the found card goes to exile playable
    /// by the agent (Opposition Agent).
    SearchTakeover {
        /// The player taking the search over.
        agent: PlayerId,
    },
    /// After `DiscardForPlayers`: discard the chosen cards, then ask the
    /// next remaining player.
    DiscardChain {
        /// The player currently discarding.
        player: PlayerId,
        /// Cards each player must discard.
        count: u8,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `DestroyChosenForPlayers`: destroy the chosen permanent
    /// (respects indestructible), then ask the next remaining player.
    DestroyChosen {
        /// What may be destroyed.
        filter: &'static baylee_cards_dsl::Filter,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
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
    /// A mana color choice.
    ///
    /// The options are already a concrete list: commander identity and the
    /// colors the lands on the battlefield produce are game state, so they
    /// are settled when the effect runs, not when the answer arrives.
    ManaChoice {
        /// Colors offered.
        colors: Vec<ManaColor>,
        /// Picks still to make (a combination picks once per mana).
        remaining: u16,
        /// Mana added per pick (1 for combination, all of it otherwise).
        per_pick: u16,
        /// What the mana may be spent on, if restricted.
        restriction: Option<baylee_cards_dsl::effect::ManaRestriction>,
    },
    /// "You may pay N life; if you don't, this enters tapped".
    PayLifeOrTapSelf {
        /// Life to pay.
        amount: u16,
    },
}

/// Whether a search shows what it found.
///
/// The printed cards agree on a rule rather than deciding one by one: a
/// search narrower than "a card" reveals its find on the way to a hidden
/// zone, and a search that ends somewhere public does not — the card is
/// about to be visible anyway. Of the 1015 printed searches in the forge
/// reference, none reveals where this says it should not, so the flag a
/// card file would carry could only ever be wrong.
fn reveals(
    filter: &'static baylee_cards_dsl::Filter,
    finds: &[baylee_cards_dsl::effect::Find],
) -> bool {
    !matches!(filter, baylee_cards_dsl::Filter::Any)
        && finds
            .iter()
            .any(|f| matches!(f.dest, SearchDest::Hand | SearchDest::TopOfLibrary))
}

/// Amount evaluation with target context ([`Amount::TargetPower`]).
pub(super) fn amount2(
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

/// The seats a [`PlayerRel`] names *during a resolution*.
///
/// [`eval::players`] answers the half that the state alone can answer, and
/// deliberately returns nothing for the two relations that need the
/// resolution's own context: `Chosen` is the player this spell or ability
/// targeted, `ControllerOfTarget` is read off its first object target. An
/// effect that reaches for `eval::players` directly therefore does *nothing*
/// on a card that says "target opponent" — which is exactly how Abraded
/// Bluffs shipped as a land that deals no damage.
pub(super) fn players_of(
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    res: &Resolution,
) -> Vec<PlayerId> {
    match rel {
        PlayerRel::Chosen => res.chosen_player.into_iter().collect(),
        PlayerRel::ControllerOfTarget => res
            .targets
            .first()
            .and_then(|t| state.object(*t))
            .map_or_else(Vec::new, |o| vec![o.controller]),
        other => eval::players(other, state, you),
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
        restriction,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_with_color on non-mana choice");
    };
    debug_assert!(colors.contains(&color));
    mana::add(state, res, color, per_pick, restriction);
    if remaining > 1 {
        res.awaiting = Some(AwaitingOp::ManaChoice {
            colors: colors.clone(),
            remaining: remaining - 1,
            per_pick,
            restriction,
        });
        return Flow::Wait(Pending::ChooseColor {
            player: res.controller,
            options: colors,
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
    // `pay` mutates the pool — never hide the call behind `debug_assert!`,
    // which is not evaluated in release. A failed payment takes the
    // not-paid fallback, exactly as if the player had declined.
    let actually_paid = paid
        && mana_pay::pay(
            &mut state.players[player.get() as usize].mana_pool,
            &baylee_core::mana::ManaCost::parse(&format!("{{{mana}}}")),
        );
    debug_assert!(!paid || actually_paid, "tax was offered as payable");
    if actually_paid {
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
        target_players: res.target_players,
        awaiting: None,
        mana_ability: false,
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
        AwaitingOp::SearchLibrary { finds, reveal } => {
            if reveal && !chosen.is_empty() {
                // Shown from the library, before they go anywhere.
                state.journal.record(GameEvent::Revealed {
                    player: res.controller,
                    cards: chosen.to_vec(),
                });
            }
            // Positional: the first card found takes the first destination.
            // Cultivate names the battlefield first and the hand second, and
            // finding only one card then puts that one onto the battlefield —
            // the same order the printed text reads in.
            for (&card, find) in chosen.iter().zip(finds) {
                let (dest, tapped) = (find.dest, find.tapped);
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
            state.shuffle_library(res.controller);
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
            // "The rest on the bottom in any order": the player chooses
            // the order (OrderObjects pending when there's a choice).
            let remaining: Vec<ObjectId> =
                rest.into_iter().filter(|c| !chosen.contains(c)).collect();
            if remaining.len() > 1 {
                res.awaiting = Some(AwaitingOp::DigBottom);
                return Flow::Wait(Pending::OrderObjects {
                    player: res.controller,
                    objects: remaining,
                });
            }
            for card in remaining {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::DigBottom => {
            // Chosen order: first listed goes to the bottom first.
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
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
        AwaitingOp::WishToHand => {
            if let Some(&card) = chosen.first() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::CopyNewTargets { copy } => {
            if let Some(obj) = state.object_mut(copy) {
                obj.targets.clear();
                obj.targets.extend(chosen.iter().copied());
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
        AwaitingOp::SearchTakeover { agent } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(agent),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if let Some(obj) = state.object_mut(card) {
                    obj.riders
                        .push(crate::object::Rider::PlayableFromExileFor(agent));
                }
            }
        }
        AwaitingOp::DiscardChain {
            player,
            count,
            remaining,
        } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(player),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            while let Some(player) = remaining.first().copied() {
                remaining.remove(0);
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                if hand.is_empty() {
                    continue;
                }
                let n = (count as usize).min(hand.len()) as u8;
                res.awaiting = Some(AwaitingOp::DiscardChain {
                    player,
                    count,
                    remaining,
                });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options: hand,
                    min: n,
                    max: n,
                    prompt: ChoicePrompt::Generic,
                });
            }
        }
        AwaitingOp::DestroyChosen { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                crate::sba::destroy(state, victim);
            }
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
                    res.awaiting = Some(AwaitingOp::DestroyChosen { filter, remaining });
                    return Flow::Wait(Pending::ChooseCards {
                        player,
                        options,
                        min: 0,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                    });
                }
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
        AwaitingOp::ManaChoice { .. } | AwaitingOp::PayLifeOrTapSelf { .. } => {
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
        | Effect::AddMana { .. }
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
            finds,
            optional,
        } => {
            // Ashiok, Dream Render: opponents can't search libraries.
            if state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                    && fx.controller != you
            }) {
                return None;
            }
            // Opposition Agent: an opponent of the searching player takes
            // the search over — they choose, and the find goes to exile
            // playable by them.
            let takeover = state
                .effects
                .iter()
                .find(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::SearchTakeover)
                        && fx.controller != you
                })
                .map(|fx| fx.controller);
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
                state.shuffle_library(you);
                return None;
            }
            // How many cards this search may produce, and how few it may
            // settle for: "up to two" is optional with two finds, "search for
            // a basic land card" is one find and mandatory.
            let want = u8::try_from(finds.len()).unwrap_or(u8::MAX);
            let least = if optional { 0 } else { want };
            if let Some(agent) = takeover {
                res.awaiting = Some(AwaitingOp::SearchTakeover { agent });
                return Some(Pending::ChooseCards {
                    player: agent,
                    options,
                    min: least,
                    max: want,
                    prompt: ChoicePrompt::SearchLibrary,
                });
            }
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                finds,
                reveal: reveals(filter, finds),
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: least,
                max: want,
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
                source: resolving_ability(state, res),
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
            // Onto the battlefield, where everyone sees it anyway.
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                finds: ONTO_BATTLEFIELD_TAPPED,
                reveal: false,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            })
        }
        Effect::AddMana { .. } => mana::exec(state, res, op),
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
                source: state.object(res.source).and_then(|o| o.card).map(|c| {
                    baylee_core::ids::AbilityRef::new(c.index, baylee_core::ids::AbilityRef::ENTERS)
                }),
            })
        }
        _ => unreachable!("not a choice op"),
    }
}

/// The card ability a resolution belongs to.
///
/// This is the handle a seat's standing answer is stored under and the
/// label a client puts on a stack entry, so it has to name the *ability*
/// (Ondu Cleric's rally trigger) rather than only the permanent.
#[must_use]
pub fn resolving_ability(
    state: &GameState,
    res: &Resolution,
) -> Option<baylee_core::ids::AbilityRef> {
    use baylee_core::ids::AbilityRef;
    let obj = state.object(res.on_stack)?;
    if let Some(loc) = obj.ability {
        return Some(AbilityRef::new(loc.card, loc.index));
    }
    // A spell resolving: what it does is its spell ability, which is not
    // an entry in the card's ability list.
    obj.card
        .map(|c| AbilityRef::new(c.index, AbilityRef::SPELL))
}

/// Runs a nested branch (If*/kicked-style conditional effects) inline;
/// a suspension inside the branch splices its remaining ops into the
/// parent's program and propagates the choice.
fn run_nested_with(
    state: &mut GameState,
    res: &mut Resolution,
    effects: Vec<Effect>,
    targets: SmallVec<[ObjectId; 2]>,
) -> Option<Pending> {
    let mut nested = Resolution {
        source: res.source,
        on_stack: res.on_stack,
        controller: res.controller,
        effects,
        pc: 0,
        targets,
        event_object: res.event_object,
        x: res.x,
        chosen_player: res.chosen_player,
        target_players: res.target_players,
        awaiting: None,
        mana_ability: false,
    };
    match run(state, &mut nested) {
        Flow::Complete => None,
        Flow::Wait(pending) => {
            res.awaiting = nested.awaiting;
            res.effects.splice(res.pc..res.pc, nested.effects);
            Some(pending)
        }
    }
}

/// Runs a nested static branch inline (see [`run_nested_with`]).
fn run_nested(
    state: &mut GameState,
    res: &mut Resolution,
    branch: &'static [Effect],
) -> Option<Pending> {
    let targets = res.targets.clone();
    run_nested_with(state, res, flatten(branch), targets)
}

/// Operations that complete immediately. This is only the dispatcher:
/// effect families live in their own modules (life/damage, zones, mana,
/// counters/P-T, tokens); control, draw, conditions, and the misc tail
/// stay below.
#[allow(clippy::too_many_lines)] // dispatch table + misc tail
fn exec_immediate(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::Sequence(_) => unreachable!("sequences are flattened"),
        Effect::GainLife { .. }
        | Effect::GainLifeFor { .. }
        | Effect::GainLifeDoubleX
        | Effect::LoseLife { .. }
        | Effect::DealDamage { .. }
        | Effect::DealDamageToTargetController { .. } => life::exec(state, res, op),
        Effect::Exile { .. }
        | Effect::Blink { .. }
        | Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::DestroyAll { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::PutSourceOnTopOfLibrary
        | Effect::BottomCardFromHand { .. }
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::PhaseOut { .. }
        | Effect::ExileLinked { .. }
        | Effect::SacrificeSelf
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::ExileSource
        | Effect::ExileAndReturnAtEndStep
        | Effect::ExileLibraryAndShuffleHand { .. }
        | Effect::Mill { .. }
        | Effect::Destroy { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::DiscardForPlayers { .. }
        | Effect::SacrificeFilter { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::ExileSelfReturnAsFace { .. }
        | Effect::ReturnLinkedToBattlefield
        | Effect::ExileTargetsCreateTokens { .. }
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetSpell => zones::exec(state, res, op),
        Effect::DelayedManaAtNextFirstMain { .. } => mana::exec(state, res, op),
        Effect::AddCounter { .. }
        | Effect::AddCounterFilter { .. }
        | Effect::DrainAllCountersIntoSelf
        | Effect::SetPTFilter { .. }
        | Effect::PumpFilter { .. }
        | Effect::PumpTarget { .. } => counters::exec(state, res, op),
        Effect::CreateTokenForTargetController { .. }
        | Effect::Amass { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::CreateTokenCopyOfFirstToken
        | Effect::CreateTokenCopyOfEquipped { .. }
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateToken { .. }
        | Effect::CreateTokenFromLinked { .. } => tokens::exec(state, res, op),
        // --- Control ---------------------------------------------------
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
        Effect::ControlRotation => {
            // Aminatou −6 (heads-up): every nonland permanent swaps to
            // the other player (the direction choice is multiplayer-only).
            for &id in &state.zones.list(ZoneLocation::Battlefield).clone() {
                if id == res.source {
                    continue;
                }
                let Some(obj) = state.object(id) else {
                    continue;
                };
                if obj
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
                {
                    continue;
                }
                let other = PlayerId::new(1 - obj.controller.get());
                change_controller(state, id, other);
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
        // --- Cards drawn -------------------------------------------------
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
        // --- Conditional branches ---------------------------------------
        Effect::IfKicked { then, otherwise } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let branch = if kicked { then } else { otherwise };
            run_nested(state, res, branch)
        }
        Effect::IfCreaturesDiedAtLeast { n, then } => {
            if state.per_turn.creatures_died >= n {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfNotLostLifeThisTurn { then } => {
            // Journal scan since turn start: any LifeChanged for `you`
            // with new < old is a life loss (CR 119.4a note).
            let lost = state.journal.entries()[state.turn_start_seq as usize..]
                .iter()
                .any(|e| match &e.event {
                    GameEvent::LifeChanged {
                        player, old, new, ..
                    } => *player == you && new < old,
                    _ => false,
                });
            if !lost {
                return run_nested(state, res, then);
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
                return run_nested(state, res, then);
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
            run_nested_with(state, res, flatten(branch), targets)
        }
        // --- Effects, emblems, and the misc tail -------------------------
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
        Effect::BecomeMonarch => {
            state.set_monarch(you);
            None
        }
        Effect::BecomePrepared => {
            if let Some(obj) = state.object_mut(res.source)
                && !obj.riders.contains(&crate::object::Rider::Prepared)
            {
                obj.riders.push(crate::object::Rider::Prepared);
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
        Effect::WishToHand { filter } => {
            // Cards outside the game, plus your own exile — the one place in
            // the game a wish can already see. The choice is optional ("you
            // may"), so the minimum is zero.
            let mut options: Vec<ObjectId> = Vec::new();
            for loc in [ZoneLocation::OutsideGame(you), ZoneLocation::Exile(you)] {
                options.extend(state.zones.list(loc).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.owner == you && eval::matches(filter, state, o, you, res.source)
                    })
                }));
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::WishToHand);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Wish,
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
                    player_options: Vec::new(),
                    min: 1,
                    max: 1,
                });
            }
            None
        }
        Effect::TakeExtraTurn => {
            state.extra_turns.push_back(you);
            None
        }
        Effect::CreateEmblem { abilities } => {
            let name = match state.object(res.source) {
                Some(o) => o.base.name,
                None => state.names.intern("emblem"),
            };
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
        Effect::TapTarget => {
            for &target in &res.targets {
                if let Some(obj) = state.object_mut(target) {
                    obj.status.insert(crate::object::Status::TAPPED);
                }
            }
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
        Effect::CopyTargetSpell { mods } => {
            // Copy the spell on the stack under your control. The copy starts
            // with the original's targets and its controller may then choose
            // new ones (CR 707.10c), so this can suspend on a choice.
            if let Some(&target_id) = res.targets.first() {
                let (card, mut base, targets, target_req) = {
                    let obj = state.object(target_id)?;
                    (
                        obj.card,
                        (*obj.base).clone(),
                        obj.targets.clone(),
                        obj.target_req,
                    )
                };
                for m in mods {
                    tokens::apply_copy_mod(&mut base, m);
                }
                let name = base.name;
                let ts = state.next_timestamp();
                let id = state.arena.insert_with(|oid| {
                    let mut obj = GameObject::new_bare(oid, you, ObjectKind::Spell, base);
                    obj.timestamp = ts;
                    obj
                });
                let picks = u8::try_from(targets.len()).unwrap_or(u8::MAX);
                {
                    let obj = state.object_mut(id).expect("fresh copy");
                    obj.card = card;
                    obj.targets = targets;
                    obj.target_req = target_req;
                    obj.zone = crate::zone::Zone::Stack;
                }
                state
                    .zones
                    .insert(id, ZoneLocation::Stack, ZonePosition::Top, true);
                // Deliberately no `SpellCast` event: a copy is *put* onto the
                // stack, not cast (CR 707.10). Journalling one made every copy
                // re-trigger "whenever you cast" abilities — Jin-Gitaxias
                // copied its own copy without end — and made copies count
                // towards Storm of Saruman's "second spell each turn".
                let _ = name;
                // "You may choose new targets for the copy." Only worth asking
                // when the copy targets objects at all and there is something
                // legal to point it at; the player declines by re-picking what
                // it already targets.
                if picks > 0
                    && let Some(req) = target_req
                    // Player targets ride in `chosen_player`, not `targets`;
                    // re-choosing those is a separate Pending.
                    && !matches!(req.spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent)
                {
                    let options = eval::target_options(&req.spec, state, you, id);
                    if options.len() >= picks as usize {
                        res.awaiting = Some(AwaitingOp::CopyNewTargets { copy: id });
                        return Some(Pending::ChooseTargets {
                            player: you,
                            options,
                            player_options: Vec::new(),
                            min: picks,
                            max: picks,
                        });
                    }
                }
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
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::AddMana { .. }
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
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
        obj.set_controller(new_controller);
        // Control changes restart summoning sickness (CR 302.6).
        obj.timestamp = ts;
    }
    state.journal.record(GameEvent::ControllerChanged {
        object: target,
        old,
        new: new_controller,
    });
}
