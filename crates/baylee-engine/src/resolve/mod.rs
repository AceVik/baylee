//! Effect resolution: the op interpreter.
//!
//! Spells and abilities resolve by running their [`Effect`] list through a
//! small continuation machine: operations that need a player choice
//! (searches, scry) suspend into a `Pending::ChooseCards` and resume on the
//! answer. Everything runs through the normal event pipeline, so the
//! journal stays complete.

use crate::choice::{ChoicePrompt, Pending, TargetPrompt, YesNoPrompt};
use crate::engine::cost_wizard;
use crate::eval;
use crate::event::{Cause, DamageTarget, GameEvent};
use crate::mana_pay;
use crate::object::{Characteristics, GameObject, ObjectKind, Status};
use crate::sba;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Amount, CostPart, Effect, PlayerRel, SearchDest, TargetSpec};
use baylee_core::color::ColorSet;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use smallvec::SmallVec;

mod chosen;
mod control;
mod counters;
mod life;
mod mana;
mod tokens;
mod zones;

pub use control::resume_control_rotation;
/// Which colours a mana source can produce right now, at this board.
///
/// Exported because `baylee-gamehost` projects the answer into the view
/// (`PublicObject::board_mana`) and a client cannot work it out: a Reflecting
/// Pool's colours are the union of `produced_colors` over one side's lands,
/// which is a *projected* characteristic no registry carries, and a Command
/// Tower's are its controller's commanders' identity. A second copy of either
/// fold would be a second answer, and the one that disagreed would be a
/// permanent the planner counts on and the engine refuses.
///
/// This is the same function [`add_mana`](mana) calls on the way to handing
/// the mana out, which is the whole point of exporting it rather than writing
/// a projection beside it.
pub use mana::colors_of;

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
    /// Whether the thing being resolved said "target" at all.
    ///
    /// [`Filter::This`](baylee_cards_dsl::Filter::This) names two different
    /// objects and this is what tells them apart: the *source* for an ability
    /// that never targeted ("this creature gets +2/+2 until end of turn") and
    /// the *target* for one that did. An empty `targets` used to mean the
    /// first of those unconditionally, which was safe only for as long as a
    /// targeted ability could never resolve without a target. "Up to one
    /// target" makes that reachable, and Karn, the Great Creator's `+1`
    /// activated with nothing to point at animated *Karn*, set his power and
    /// toughness to a mana value nobody had chosen, and the next state-based
    /// check swept the walker into the graveyard.
    pub targeted: bool,
    /// Whether this is a mana ability resolving off the stack (CR 605.3b).
    ///
    /// It changes what happens when the resolution *finishes*: there is no
    /// stack object to finalize, and its controller keeps priority.
    pub mana_ability: bool,
    /// The permanent whose ability an earlier effect of *this* resolution
    /// countered.
    ///
    /// Tishana's Tidebinder is one sentence in two effects — counter the
    /// ability, then strip the permanent it came from — and the second half
    /// has no way of its own to name that permanent: both read the same
    /// target, and an ability that has been countered ceases to exist
    /// (CR 701.6a), so the lookup finds nothing. It found nothing for as
    /// long as the card existed, and the rider had never once fired. The
    /// counter writes the answer down here on its way past instead, which
    /// is what makes the pair independent of the order the card lists them
    /// in.
    pub countered_source: Option<ObjectId>,
    /// What this resolution's targets looked like when it began.
    ///
    /// CR 608.2h: an effect that needs information about an object which is
    /// no longer in the zone it was expected to be in uses that object's
    /// *last known information*. A resolution expects its targets where they
    /// were when it started, so that is when the snapshot is taken — by
    /// [`run`], once, on the first pass, which is why the field is an
    /// `Option` and not an empty `Vec` that a re-entry after a player choice
    /// would refill from the wrong moment.
    ///
    /// It is `None` at every construction site for the same reason: nothing
    /// but `run` may decide when "the resolution began" was.
    pub target_lki: Option<Vec<TargetLki>>,
}

/// One target as it last existed where the resolution expected it.
///
/// The `version` is the discriminator and the whole of the fix: `version`
/// is bumped in exactly one place, `GameState::move_object`, so "the object
/// has a different version than when this resolution started" *is* "the
/// object is no longer in the zone the effect expected it in". A reader that
/// took the snapshot unconditionally would answer with stale information for
/// the opposite shape — an effect list that changes a permanent and then
/// reads it, still on the battlefield, which Inspirit Flagship Vessel does.
#[derive(Clone, Debug)]
pub struct TargetLki {
    /// The target this describes.
    pub id: ObjectId,
    /// Its `version` when the resolution began.
    pub version: u32,
    /// Its characteristics when the resolution began.
    pub chars: crate::object::Characteristics,
}

/// What [`Filter::This`](baylee_cards_dsl::Filter::This) names right now.
///
/// The target if one was chosen; the source if the ability never asked for
/// one; and `None` — nothing at all — if it asked and got none, which is
/// what "up to one target" allows. A caller that gets `None` registers no
/// effect: there is nothing for it to apply to. See [`Resolution::targeted`].
pub(crate) fn this_object(res: &Resolution) -> Option<ObjectId> {
    match res.targets.first().copied() {
        Some(target) => Some(target),
        None if res.targeted => None,
        None => Some(res.source),
    }
}

/// The one destination Path to Exile's basic-land search uses.
static ONTO_BATTLEFIELD_TAPPED: &[baylee_cards_dsl::effect::Find] =
    &[baylee_cards_dsl::effect::Find::BATTLEFIELD_TAPPED];

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
pub enum AwaitingOp {
    /// Rotation direction, chosen by the neighbour to receive from.
    ControlRotation {
        /// Living seats in table order at the time of the choice.
        seats: Vec<PlayerId>,
    },
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
        /// **Whose library the cards came out of**, which is not always the
        /// controller's: Jace, the Mind Sculptor's +2 looks at the top card
        /// of *target player's* library and bottoms it into *that player's*
        /// library, while the controller is the one deciding. Reading
        /// `res.controller` here moved the card from one player's library
        /// into another's.
        player: PlayerId,
    },
    /// Surveil: chosen cards go to the graveyard, the rest stays on top
    /// (CR 701.25a).
    ///
    /// No `player` field beside [`Self::Scry`]'s: a surveil is always the
    /// controller's own library, and the graveyard the cards land in is the
    /// owner's whatever a card might say. See [`Effect::Surveil`] for why
    /// there is no version of this that names somebody else.
    Surveil,
    /// The controller decides whether to take an optional clause
    /// ([`Effect::MayDo`]).
    MayDo {
        /// What runs on a yes.
        effects: &'static [Effect],
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
    /// A player decides whether to pay a non-mana cost by naming what pays
    /// it ([`Effect::PlayerMayPayCostOr`]). Naming nothing is declining.
    PlayerMayPayCost {
        /// The player deciding.
        player: PlayerId,
        /// The one part they may pay.
        cost: &'static CostPart,
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
    /// After `ReturnChosenToHand`: put the chosen permanent into its
    /// owner's hand, then ask the next remaining player.
    ReturnChosen {
        /// What may be returned.
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
    /// Commanders whose owners have still to answer CR 903.9b for the
    /// operation at `res.pc`, which has not run yet.
    ///
    /// One entry per commander, because the rule "may apply more than once
    /// to the same event" and a wrath that bounces two of them is two
    /// questions to two players. The last answer resumes the operation
    /// *without* advancing the program counter: the effect was suspended
    /// before it touched anything, so it runs once, with every answer in
    /// hand.
    CommanderReplace {
        /// The commander the question on the table is about. It lives here
        /// rather than on [`Resolution`] because the answer arrives as a
        /// bare yes and has to find its card again — and a struct with
        /// seven literals in three crates is not the place to put a field
        /// only one operation reads.
        asked: ObjectId,
        /// Owners still to be asked, with the commander each is asked
        /// about and whether the library is the destination in question.
        remaining: Vec<(PlayerId, ObjectId, bool)>,
    },
}

/// Whether a search shows what it found.
///
/// The printed cards agree on a rule rather than deciding one by one: a
/// search narrower than "a card" reveals its find on the way to a hidden
/// zone, and a search that ends somewhere public does not — the card is
/// about to be visible anyway. Of the 1015 printed searches in the scripts
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

/// The first target's characteristics, as the effect asking is entitled to
/// see them (CR 608.2h).
///
/// Live while the object is still where the resolution left it, and the
/// snapshot [`run`] took once it is not. Swords to Plowshares is the card
/// that needs the second half: it exiles the creature and *then* reads its
/// power, and an object outside the battlefield reads its printed `base`,
/// because `move_object` clears the projection cache and the refresh pass
/// revisits only the battlefield and the stack. A Llanowar Elves with a
/// +1/+1 counter on it gained its controller one life instead of two — for
/// as long as the card has existed, the cache clear being older than every
/// entry that touches it.
fn target_chars<'a>(
    res: &'a Resolution,
    state: &'a GameState,
) -> Option<&'a crate::object::Characteristics> {
    let id = res.targets.first().copied()?;
    let obj = state.object(id);
    let known = res
        .target_lki
        .as_ref()
        .and_then(|lki| lki.iter().find(|l| l.id == id));
    match (obj, known) {
        // Still the same object in the same zone: nothing is lost by asking
        // it, and an effect list that changed it wants the change.
        (Some(obj), Some(known)) if obj.version == known.version => Some(obj.characteristics()),
        (_, Some(known)) => Some(&known.chars),
        (Some(obj), None) => Some(obj.characteristics()),
        (None, None) => None,
    }
}

/// Amount evaluation with target context ([`Amount::TargetPower`]).
pub(super) fn amount2(amount: &Amount, state: &GameState, you: PlayerId, res: &Resolution) -> u32 {
    match amount {
        Amount::TargetPower => target_chars(res, state)
            .and_then(|c| c.power)
            .map_or(0, |p| p.max(0) as u32),
        // Deliberately *not* through `target_chars`: the one card that reads
        // this is Reanimate, whose target is a creature card in a graveyard
        // and whose mana value is the printed one. A card that never was a
        // permanent has no last known information on the battlefield to
        // prefer.
        Amount::TargetCmc => res
            .targets
            .first()
            .and_then(|t| state.object(*t))
            .map_or(0, |o| o.characteristics().mana_cost.cmc()),
        other => eval::amount(other, state, you, res.source, res.x),
    }
}

/// The filters a continuous effect created by a **resolution** is registered
/// with (CR 611.2c).
///
/// A static ability's effect is dynamic — an anthem lifts a creature that
/// enters ten turns later, because the ability keeps applying for as long as
/// its source is there. A spell or ability that *resolves* is the opposite:
/// it happens once, and if what it does is modify characteristics or change
/// control, the objects it affects are the ones it found. So the filter is
/// read here, at the moment the effect begins, and the effect is registered
/// against the objects it named rather than against the question.
///
/// The one it cannot answer is a filter that reaches past the battlefield:
/// enumerating it would mean walking every zone the filter could mean, and a
/// narrowing to the battlefield alone would silently drop the rest. Those
/// stay dynamic, which is what they were.
///
/// One effect per object rather than one effect naming several, because an
/// [`EffectFilter`](crate::effects::EffectFilter) names exactly one — the
/// same shape `Effect::PumpTarget` already registers for a spell with two
/// targets.
/// `only` narrows the set to the seats it lists, which is the half no
/// `Filter` can do: "creatures **target player** controls" depends on a
/// choice, and a filter is told the ability's controller and its source and
/// nothing else. The seat is known here, so the set is bound here — which is
/// where CR 611.2c wants it bound anyway. A narrowed effect that could not
/// lock would have nowhere to put the seat, and no card asks for one: only
/// `Effect::PumpFilter` passes `only`, and `Modifier::ModifyPT` always
/// locks.
pub(super) fn bound_now(
    state: &GameState,
    filter: &'static baylee_cards_dsl::Filter,
    modifier: &baylee_cards_dsl::Modifier,
    you: PlayerId,
    this: ObjectId,
    only: Option<&[PlayerId]>,
) -> SmallVec<[crate::effects::EffectFilter; 4]> {
    if !crate::effects::locks_its_set(modifier) || crate::state::filter_reaches_other_zones(filter)
    {
        debug_assert!(
            only.is_none(),
            "a dynamic effect cannot carry the seat its card named",
        );
        return smallvec::smallvec![crate::effects::EffectFilter::Dsl(filter)];
    }
    state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                only.is_none_or(|seats| seats.contains(&o.controller))
                    && eval::matches(filter, state, o, you, this)
            })
        })
        .map(|id| crate::effects::EffectFilter::object(state, *id))
        .collect()
}

/// The seats a [`PlayerRel`] names *during a resolution*.
///
/// [`eval::players`] answers the half that the state alone can answer, and
/// deliberately answers `None` for the two relations that need the
/// resolution's own context: `Chosen` is the player this spell or ability
/// targeted, `ControllerOfTarget` is read off its first object target. An
/// effect that reaches for `eval::players` directly therefore does *nothing*
/// on a card that says "target opponent" — which is exactly how Abraded
/// Bluffs shipped as a land that deals no damage. **Every effect inside a
/// resolution asks this function**, whatever relation the card names; the
/// `other` arm below is the only place in the module that may unwrap the
/// state-only half, and its `expect` is unreachable because the two context
/// relations are matched above it.
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
        other => eval::players(other, state, you)
            .expect("the two context relations are matched above this arm"),
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
    // The moment CR 608.2h measures from. `run` is re-entered after every
    // suspended choice, so this has to be the *first* entry and not any
    // entry, which is what the `Option` says.
    if res.target_lki.is_none() {
        res.target_lki = Some(
            res.targets
                .iter()
                .filter_map(|&id| {
                    state.object(id).map(|o| TargetLki {
                        id,
                        version: o.version,
                        chars: o.characteristics().clone(),
                    })
                })
                .collect(),
        );
    }
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

/// Puts CR 903.9b's question before an operation that is about to move
/// cards into hands or libraries, and suspends if anyone has to answer it.
///
/// `moves` is what the operation is about to hand to
/// [`GameState::move_object`], destination included — the same pairs, not a
/// summary of them, so what the player is asked about cannot drift from
/// what actually moves.
///
/// **Call this before the operation mutates anything.** The last answer
/// re-enters the operation at the same program counter, so an effect that
/// had already flipped a card's kind or dealt its damage would do it twice.
pub(super) fn ask_commander_replace(
    state: &GameState,
    res: &mut Resolution,
    moves: &[(ObjectId, ZoneLocation)],
) -> Option<Pending> {
    let mut remaining: Vec<(PlayerId, ObjectId, bool)> = Vec::new();
    for &(id, to) in moves {
        // Already answered: this is the second visit, the one that runs.
        if state.commander_redirect.iter().any(|(o, _)| *o == id) {
            continue;
        }
        if let Some(owner) = state.commander_owner(id, to) {
            remaining.push((owner, id, matches!(to, ZoneLocation::Library(_))));
        }
    }
    let (asked, pending) = next_commander_ask(state, &mut remaining)?;
    res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
    Some(pending)
}

/// Takes the next owner off the list and builds their question.
fn next_commander_ask(
    state: &GameState,
    remaining: &mut Vec<(PlayerId, ObjectId, bool)>,
) -> Option<(ObjectId, Pending)> {
    if remaining.is_empty() {
        return None;
    }
    let (player, card, to_library) = remaining.remove(0);
    let pending = Pending::YesNo {
        player,
        prompt: YesNoPrompt::CommanderReplace { card, to_library },
        // A card-scoped handle, so "always send Katara home" is an answer a
        // seat can keep — and its own reserved index, because agreeing to
        // that for a graveyard is not agreeing to it for a bounce.
        source: state.object(card).and_then(|o| o.card).map(|c| {
            baylee_core::ids::AbilityRef::new(
                c.index,
                baylee_core::ids::AbilityRef::COMMANDER_REPLACE,
            )
        }),
    };
    Some((card, pending))
}

/// Resumes a yes/no choice (shockland payment and friends).
///
/// # Panics
/// When the suspended operation is not a yes/no choice.
#[must_use]
pub fn resume_yes_no(state: &mut GameState, res: &mut Resolution, answer: bool) -> Flow {
    // CR 903.9b: record what this owner said, then either ask the next one
    // or run the operation that has been waiting for all of them.
    if matches!(res.awaiting, Some(AwaitingOp::CommanderReplace { .. })) {
        let Some(AwaitingOp::CommanderReplace {
            asked,
            mut remaining,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        state.commander_redirect.push((asked, answer));
        if let Some((asked, pending)) = next_commander_ask(state, &mut remaining) {
            res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
            return Flow::Wait(pending);
        }
        // `take` above already cleared `awaiting`, and no `pc += 1` here:
        // the operation this was asked for has not run yet.
        return run(state, res);
    }
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
    } else {
        state.set_tapped(res.source, true);
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes an optional clause ([`Effect::MayDo`]): `yes` means the
/// controller takes it.
///
/// A no is not a failure and runs no fallback — the clause simply does not
/// happen and the rest of the ability carries on, which is what makes this
/// different from [`resume_tax_choice`], where declining *is* an outcome
/// the card prints.
///
/// # Panics
/// When the suspended operation is not an optional clause.
#[must_use]
pub fn resume_may_do(state: &mut GameState, res: &mut Resolution, yes: bool) -> Flow {
    let AwaitingOp::MayDo { effects } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_may_do on a choice that is not an optional clause");
    };
    if yes && let Some(pending) = run_nested(state, res, effects) {
        return Flow::Wait(pending);
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
    run_fallback(state, res, effect)
}

/// Runs the branch an "unless" effect takes when the player does not pay.
///
/// Shared by the two shapes of that effect rather than written twice: a
/// fallback that suspended on a choice has to splice its own remaining
/// program into the resolution that called it, and a second copy of that
/// splice would be a second place for the program counter to be wrong.
fn run_fallback(state: &mut GameState, res: &mut Resolution, effect: &'static Effect) -> Flow {
    if let Some(pending) = run_nested(state, res, std::slice::from_ref(effect)) {
        return Flow::Wait(pending);
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
            // The shuffle comes **before** the cards are placed, and that is
            // the whole of what Mystical Tutor prints: "search your library
            // for an instant or sorcery card, reveal it, then shuffle and put
            // that card on top." Shuffling afterwards put the found card on
            // top and then shuffled it straight back in, so the tutor
            // returned a random card to the top of the library — which is
            // every tutor-to-top in the pool.
            //
            // For a find that leaves the library (Cultivate's battlefield and
            // hand) the order is unobservable: the card is gone either way,
            // and the rest is a shuffled library in both readings.
            state.shuffle_library(res.controller);
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
                        }
                        if tapped {
                            state.set_tapped(card, true);
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
        }
        AwaitingOp::Scry { player } => {
            // Chosen cards go to the bottom in chosen order; the rest stays
            // on top in its original relative order (scry approximation).
            // The library is the one they were looked at in — see the
            // variant's own doc.
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(player),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::Surveil => {
            // Chosen cards go to their owner's graveyard; the rest stays on
            // top in its original relative order (CR 701.25a says "in any
            // order", and this is the same approximation the scry above
            // makes). The owner and not the controller: a card only ever
            // goes to its owner's graveyard, and a surveil that took a
            // stolen card would still put it back where it came from.
            for &card in chosen {
                let owner = state.object(card).map_or(res.controller, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
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
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
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
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
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
        AwaitingOp::ReturnChosen { filter, remaining } => {
            if let Some(&returned) = chosen.first() {
                // CR 400.3: its owner's hand, whoever was controlling it.
                let owner = state.object(returned).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(returned) {
                    obj.kind = ObjectKind::Card;
                }
                // No `ask_commander_replace` here, and this arm is the only
                // one of the three that would ever want it: CR 903.9b is a
                // replacement for a hand or a library, while a commander
                // reaching a *graveyard* is CR 903.9a, a state-based action
                // — so the two siblings, which both end in a graveyard, have
                // nothing to ask.
                //
                // What stops it is the shape rather than the rule. The
                // question re-enters its operation at the same program
                // counter with nothing yet mutated (`resume_yes_no` returns
                // `run` with no `pc += 1`), which a start block can survive
                // and a continuation cannot: the operation here is the whole
                // per-player chain, so the re-run would ask the first player
                // to choose all over again. Unreachable today — the only
                // filter the pool writes for this effect is `Land.YouCtrl`
                // and no commander is a land — and listed in
                // `docs/engine-internals.md` beside the other paths the rule
                // does not reach, so the day a card writes
                // `Creature.YouCtrl` here it is a known gap rather than a
                // surprise.
                let _ = state.move_object(
                    returned,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::ReturnChosen { filter, remaining });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                });
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
        AwaitingOp::ControlRotation { .. }
        | AwaitingOp::ManaChoice { .. }
        | AwaitingOp::PayLifeOrTapSelf { .. }
        | AwaitingOp::MayDo { .. }
        | AwaitingOp::CommanderReplace { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
        AwaitingOp::PlayerMayPay { .. } => {
            unreachable!("tax choices resume via resume_tax_choice")
        }
        AwaitingOp::PlayerMayPayCost {
            player,
            cost,
            effect,
        } => {
            // Naming nothing is declining. `min: 0` is what makes the
            // question a "may", so an empty answer is the not-paid branch
            // rather than an error — and it is the only shape that can
            // express "I could pay and would rather not", which a `YesNo`
            // followed by a second question could not without asking twice.
            //
            // A payment that fails takes the fallback as well. It is the
            // same reading as `resume_tax_choice`: the answer was legal
            // when it was offered, so a refusal here is a rules outcome
            // and never a reason to abandon the resolution.
            let paid = chosen
                .first()
                .is_some_and(|&chosen| cost_wizard::pay(state, player, cost, chosen).is_ok());
            if !paid {
                return run_fallback(state, res, effect);
            }
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
        | Effect::Surveil { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
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
                    && state.is_opponent(fx.controller, you)
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
                        && state.is_opponent(fx.controller, you)
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
                // Hidden zone: failing to find is always legal (CR 701.23b).
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
            let player = players_of(player, state, you, res).first().copied()?;
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
            res.awaiting = Some(AwaitingOp::Scry { player });
            // Two players, two roles. Jace's +2 prints "Look at the top card
            // of **target player's** library. **You** may put that card on
            // the bottom of **that player's** library" — so the library is
            // the target's and the decision is the controller's. Asking
            // `player` handed the opponent the choice of whether to keep
            // their own card, which is the opposite of what the card does.
            Some(Pending::ChooseCards {
                player: you,
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
            res.awaiting = Some(AwaitingOp::Scry { player: you });
            Some(Pending::ChooseCards {
                player: you,
                options: looked,
                min: 0,
                max: n as u8,
                prompt: ChoicePrompt::ScryBottom,
            })
        }
        Effect::Surveil { amount } => {
            let n = eval::amount(&amount, state, you, res.source, res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            // CR 701.25c: surveil 0 is not a surveil event at all, and an
            // empty library is the same nothing. Returning `None` here is
            // what makes that true — the resolution simply goes on.
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Surveil);
            Some(Pending::ChooseCards {
                player: you,
                options: looked,
                min: 0,
                max: n as u8,
                prompt: ChoicePrompt::SurveilGraveyard,
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
            // `players_of`, not `eval::players`: ward names the *caster*
            // (`ControllerOfTarget`), which the state alone cannot answer.
            let player = players_of(player, state, you, res).first().copied()?;
            // Evaluated here rather than written into the card, because
            // Esper Sentinel's tax is its own power and a creature's power
            // is not known until the ability resolves. `u16` is what the
            // prompt and the suspended op carry; the clamp is a formality
            // (no power in the pool is near it) and not a rules choice.
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            // The question is put whether or not the mana is already
            // floating, because CR 605.3a lets the player make it now: a
            // mana ability may be activated "whenever a rule or effect asks
            // for a mana payment, even if it's in the middle of casting or
            // resolving a spell". This used to run the fallback outright
            // against an empty pool and never ask at all, which is the
            // wrong outcome for the six cards in this pool that tax an
            // opponent — an opponent who has usually just tapped out to
            // cast the very spell being taxed. Three tests worked around it
            // by seating extra lands and said so in their own comments.
            //
            // Whether the pool covers it is decided when the answer comes
            // back, in `Engine::apply`, which is where a window can be
            // opened; nothing here can open one, because a `Resolution` has
            // no access to the engine's priority machinery.
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
        Effect::PlayerMayPayCostOr {
            player,
            cost,
            effect,
        } => {
            // `players_of` for `PlayerMayPayOr`'s reason: the payer is named
            // relative to the ability, and a Karoo names its own controller
            // where a ward names the caster.
            let player = players_of(player, state, you, res).first().copied()?;
            let options = cost_wizard::options(state, player, res.source, cost);
            if options.is_empty() {
                // No legal answer, so no question: a Karoo under a player
                // with no other land to return sacrifices itself, and
                // asking would be a prompt with one button on it. The
                // fallback runs inline through the same door a declined
                // payment takes.
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayCost {
                player,
                cost,
                effect,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: cost_wizard::prompt(cost),
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
                    && state.is_opponent(fx.controller, you)
            }) {
                return None;
            }
            let player = players_of(player, state, you, res).first().copied()?;
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
        Effect::MayDo { effects } => {
            res.awaiting = Some(AwaitingOp::MayDo { effects });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::PayLifeOrEnterTapped { amount } => {
            // Not payable at all → no choice, enters tapped (CR 614.1c).
            if !state.can_pay_life(you, i32::from(amount)) {
                state.set_tapped(res.source, true);
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
        // No card, no handle: a token's, a token copy's and an emblem's
        // ability is addressed by nothing a standing answer could be filed
        // under, and saying so is the whole point of the `Option`. It used
        // to answer with card index 0, so one "always say yes" would have
        // covered every such ability at once — and a real card besides.
        return loc.card.map(|card| AbilityRef::new(card, loc.index));
    }
    // A spell resolving: what it does is its spell ability, which is not
    // an entry in the card's ability list.
    obj.card
        .map(|c| AbilityRef::new(c.index, AbilityRef::SPELL))
}

/// Runs a nested branch (If*/kicked-style conditional effects) inline;
/// a suspension inside the branch splices its remaining ops into the
/// parent's program and propagates the choice.
///
/// The splice *replaces* the parent's op and starts at the nested program
/// counter, and both halves of that are load-bearing. Inserting in front of
/// the parent's op instead left the branch itself standing in the program,
/// so resuming ran back into it and asked the same question a second time —
/// Luminarch Ascension offered its quest counter twice and took two. And
/// splicing the whole nested program rather than its tail would re-run the
/// ops the branch had already finished before it suspended. Neither could
/// be seen until an effect that suspends was put inside a branch, which is
/// what [`Effect::MayDo`] did.
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
        targeted: res.targeted,
        awaiting: None,
        mana_ability: false,
        countered_source: res.countered_source,
        target_lki: None,
    };
    match run(state, &mut nested) {
        Flow::Complete => None,
        Flow::Wait(pending) => {
            res.awaiting = nested.awaiting;
            let tail = nested.effects.split_off(nested.pc);
            res.effects.splice(res.pc..=res.pc, tail);
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
        | Effect::ReturnChosenToHand { .. }
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
            // Two readings that were both wrong, and each looked right from
            // the other side. The seat was *always* the effect's controller,
            // so `new_controller` was a field a card could write and nothing
            // would read — Wishclaw Talisman's "An opponent gains control of
            // this artifact" is the whole price of a repeatable tutor, and it
            // was handing the artifact back to the player who activated it.
            // And the object was always `res.targets.first()`, so an ability
            // saying "this artifact" rather than "target permanent" changed
            // the control of nothing at all and resolved quietly.
            //
            // `players_of` for `PlayerMayPayOr`'s reason: a seat named
            // relative to the ability is not a seat the state alone can
            // answer. A relation nobody is (no opponent left) changes
            // nothing, which is the honest outcome and not a panic.
            //
            // `.first()` and not a question: at a duel a relation naming an
            // opponent names exactly one seat, and the only card in this
            // pool writing this effect prints "an opponent" — which at three
            // seats is a choice the controller announces on resolution (CR
            // 608.2d). Taking the first is a duel assumption and is
            // wrong at a bigger table; it is written down here rather than
            // guessed at, because the fix is a `Pending` and not an index.
            let subject = res.targets.first().copied().unwrap_or(res.source);
            if let Some(&seat) = players_of(new_controller, state, you, res).first() {
                change_controller(state, subject, seat);
            }
            None
        }
        Effect::ControlRotation => control::ask(state, res),
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
            let n = amount2(&amount, state, you, res) as usize;
            state.draw_cards(you, n);
            None
        }
        Effect::DrawCardsFor { amount, who } => {
            let n = amount2(&amount, state, you, res) as usize;
            for player in players_of(who, state, you, res) {
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
            // with new < old is a life loss (CR 119.3 note).
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
        Effect::IfNoCountersOnSelf { kind, then } => {
            // `is_some_and` and not `map_or(true, …)`: a source that is no
            // longer on the battlefield has not run out of counters, it has
            // stopped being a thing the sentence is about.
            if state
                .object(res.source)
                .is_some_and(|o| o.counters.get(kind) == 0)
            {
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
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                // Nothing to become anything: the ability said "target" and
                // was activated with none, so this half of its sentence has
                // no subject and registers nothing.
                let this = this_object(res)?;
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                bound_now(state, filter, &modifier, you, res.source, None)
            };
            let timestamp = state.next_timestamp();
            for filter in filters {
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
            }
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
                    reason: TargetPrompt::Targets,
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
                obj.own_abilities = Some(abilities);
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
                    filter: crate::effects::EffectFilter::object(state, target),
                    modifier: baylee_cards_dsl::Modifier::GrantsFlashback,
                });
            }
            None
        }
        Effect::TapTarget => {
            for &target in &res.targets.clone() {
                state.set_tapped(target, true);
            }
            None
        }
        Effect::UntapTarget => {
            for &target in &res.targets {
                untap(state, target);
            }
            None
        }
        // "Untap this artifact." The source and not a target, so nothing is
        // chosen and nothing can be made an illegal choice; a source that has
        // left the battlefield untaps nothing, which `untap` answers by
        // finding no object rather than by a check here.
        Effect::UntapSelf => {
            untap(state, res.source);
            None
        }
        Effect::TargetSourceLosesAbilities { source_filter } => {
            // The permanent an earlier effect of this same resolution took
            // an ability off. Reading `res.targets` here instead is what
            // made the whole rider dead code: the ability it names has been
            // removed from the arena by then.
            if let Some(src) = res.countered_source {
                let applies = state.object(src).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && eval::matches(source_filter, state, o, you, res.source)
                });
                if applies {
                    let ts = state.next_timestamp();
                    state.effects.register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(res.source),
                        controller: you,
                        layer: baylee_cards_dsl::Layer::Ability,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                        filter: crate::effects::EffectFilter::object(state, src),
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
                    // CR 707.10: a copy is put on the stack, not cast from
                    // hand. Rebound must not schedule a vanished copy.
                    obj.cast_from_hand = false;
                    obj.targets = targets;
                    obj.target_req = target_req;
                    obj.zone = crate::zone::Zone::Stack;
                    // CR 704.5e: it stops existing the moment it is anywhere
                    // but the stack or the battlefield. Carrying the copied
                    // card is what makes the marker necessary — without it
                    // the copy resolved into a graveyard and stayed there as
                    // a second, real card.
                    obj.riders.push(crate::object::Rider::SpellCopy);
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
                            reason: TargetPrompt::Targets,
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
                // An Equipment grants through `Filter::AttachedToBySource`,
                // so what it is attached to is an input to the layer
                // projection. This is the attaching write; the SBA unattach
                // in `sba.rs` is the other, and both have to bump the
                // generation or the cached characteristics stay valid and
                // the equipped creature keeps none of the keywords.
                state.invalidate_projections();
            }
            None
        }
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
    }
}

/// Untaps one permanent and says so.
///
/// One door for both untap **effects**, and it exists because the two used
/// to disagree with the third. The untap step journals
/// `GameEvent::ObjectUntapped` (CR 502.3); `Effect::UntapTarget` wrote the
/// bit and journalled nothing, so an untap from an effect was invisible to
/// anything reading the game's own record of what happened — a replay, a
/// client's log, and any "becomes untapped" trigger the DSL might learn.
///
/// That was latent rather than broken: `AbilityDef::Trigger` has
/// `BecomesTapped` and no untapped twin, so nothing could have been
/// listening. Latent is the state a rule is in just before somebody adds the
/// trigger and cannot work out why it never fires, so the fix comes with the
/// second effect rather than after it, and
/// `card_tests::lands::deserted_temple_says_so_when_it_untaps_a_land` is
/// what holds it — a test on the *older* variant, because that is the one
/// that was wrong.
///
/// A permanent that is already untapped is left alone and journals nothing:
/// untapping an untapped permanent is not an event, and recording one would
/// put a "became untapped" in the log for a permanent that did not.
fn untap(state: &mut GameState, id: ObjectId) {
    let tapped = state
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED));
    if !tapped {
        return;
    }
    state.set_tapped(id, false);
    state.journal.record(GameEvent::ObjectUntapped {
        object: id,
        cause: Cause::Effect,
    });
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

#[cfg(test)]
mod bound_now_tests {
    use super::*;
    use crate::effects::EffectFilter;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Filter, Modifier, ZoneRef};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        GameState::from_preset(&preset(11, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game")
    }

    /// A permanent on the battlefield with a controller and nothing else.
    ///
    /// Bare because what `bound_now` reads is the battlefield list and the
    /// controller, and a printed card would put four other characteristics
    /// in front of the one question being asked.
    fn permanent(state: &mut GameState, seat: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// The ids a bound set names, in the order it named them.
    fn named(bound: &[EffectFilter]) -> Vec<ObjectId> {
        bound
            .iter()
            .map(|f| match f {
                EffectFilter::ObjectIs(id, _) => *id,
                EffectFilter::Dsl(filter) => panic!("a dynamic filter in a bound set: {filter:?}"),
            })
            .collect()
    }

    /// CR 611.2c: an effect from a *resolution* that modifies
    /// characteristics affects the objects it found and no others. So the
    /// filter is read here, once, and the effect is registered against each
    /// object it named — one `EffectFilter` per object, because an
    /// `EffectFilter` names exactly one.
    ///
    /// The permanent created afterwards is the whole point: the same call
    /// made a moment later names it, and the set already bound does not.
    #[test]
    fn a_locking_modifier_binds_the_board_it_found() {
        let mut state = state();
        let first = permanent(&mut state, me(), "First");
        let second = permanent(&mut state, me(), "Second");

        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(-2, -2),
            me(),
            first,
            None,
        );
        assert_eq!(
            named(&bound),
            vec![first, second],
            "one per permanent, in battlefield order"
        );

        let latecomer = permanent(&mut state, me(), "Latecomer");
        assert!(
            !named(&bound).contains(&latecomer),
            "\"all creatures get -2/-2 until end of turn\" leaves a creature \
             that arrives afterwards alone"
        );
        let again = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(-2, -2),
            me(),
            first,
            None,
        );
        assert_eq!(
            named(&again),
            vec![first, second, latecomer],
            "and it is the moment that bound the set, not the filter"
        );
    }

    /// The set is bound by **identity** and not by id, which is the pair
    /// `EffectFilter::object` exists to write: a permanent that leaves and
    /// comes back keeps its id and is a new object (CR 400.7), so the
    /// version the set recorded is the one it was bound at.
    #[test]
    fn a_bound_set_records_the_version_it_saw() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Bear");
        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            bear,
            None,
        );
        let object = state.object(bear).expect("just made it");
        assert!(bound[0].names(object), "the object it was bound against");

        state
            .move_object(
                bear,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("it blinks out");
        state
            .move_object(
                bear,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("and back");
        assert!(
            !bound[0].names(state.object(bear).expect("same id")),
            "and not whatever is at that id later"
        );
    }

    /// A modifier that changes neither characteristics nor control has no
    /// set to lock (CR 611.2c), so it stays the question it was written as:
    /// one dynamic filter, which keeps catching whatever arrives.
    #[test]
    fn a_modifier_that_locks_nothing_stays_a_question() {
        let mut state = state();
        permanent(&mut state, me(), "Present");
        let source = permanent(&mut state, me(), "Source");

        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::DoesNotUntap,
            me(),
            source,
            None,
        );
        assert_eq!(bound.len(), 1);
        assert!(
            matches!(bound[0], EffectFilter::Dsl(f) if *f == Filter::Any),
            "the filter it was written with, unread"
        );
    }

    /// A filter that reaches past the battlefield stays dynamic whatever
    /// the modifier does: enumerating it would mean walking every zone the
    /// filter could mean, and narrowing to the battlefield alone would
    /// silently drop the rest.
    #[test]
    fn a_filter_that_leaves_the_battlefield_is_not_enumerated() {
        static ELSEWHERE: Filter =
            Filter::And(&[Filter::CREATURE, Filter::InZone(ZoneRef::Graveyard)]);
        let mut state = state();
        let source = permanent(&mut state, me(), "Source");

        let bound = bound_now(
            &state,
            &ELSEWHERE,
            &Modifier::ModifyPT(1, 1),
            me(),
            source,
            None,
        );
        assert_eq!(bound.len(), 1);
        assert!(
            matches!(bound[0], EffectFilter::Dsl(f) if *f == ELSEWHERE),
            "a locking modifier, and still the question"
        );
    }

    /// `only` is the half no `Filter` can do: "creatures target player
    /// controls" depends on a choice, and a filter is told the ability's
    /// controller and its source and nothing else. The seat is known at
    /// resolution, which is where CR 611.2c wants the set bound anyway.
    #[test]
    fn a_named_seat_narrows_the_set_no_filter_could() {
        let mut state = state();
        let mine = permanent(&mut state, me(), "Mine");
        let theirs = permanent(&mut state, them(), "Theirs");

        let both = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            mine,
            None,
        );
        assert_eq!(named(&both), vec![mine, theirs]);

        let narrowed = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            mine,
            Some(&[them()]),
        );
        assert_eq!(
            named(&narrowed),
            vec![theirs],
            "the seat the card named, and not the one that cast it"
        );
    }
}
