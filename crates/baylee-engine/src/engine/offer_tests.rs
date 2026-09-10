//! Every ability the engine offers is one it will accept.
//!
//! `LegalActions` is an *enumeration*, not a hint: the engine lists what a
//! seat may do right now and `apply` validates the answer against that same
//! list, so a pair that appears in `abilities` and is then refused by
//! `apply` is the engine contradicting itself. A client cannot recover from
//! it — the action it was told to offer is the action it is punished for
//! sending — and a player sees a lit permanent that does nothing.
//!
//! One live case of exactly that shape was found by hand: four lands
//! printing "target player", where the offer counted the objects *and* the
//! seats the spec could point at (CR 115.4 makes that one choice over two
//! lists) and the activation counted only the objects. All four were
//! `Coverage::Implemented`, which is the part that matters — that is what
//! the deckbuilder hands a player as playable.
//!
//! The bug found beside it is deliberately *not* this shape, and this module
//! would not catch it: a prepared cast was offered at instant speed and
//! activated at instant speed, the two halves agreeing and both wrong. Two
//! probes that agree can still be wrong about the rules, which is what a
//! behavioural card test is for. This module answers the narrower question
//! of whether they agree at all.
//!
//! The AI-vs-AI soak in `baylee-gamehost` cannot reach this class. Its probe
//! plays a game per card with `HeuristicAgent`, so an ability the heuristic
//! never chooses is never activated: reintroducing the land bug leaves that
//! sweep green. This module takes the agent out of the question — it presses
//! every button the engine draws, whether or not anything would want to.
//!
//! That includes the two offers printed on no card: measured on the pool as
//! it stands, the sweep reaches `choice::GRANTED_ABILITY` on Urza's Saga and
//! `choice::PREPARED_CAST` on Emeritus of Woe. Those are the indices a client
//! cannot look up on a face, so an offer it cannot honour is the one a player
//! has no way to make sense of.

use super::testkit::*;
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::types::TypeSet;

/// The seed every probe runs at. One fixed seed rather than the card's own
/// index: what is under test is a question asked of a board, and the board
/// is built by hand, so the only thing a varying seed would move is which
/// seat takes the first turn.
const SEED: u64 = 4_211;

/// The floor under the number of cards that had *any* deed to press,
/// asserted so the sweep cannot quietly become a no-op.
///
/// This is the lesson the gamehost soak taught: a pool-wide test that
/// reaches nothing passes exactly as loudly as one that reaches everything.
/// The number is 464 measured on 2026-09-10, less a tenth, so ordinary pool
/// growth never touches it and a setup change that stops part of the pool
/// from arriving at its main phase fails here instead of silently.
///
/// It is now every implemented card the engine draws a button for on this
/// board: no board is abandoned on the way in and none is abandoned tapping
/// its mana. What is left out is *not* simply "cards with no activated
/// ability". An ability whose cost `can_afford` refuses to offer is a card
/// that arrives, is asked, and answers with nothing — counted here exactly
/// as a vanilla creature is. Recurring Nightmare is that shape (a sacrifice
/// cost is a choice an activation cannot make yet), and it left the
/// implemented pool by hand rather than by anything this number can see.
const COVERAGE_FLOOR: usize = 417;

/// The floor under the deeds driven all the way back to a quiet priority.
///
/// Pressing a button and watching the engine refuse the very answer it just
/// enumerated is the point of the drive, and a drive that stops at the first
/// question it cannot answer checks nothing past the press. This is the
/// second half of [`COVERAGE_FLOOR`]: that one says the sweep still reaches
/// the pool, this one says it still gets through it. Every one of the 658
/// deeds rested when this was measured on 2026-09-10, with nothing stalled
/// and no question the driver could not answer; the floor is that less a
/// tenth.
const RESTED_FLOOR: usize = 592;

/// One thing an object was offered, in the two lists an offer can live in.
///
/// A mana ability belongs here on the same footing as any other: CR 605.1
/// only says it skips the stack, and `mana_abilities` is an enumeration in
/// exactly the same sense — the client's mana planner sends straight off it.
#[derive(Clone, Copy, Debug)]
enum Deed {
    /// `LegalActions::mana_abilities` named this object.
    Mana,
    /// `LegalActions::abilities` named this object and this index.
    Ability(u32),
}

impl Deed {
    fn action(self, source: ObjectId) -> PlayerAction {
        match self {
            Self::Mana => PlayerAction::ActivateManaAbility { source },
            Self::Ability(ability_index) => PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        }
    }
}

/// Four of each registered basic: twenty mana of every colour, which is
/// enough that affordability never decides whether an ability is offered.
///
/// It has to be enough, because an ability the engine declines to offer for
/// want of mana is an ability this sweep never presses.
fn basics() -> Vec<CardIndex> {
    let mut field = Vec::new();
    for basic in baylee_cards::decks::basic_lands().into_iter().flatten() {
        field.extend(std::iter::repeat_n(basic, 4));
    }
    field
}

/// The quietest creature in the pool.
///
/// Twenty basics cannot supply two things a probe board wants: a creature to
/// sacrifice or to point at, and a creature *card* in a graveyard to bring
/// back. One card serves both, and Llanowar Elves is the one that brings
/// least of its own — its whole text is one mana ability, so no trigger
/// fires, no static applies and no keyword changes what anything else on the
/// board can do.
///
/// The pool has no vanilla creature at all: the only implemented creatures
/// with an empty `abilities` list are the five werewolves, and daybound
/// would put every probe board into a *game state* (CR 731) the card under
/// test can read. One mana ability nothing presses is the smaller footprint
/// — and nothing presses it, because [`deeds`] only ever looks at the
/// probed card's own objects.
fn quiet_creature() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Whether a card's front face is something that can sit on a battlefield.
///
/// The same five types `decks::probe_preset` asks about, and for the same
/// reason: `starting_battlefield` puts a card there without casting it, so
/// handing it an instant would be building a board no game could reach. A
/// card that fails this is still probed — in hand, where cycling and its
/// relatives live (`ActivationZone::Hand`).
fn is_permanent(def: &baylee_cards_dsl::CardDef) -> bool {
    def.faces.first().is_some_and(|f| {
        f.types.contains(TypeSet::LAND)
            || f.types.contains(TypeSet::CREATURE)
            || f.types.contains(TypeSet::ARTIFACT)
            || f.types.contains(TypeSet::ENCHANTMENT)
            || f.types.contains(TypeSet::PLANESWALKER)
    })
}

/// Answers mulligans, priorities and empty combat declarations until `seat`
/// holds priority in its own first main phase.
///
/// Tolerant where [`reach_main_phase`] panics: this walker is *setup* for
/// hundreds of unrelated cards, and a board that asks a question it does not
/// know how to answer on the way is a card this sweep cannot probe rather
/// than a failure. The distinction is kept honest by [`COVERAGE_FLOOR`],
/// which notices when "cannot probe" starts meaning "most of the pool".
///
/// Three of those questions are asked by a permanent *on the way in*, and
/// the walker answers them rather than abandoning the board — twenty-three
/// implemented cards, the ten shocklands among them, had no probe at all
/// until it did. What it must not do is answer them the way
/// [`drive_to_rest`] does: the driver is exercising an effect and takes an
/// option wherever one is offered, and this is setup, which wants the card
/// the pool prints. So an optional choice is **declined** — exactly `min`
/// targets, which is none of them for the copy clause on Phantasmal Image
/// and its relatives, because a Cursed Mirror that entered as a copy of
/// something else is not the card the sweep came to probe.
///
/// The shockland's question is the one that has no "decline": both answers
/// are legal and one of them puts the land onto the battlefield tapped,
/// where it offers nothing and the probe is pointless. It is answered yes,
/// which is also the answer that leaves the land in the state every other
/// land on this board is already in.
fn walk_to_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    for _ in 0..60 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return true;
        }
        let refused = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).is_err()
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).is_err()
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .is_err(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .is_err(),
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: options.into_iter().take(usize::from(min)).collect(),
                        players: Vec::new(),
                    },
                )
                .is_err(),
            Pending::ChooseSubtype { player, options } => match options.first().copied() {
                Some(first) => engine
                    .apply(player, PlayerAction::ChooseSubtype(first))
                    .is_err(),
                None => return false,
            },
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { .. },
                ..
            } => engine.apply(player, PlayerAction::YesNo(true)).is_err(),
            _ => return false,
        };
        if refused {
            return false;
        }
    }
    false
}

/// A board holding `card` in both zones an ability can be activated from —
/// seat 0's hand and, when it is a permanent, seat 0's battlefield — beside
/// twenty basics and two [`quiet_creature`]s, one of which is put into the
/// graveyard before anything is asked, walked to seat 0's first main phase
/// with every basic tapped for mana.
///
/// The card's own permanent is deliberately left untapped
/// ([`tap_mana_except`]'s reason): one whose interesting ability costs `{T}`
/// would otherwise have spent itself paying for the mana that pays for it.
/// The surviving Elf is left untapped for a different reason — it is there
/// to be a creature, not to make mana.
///
/// Twenty basics answer only "can this be afforded". The pair of Elves
/// answers "is there anything for it to reach": an Equipment has nothing to
/// equip on a board of lands, and a land that returns a creature card from a
/// graveyard has nothing to return from an empty one. Neither was offered
/// anything to press before, so neither was probed at all.
///
/// Returns the engine, the card's objects (battlefield first, then hand —
/// a fixed order, so a deed found on one board addresses the same object on
/// the next), and the offer standing at that moment. `None` when the board
/// never got there.
fn probe(card: CardIndex) -> Option<(Engine<RegistryLookup>, Vec<ObjectId>, LegalActions)> {
    let seat = PlayerId::new(0);
    let def = baylee_cards::by_index(card)?;
    let elf = quiet_creature();
    let mut field = basics();
    if is_permanent(def) {
        field.insert(0, card);
    }
    field.extend([elf, elf]);
    let filler = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .next()
        .unwrap_or(card);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &field)
        .hand(0, &[card])
        .start();
    // One of the two Elves into the graveyard, before anything is asked. It
    // is the *last* of them and not the first because the card under test may
    // itself be Llanowar Elves and `on_battlefield` answers with the first
    // match: the probed permanent has to be the one that survives.
    let doomed = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .rfind(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == elf))
        })?;
    sba::destroy(engine.dev_state_mut(seat)?, doomed);
    if !walk_to_own_main(&mut engine, seat) {
        return None;
    }
    let survivor = on_battlefield(&engine, seat, elf);
    let permanent = on_battlefield(&engine, seat, card);
    let objects: Vec<ObjectId> = permanent
        .into_iter()
        .chain(in_hand(&engine, seat, card))
        .collect();
    if objects.is_empty() {
        return None;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    for source in legal.mana_abilities {
        if Some(source) == permanent || Some(source) == survivor {
            continue;
        }
        // A mana ability the engine offered and then refused is the same
        // contradiction this module is about, and the board is abandoned
        // rather than unwrapped through — which is tolerance in the same
        // sense as `walk_to_own_main`'s, and cost exactly as much. It used
        // to say that these are the basics and a basic that cannot tap for
        // mana is the mana tests' problem. They are not always the basics:
        // Chromatic Lantern and Great Divide Guide grant every land a second
        // mana ability, the same land came back in this list twice, and the
        // second press was refused. Both cards left the sweep here, silently,
        // and it took reading the delta to notice. See
        // [`a_permanent_that_makes_mana_two_ways_is_offered_once`].
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .ok()?;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    Some((engine, objects, *legal))
}

/// Everything `legal` offers to do with `objects`, each named by its
/// *position* in that slice rather than by its `ObjectId`, so the answer
/// survives being carried to a freshly built board.
fn deeds(legal: &LegalActions, objects: &[ObjectId]) -> Vec<(usize, Deed)> {
    let mut found = Vec::new();
    for (slot, object) in objects.iter().enumerate() {
        if legal.mana_abilities.contains(object) {
            found.push((slot, Deed::Mana));
        }
        found.extend(
            legal
                .abilities
                .iter()
                .filter(|(source, _)| source == object)
                .map(|(_, index)| (slot, Deed::Ability(*index))),
        );
    }
    found
}
/// Where driving one activation to a rest ended.
///
/// Every answer the driver gives is one the question itself enumerated, so
/// [`Rest::Refused`] is the offer contradiction one question further in: the
/// engine listed an answer and then rejected it.
#[derive(Debug)]
enum Rest {
    /// Back to a quiet priority: the stack is empty and the seat holds it.
    Reached,
    /// The game ended on the way, which is a rest of its own.
    Over,
    /// A pending this driver has no arm for. Not a failure — the driver is
    /// deliberately not a second house AI — but counted, because "cannot
    /// answer this one" growing into "cannot answer anything" is how a sweep
    /// dies quietly.
    Unanswered(&'static str),
    /// The step cap ran out.
    Stalled,
    /// An answer taken from the enumeration was refused.
    Refused(String),
}

/// Answers whatever the engine asks — always out of what the question itself
/// enumerated — until the activation has resolved and `seat` is back at a
/// quiet priority.
///
/// This is the offer invariant carried through the wizard. `LegalActions` is
/// not the only enumeration the engine publishes: `ChooseTargets` carries the
/// legal targets, `ChooseCards` the legal cards, `ChooseColor` the legal
/// colours. Each is a promise of the same kind, and an answer lifted straight
/// out of one and then refused is the same two-probes disagreement.
///
/// The match is exhaustive on purpose. A new `Pending` variant is a new
/// question the engine can ask, and the choice of whether this sweep answers
/// it or counts it as unreached should be made when it is added, not
/// inherited from a `_` arm.
///
/// The two departures are `DiscardChoice` and `MulliganBottom`, which
/// enumerate a *count* and leave the cards to the seat's own hand — so the
/// hand is where those answers come from, exactly as a client reads them.
#[allow(clippy::too_many_lines)] // one flat arm per question the engine asks
fn drive_to_rest(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Rest {
    for _ in 0..200 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return Rest::Reached;
        }
        let (player, action) = match engine.pending().clone() {
            Pending::GameOver(_) => return Rest::Over,
            Pending::Mulligan { .. } => return Rest::Unanswered("Mulligan"),
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseAttackers { player, .. } => {
                (player, PlayerAction::DeclareAttackers { attackers: vec![] })
            }
            Pending::ChooseBlockers { player, .. } => {
                (player, PlayerAction::DeclareBlockers { blockers: vec![] })
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                // An "up to" prompt (`min` 0) is answered with one anyway:
                // choosing nothing is legal and exercises nothing.
                let want = usize::from(min).max(1).min(usize::from(max));
                let objects: Vec<_> = options.into_iter().take(want).collect();
                let players = player_options
                    .into_iter()
                    .take(want.saturating_sub(objects.len()))
                    .collect();
                (player, PlayerAction::ChooseTargets { objects, players })
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                let want = usize::from(min).max(1).min(usize::from(max));
                (
                    player,
                    PlayerAction::ChooseObjects {
                        objects: options.into_iter().take(want).collect(),
                    },
                )
            }
            Pending::LegendChoice { player, options } => (
                player,
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(1).collect(),
                },
            ),
            Pending::DiscardChoice { player, count }
            | Pending::MulliganBottom { player, count } => {
                let hand = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                (
                    player,
                    PlayerAction::ChooseObjects {
                        objects: hand.into_iter().take(usize::from(count)).collect(),
                    },
                )
            }
            Pending::ChooseSubtype { player, options } => {
                let Some(first) = options.first().copied() else {
                    return Rest::Unanswered("ChooseSubtype");
                };
                (player, PlayerAction::ChooseSubtype(first))
            }
            Pending::ChooseColor { player, options } => {
                let Some(first) = options.first().copied() else {
                    return Rest::Unanswered("ChooseColor");
                };
                (player, PlayerAction::ChooseColor(first))
            }
            Pending::ChoosePlayer { player, options } => {
                let Some(first) = options.first().copied() else {
                    return Rest::Unanswered("ChoosePlayer");
                };
                (player, PlayerAction::ChoosePlayer(first))
            }
            Pending::ChooseCastMode { player, options } => {
                let Some(first) = options.first() else {
                    return Rest::Unanswered("ChooseCastMode");
                };
                (player, PlayerAction::ChooseMode(first.index as usize))
            }
            Pending::ChooseNumber { player, min, .. } => (player, PlayerAction::ChooseNumber(min)),
            Pending::YesNo { player, .. } => (player, PlayerAction::YesNo(true)),
            Pending::OrderObjects { player, objects } => {
                (player, PlayerAction::OrderObjects { objects })
            }
        };
        if let Err(err) = engine.apply(player, action.clone()) {
            return Rest::Refused(format!(
                "{action:?} came out of the question, then: {err:?}"
            ));
        }
    }
    Rest::Stalled
}

/// What one chunk of the pool managed, so the sweep can say how far it got
/// rather than only whether it found anything.
#[derive(Default)]
struct Tally {
    /// Cards with at least one deed to press.
    cards: usize,
    /// Deeds pressed.
    deeds: usize,
    /// Deeds driven all the way back to a quiet priority.
    rested: usize,
    /// Questions [`drive_to_rest`] has no arm for, by name rather than by
    /// count: a report that says *which* one stopped it is the difference
    /// between a taxonomy finding and a number nobody can act on.
    unanswered: Vec<&'static str>,
    /// Deeds still asking questions when the step cap ran out.
    stalled: usize,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.cards += other.cards;
        self.deeds += other.deeds;
        self.rested += other.rested;
        self.unanswered.extend(other.unanswered.iter().copied());
        self.stalled += other.stalled;
    }
}

/// Presses every button the engine draws on one card, drives each press to a
/// rest, and names what the engine took back.
///
/// A board per deed because the first activation moves the game on: a cost is
/// paid, a permanent taps, the stack fills. Asking the second question of that
/// state would be asking a different question.
///
/// A panic inside the rules is named here rather than taking the whole sweep
/// down, for `probe_chunk`'s reason: a run should say *every* card it found,
/// not stop at the first.
fn refusals(def: &'static baylee_cards_dsl::CardDef) -> (Vec<String>, Tally) {
    let seat = PlayerId::new(0);
    let mut tally = Tally::default();
    let Some((_, objects, legal)) = probe(def.index) else {
        return (Vec::new(), tally);
    };
    let wanted = deeds(&legal, &objects);
    if wanted.is_empty() {
        return (Vec::new(), tally);
    }
    tally.cards = 1;
    tally.deeds = wanted.len();
    let mut offenders = Vec::new();
    for (slot, deed) in &wanted {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (mut engine, objects, _) = probe(def.index)?;
            if let Err(err) = engine.apply(seat, deed.action(objects[*slot])) {
                return Some(Rest::Refused(format!(
                    "{deed:?} was offered, then: {err:?}"
                )));
            }
            Some(drive_to_rest(&mut engine, seat))
        }));
        match outcome {
            Err(_) => offenders.push(format!("{} — {deed:?} panicked the engine", def.name())),
            Ok(None) => {}
            Ok(Some(Rest::Refused(what))) => offenders.push(format!("{} — {what}", def.name())),
            Ok(Some(Rest::Reached | Rest::Over)) => tally.rested += 1,
            Ok(Some(Rest::Unanswered(what))) => tally.unanswered.push(what),
            Ok(Some(Rest::Stalled)) => tally.stalled += 1,
        }
    }
    (offenders, tally)
}

/// The sweep, cut into one chunk per core for the reason the gamehost soak
/// gives: a board is built and walked once per deed and there are hundreds of
/// cards, and a test that is cheap is a test that keeps running on every
/// commit. The chunks share nothing — the registry is a static and every board
/// is built from the same constant seed — so what a chunk finds does not
/// depend on how the pool was divided.
fn sweep() -> (Vec<String>, Tally) {
    let cards: Vec<&'static baylee_cards_dsl::CardDef> =
        baylee_cards::all().filter(|d| d.is_implemented()).collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    let mut offenders = Vec::new();
                    let mut tally = Tally::default();
                    for def in slice {
                        let (found, one) = refusals(def);
                        offenders.extend(found);
                        tally.absorb(&one);
                    }
                    (offenders, tally)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("probe chunk"))
            .fold(
                (Vec::new(), Tally::default()),
                |(mut all, mut total), (found, one)| {
                    all.extend(found);
                    total.absorb(&one);
                    (all, total)
                },
            )
    })
}

#[test]
fn every_offered_ability_can_be_activated() {
    let (offenders, tally) = sweep();
    assert!(
        offenders.is_empty(),
        "the engine offered {} thing(s) and then took them back:\n  {}\n\
         (of {} deeds on {} cards: {} rested, {} unanswered, {} stalled)",
        offenders.len(),
        offenders.join("\n  "),
        tally.deeds,
        tally.cards,
        tally.rested,
        tally.unanswered.len(),
        tally.stalled,
    );
    assert!(
        tally.cards >= COVERAGE_FLOOR,
        "only {} implemented cards had any ability offered at all (floor \
         {COVERAGE_FLOOR}) — the sweep is no longer reaching the pool",
        tally.cards,
    );
    assert!(
        tally.rested >= RESTED_FLOOR,
        "only {} of {} deeds resolved all the way back to a quiet priority \
         (floor {RESTED_FLOOR}): {} were still asking when the step cap ran \
         out, and these questions have no arm in the driver: {:?}",
        tally.rested,
        tally.deeds,
        tally.stalled,
        {
            let mut kinds = tally.unanswered.clone();
            kinds.sort_unstable();
            kinds.dedup();
            kinds
        },
    );
}

/// The one live disagreement of this shape that the sweep's board cannot
/// reach, given the board it needs.
///
/// Recurring Nightmare's only ability is `Sacrifice a creature, Return this
/// enchantment to its owner's hand: Return target creature card from your
/// graveyard to the battlefield`, and [`probe`] stands a card up beside
/// twenty basics on a board with no creature and an empty graveyard. Both
/// halves are missing there, and `legal_actions` asks about the *target*
/// before it asks about the cost, so the sweep finds a card with nothing to
/// press and says nothing about it.
///
/// Given both halves the offer went out and `pay_cost` then refused it:
/// [`CostPart::Sacrifice`] and [`CostPart::Discard`] are choice costs no
/// activation can pay yet, and `can_afford` passed them in a silent no-op
/// arm. So the engine offered a card's only ability and took it back — on a
/// card the deckbuilder was listing as playable.
///
/// [`CostPart::Sacrifice`]: baylee_cards_dsl::CostPart::Sacrifice
/// [`CostPart::Discard`]: baylee_cards_dsl::CostPart::Discard
#[test]
fn an_ability_the_engine_cannot_pay_for_is_never_offered() {
    let seat = PlayerId::new(0);
    let nightmare = card_index("a6708b11-1bcd-4208-a967-fe91f2e3313c");
    let elves = card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[nightmare, elves, elves])
        .start();
    // One of the two Elves into the graveyard: the ability wants a creature
    // to sacrifice *and* a creature card to bring back, and without the
    // second the offer is refused for want of a target long before anything
    // asks whether the cost is payable.
    let doomed = on_battlefield(&engine, seat, elves).expect("an Elf on the battlefield");
    sba::destroy(
        engine
            .dev_state_mut(seat)
            .expect("the test kit grants dev commands"),
        doomed,
    );
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(seat))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == elves))),
        "the probe needs a creature card in the graveyard to be worth anything"
    );
    assert!(
        on_battlefield(&engine, seat, elves).is_some(),
        "the probe needs a creature left to sacrifice"
    );
    let source = on_battlefield(&engine, seat, nightmare).expect("Recurring Nightmare in play");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == source)
        .map(|(_, index)| *index)
        .collect();
    assert!(
        offered.is_empty(),
        "the engine offered Recurring Nightmare's ability {offered:?}, whose \
         cost `pay_cost` cannot pay"
    );
    // The other probe, asked the same question: refused at the door, with
    // nothing paid on the way to the refusal.
    let refused = engine.apply(
        seat,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    );
    assert!(
        matches!(
            refused,
            Err(EngineError::IllegalAction("ability not activatable"))
        ),
        "the activation should agree with the offer, and answered {refused:?}"
    );
}

/// The class that card is one member of, guarded where the sweep cannot look.
///
/// An ability whose cost `can_afford` refuses unconditionally is never
/// offered, so [`COVERAGE_FLOOR`] counts the card exactly as it counts a
/// vanilla creature: it arrives, is asked, and answers with nothing. The
/// sweep cannot *report* the next Recurring Nightmare — it can only fail to
/// mention it — while the deckbuilder goes on listing the card as playable
/// and it sits in a deck doing nothing at all.
///
/// So this half is static and pool-wide, in the shape of
/// `keyword_tests::no_card_claims_a_keyword_the_engine_ignores`: a card that
/// prints such a cost says `Coverage::Partial` with the reason on it, or the
/// build fails. Recurring Nightmare was moved by hand; this is what stops
/// the next one arriving as playable.
///
/// Activated abilities only, and both spellings of one
/// ([`AbilityDef::ActivatedConditional`] is the same ability with a
/// precondition, and reading only the first is how six other places got this
/// wrong). A spell's alternative or additional cost is a different path
/// entirely — it is paid in the casting wizard, which does have somewhere to
/// ask — and neither `can_afford` nor `pay_cost` is ever shown one.
#[test]
fn no_implemented_card_hides_an_ability_the_engine_will_never_offer() {
    let mut offenders = Vec::new();
    for def in baylee_cards::all().filter(|d| d.is_implemented()) {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                let (AbilityDef::Activated { cost, .. }
                | AbilityDef::ActivatedConditional { cost, .. }) = ability
                else {
                    continue;
                };
                for part in cost.parts {
                    if crate::engine::abilities::choice_cost_unpayable(part) {
                        offenders.push(format!("{} — {part:?}", def.name()));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "an implemented card carries an activated ability the engine will \
         never offer, because `can_afford` refuses its cost on every board: \
         {offenders:?}"
    );
}

/// The tokens carrying such an ability, and why each is not a live defect.
///
/// A name and a reason, the way `keyword_tests::ENFORCED` is a keyword and
/// the rule that reads it: a list nobody can grow without writing down what
/// they are excusing. Both halves are checked below, so an entry that stops
/// being true fails as loudly as a token that stops being listed.
const INERT_TOKENS: &[(&str, &str)] = &[(
    "Blood",
    "`{1}, {T}, Discard a card, Sacrifice this artifact: Draw a card` — the \
     discard is a choice an activation has nowhere to make, and no card in \
     the pool creates a Blood token, so nothing is ever offered it",
)];

/// [`no_implemented_card_hides_an_ability_the_engine_will_never_offer`], for
/// the permanents no card prints.
///
/// A token is a permanent `legal_actions` asks `can_afford` about like any
/// other, and `tokens::ALL` sits *beside* `cards/` rather than inside it,
/// which is how the first pass at this class walked straight past Blood. The
/// day a card creates one, its only ability is never offered and the player
/// is handed an artifact that does nothing at all — the same defect as the
/// card half, arriving through a door the card half cannot see.
#[test]
fn no_token_carries_an_ability_the_engine_will_never_offer() {
    let mut offenders = Vec::new();
    let mut still_inert = Vec::new();
    for token in baylee_cards::tokens::ALL {
        let excused = INERT_TOKENS.iter().any(|(name, _)| *name == token.name);
        for ability in token.abilities {
            let (AbilityDef::Activated { cost, .. }
            | AbilityDef::ActivatedConditional { cost, .. }) = ability
            else {
                continue;
            };
            for part in cost.parts {
                if !crate::engine::abilities::choice_cost_unpayable(part) {
                    continue;
                }
                if excused {
                    still_inert.push(token.name);
                } else {
                    offenders.push(format!("{} — {part:?}", token.name));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a token carries an activated ability the engine will never offer, \
         because `can_afford` refuses its cost on every board: {offenders:?}"
    );
    for (name, why) in INERT_TOKENS {
        assert!(
            still_inert.contains(name),
            "{name} is excused here for a cost it no longer has — {why}; take \
             the entry out of INERT_TOKENS"
        );
    }
}

/// The twin of that pair, and the one no test of an action could fail on.
///
/// [`no_implemented_card_hides_an_ability_the_engine_will_never_offer`] and
/// its token half are about a cost the engine *refuses*. This is about a cost
/// the engine forgets: `ExileFromHand` and `PayLifeX` are paid in the casting
/// wizard, so `can_afford` accepts them and `pay_cost` walks past them —
/// right for Force of Will's pitch and Toxic Deluge's X, and on an activated
/// ability a pitch cost that exiles nothing. The activation succeeds, which
/// is why nothing else can catch it: the only evidence would be the card
/// still in a hand that was supposed to have paid it.
///
/// Both doors in one test, because neither has anything to excuse. No card
/// and no token prints such a cost today, so there is no [`INERT_TOKENS`]
/// half to keep honest — the message says which one arrived and that is
/// enough.
///
/// Every card, and deliberately not only the implemented ones — which is the
/// one place this parts company with the pair above. `Coverage::Partial` is a
/// real answer there: a refused cost is never offered, so the ability is
/// inert, which is exactly what `Partial` promises about the clause it names.
/// It is no answer at all here. A partial card *plays* — the deckbuilder
/// offers it, marked — so a `Partial` card carrying `ExileFromHand` on an
/// activation ships a pitch cost that exiles nothing into somebody's deck,
/// and the label would be documenting the cheat rather than preventing it.
/// The ability comes off the card with a `// NOT SUPPORTED:` line instead.
#[test]
fn nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip() {
    let mut offenders = Vec::new();
    let mut check = |who: &str, ability: &AbilityDef| {
        let (AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. }) =
            ability
        else {
            return;
        };
        for part in cost.parts {
            if crate::engine::abilities::paid_by_the_casting_wizard(part) {
                offenders.push(format!("{who} — {part:?}"));
            }
        }
    };
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in baylee_cards::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    assert!(
        offenders.is_empty(),
        "an activated ability carries a cost part `pay_cost` skips, so the \
         engine offers the ability and then never charges for it — and \
         `Coverage::Partial` does not excuse it, because a partial card is \
         dealt into a deck and played: {offenders:?}"
    );
}

/// The same class once more, on the side of the line where the parts above
/// *are* paid — because "the casting wizard pays it" turned out to be a claim
/// that has to name **which** of a spell's three cost lists.
///
/// They are paid in three different places and none of them by the same code:
///
/// - `alternative_costs[i].cost.parts` pays `PayLife` and `ExileFromHand` —
///   Force of Will's pitch. It is the only one with a gate: `cast_options`
///   runs `can_afford` over it, so a `Sacrifice(_)` written here is *refused*.
///   That is not safety. It is a refusal at the mode, and it took a second
///   fix for it to be a refusal at all: `casting::can_cast` probed the
///   alternative's *mana* and nothing else, so a pitch with nothing to pitch
///   was listed as castable and the wizard then had no option to offer — a
///   dead offer, the shape the `CommanderControlled` bug had. Both askers now
///   read `casting::pitchable`, and a part neither of them understands is
///   still one this list must never carry.
/// - `mandatory_additional_costs` pays `PayLifeX` and `PayLife` — Toxic
///   Deluge's X. Nothing gates it at all, so anything else there is not
///   refused but skipped, and the spell is cast without paying it.
/// - `additional_costs[i].parts` is paid by **nothing**: `finish_cast`
///   combines `add.mana` and never looks at the parts. The field exists, a
///   card can fill it, and a kicker taken with it costs exactly its mana.
///
/// So the rule is per list, and each half of it is held by the predicate the
/// payment itself reads ([`cast_wizard::paid_as_an_alternative_cost`],
/// [`cast_wizard::paid_as_a_mandatory_additional_cost`]) — the arrangement
/// that stopped the activated-ability twin from drifting.
///
/// Every card, not the implemented ones only, for `93ed903`'s reason: a
/// partial card is dealt into decks and played, and a spell cast for free is
/// not something `Coverage::Partial` should be able to hide. No token half —
/// `TokenDef` carries none of these three lists, having no cost to cast — so
/// the door beside the pool is checked here and found not to exist.
///
/// [`cast_wizard::paid_as_an_alternative_cost`]: super::cast_wizard
/// [`cast_wizard::paid_as_a_mandatory_additional_cost`]: super::cast_wizard
#[test]
fn no_spell_cost_list_carries_a_part_its_payment_walks_past() {
    let mut offenders = Vec::new();
    for def in baylee_cards::all() {
        for face in def.faces {
            for alt in face.alternative_costs {
                for part in alt.cost.parts {
                    if !crate::engine::cast_wizard::paid_as_an_alternative_cost(part) {
                        offenders.push(format!(
                            "{} — alternative cost {part:?}, which `finish_cast` does not pay",
                            face.name,
                        ));
                    }
                }
            }
            for part in face.mandatory_additional_costs {
                if !crate::engine::cast_wizard::paid_as_a_mandatory_additional_cost(part) {
                    offenders.push(format!(
                        "{} — mandatory additional cost {part:?}, which `finish_cast` skips",
                        face.name,
                    ));
                }
            }
            for add in face.additional_costs {
                for part in add.parts {
                    offenders.push(format!(
                        "{} — kicker part {part:?}, and a kicker's parts are read by nothing",
                        face.name,
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a spell prints a cost part on a list whose payment never reads it, so \
         the spell is cast without paying it (or, on an alternative cost the \
         gate refuses, is offered and then uncastable): {offenders:?}"
    );
}

/// A permanent that makes mana two ways is offered once.
///
/// `mana_abilities` is addressed by `PlayerAction::ActivateManaAbility
/// { source }`, which names no index, so the list is a list of *permanents*:
/// a second entry for one of them is an offer nothing can accept. The first
/// press takes whichever ability the engine prefers, the permanent is
/// tapped, and the duplicate is then refused by the very list that put it
/// there — this module's whole subject, one line lower down than usual.
///
/// Chromatic Lantern is where the pool says it. It grants every land
/// "{T}: Add one mana of any color" on top of the CR 305.6 shortcut a basic
/// already has, and entered each of them in the list twice.
///
/// Every reader had to know that on its own. `testkit::tap_mana_except`
/// unwraps and would have panicked; [`probe`] gave up on the board, which is
/// why Chromatic Lantern and Great Divide Guide were the last two implemented
/// cards the sweep could not reach; and the client survived only because
/// `manasources::sources` dedupes by object id for a reason of its own.
#[test]
fn a_permanent_that_makes_mana_two_ways_is_offered_once() {
    let seat = PlayerId::new(0);
    let lantern = card_index("539f5396-d99a-417d-a84c-dff7930b5900");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[lantern, forest, forest])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.mana_abilities.clone();
    let mut once = offered.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(
        once.len(),
        offered.len(),
        "a permanent was offered as a mana source more than once: {offered:?}"
    );
    assert_eq!(offered.len(), 2, "two Forests under a Lantern: {offered:?}");
    // The other half of the same claim: every entry in the list is one the
    // action will take. Pressed in the order the engine listed them, which
    // is how `HeuristicAgent` and the client's planner both read it.
    for source in offered {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap_or_else(|err| panic!("{source:?} was offered, then: {err:?}"));
    }
}

/// Suspend is offered on the same two conditions as any other activation.
///
/// Its first ability is an activated one — "rather than cast this card from
/// your hand, **pay {U}** and exile it" — with "activate only as a sorcery"
/// on it (CR 702.62a). The offer used to ask only about the turn, so a
/// suspend card sat lit in an opponent's hand-zone list with an empty pool,
/// and `apply` then answered `cannot pay the suspend cost`. That is the
/// shape this whole module exists for, and it was the one branch in the
/// hand-zone scan that did not go through `can_afford`.
///
/// The pool and not the untapped Island is deliberately what decides it:
/// every activation in this engine is offered against mana that is already
/// floating, which is why the client plans a run of taps first.
#[test]
fn a_suspend_card_is_offered_only_once_its_cost_is_on_the_table() {
    let seat = PlayerId::new(0);
    let vision = card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d");
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = Duel::new(SEED, island)
        .hand(0, &[vision])
        .battlefield(0, &[island])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.suspendable.is_empty(),
        "Suspend 4—{{U}} was offered with nothing in the pool: {:?}",
        legal.suspendable
    );
    // And it is not castable either, at any point in this test. Ancestral
    // Vision prints *no* mana cost, which CR 202.1a says is not a cost of
    // zero: the only way it leaves this hand is the suspend ability. It was
    // in `castable` here — a free "target player draws three cards" for
    // anybody who reached their own main phase.
    assert!(
        legal.castable.is_empty(),
        "a card with no mana cost was castable for nothing: {:?}",
        legal.castable
    );

    // The Island is untapped and the timing is right, so the only thing
    // between the two assertions is the {U} itself.
    let source = legal.mana_abilities[0];
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("an untapped Island taps for {U}");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.suspendable.len(),
        1,
        "with {{U}} floating the card is suspendable: {:?}",
        legal.suspendable
    );
    assert!(
        legal.castable.is_empty(),
        "a blank cost is not paid by floating a mana either: {:?}",
        legal.castable
    );

    // The other half of this module's claim: what is offered is accepted.
    engine
        .apply(
            seat,
            PlayerAction::Suspend {
                card: legal.suspendable[0],
            },
        )
        .expect("offered, then refused");
}

/// A cost of `{0}` is still a cost, and still buys a spell.
///
/// The other side of `casting::has_a_printed_cost`, and the reason it is a
/// predicate rather than a comparison against `ManaCost::ZERO`: Mox Opal
/// prints `{0}` and is cast by paying nothing, which is an entirely ordinary
/// thing for a spell to do. A guard that could not tell it from a blank would
/// have made four cards in this pool uncastable instead of two.
#[test]
fn a_printed_zero_is_a_cost_and_the_spell_is_still_castable() {
    let seat = PlayerId::new(0);
    let opal = card_index("de2440de-e948-4811-903c-0bbe376ff64d");
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = Duel::new(SEED, island).hand(0, &[opal]).start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.castable.len(),
        1,
        "Mox Opal costs {{0}} and an empty pool pays it: {:?}",
        legal.castable
    );
    engine
        .apply(
            seat,
            PlayerAction::CastSpell {
                card: legal.castable[0],
            },
        )
        .expect("offered, then refused");
}

/// Only a card with another way out of the hand prints no mana cost.
///
/// The pool-wide half of CR 202.1a, and the guard on the data rather than on
/// the rule: `face!`'s default cost is the *blank* one, so a hand-written
/// card that prints `{0}` and simply omits the field used to be
/// indistinguishable from Ancestral Vision — and is now uncastable instead of
/// free, which is the quieter of the two failures and the reason this is
/// asserted rather than commented. Mox Opal and Pact of Negation were both
/// sitting here when it was written.
///
/// What is allowed to be blank is a card that names another way to be cast:
/// suspend (`AbilityDef::Suspend`) or a printed alternative cost. Lands and
/// the backs of transforming cards are not cast from a hand at all.
#[test]
fn a_face_with_no_mana_cost_has_another_way_out_of_the_hand() {
    let mut blank: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let suspends = def
            .abilities
            .iter()
            .any(|a| matches!(a, AbilityDef::Suspend { .. }));
        for face in def.faces {
            if face.mana_cost.symbols().next().is_some()
                || !face.castable_from_hand
                || face.types.contains(TypeSet::LAND)
            {
                continue;
            }
            if !suspends && face.alternative_costs.is_empty() {
                blank.push(face.name);
            }
        }
    }
    assert!(
        blank.is_empty(),
        "these faces print no mana cost and no other way to be cast, so they \
         can never leave a hand — a `{{0}}` card that omitted `mana_cost` \
         looks exactly like this: {blank:?}"
    );
}

/// The suspend offer and the suspend payment agree under Mycosynth Lattice.
///
/// Suspend's cost is a bare `ManaCost` rather than a `Cost`, so it reaches
/// the pool through `can_pay_mana` on the offer side and `casting::pay_with`
/// on the apply side. Both branch on `casting::mana_is_wild`, and that is
/// exactly the split that was there before: the offer probed
/// `can_pay_wild` while the payment called `mana_pay::pay`, so a Lattice on
/// the table made the two halves disagree about which mana counts.
///
/// Nothing else in this module reaches it — the sweep's probe board has no
/// Lattice on it, and a rule that is only ever exercised on one side of a
/// branch is a rule with no test at all.
#[test]
fn wild_mana_pays_a_suspend_cost_the_offer_also_accepts() {
    let seat = PlayerId::new(0);
    let vision = card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d");
    let lattice = card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[vision])
        .battlefield(0, &[lattice, forest])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );

    // A Forest and a {U} cost: without the Lattice this is unpayable, which
    // is what makes the tap the whole of the test.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.suspendable.is_empty(),
        "Suspend 4—{{U}} was offered with nothing in the pool: {:?}",
        legal.suspendable
    );
    // The Lattice makes every permanent an artifact and none of them a mana
    // source, so the Forest is the only entry there is.
    assert_eq!(
        legal.mana_abilities.len(),
        1,
        "the Forest is the board's only mana source: {:?}",
        legal.mana_abilities
    );
    let source = legal.mana_abilities[0];
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("an untapped Forest taps for {G}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.suspendable.len(),
        1,
        "under a Lattice the floating {{G}} pays {{U}}: {:?}",
        legal.suspendable
    );
    // The half that was broken: offered, then refused by the payment.
    engine
        .apply(
            seat,
            PlayerAction::Suspend {
                card: legal.suspendable[0],
            },
        )
        .expect("what the engine offers, the engine accepts");
}
