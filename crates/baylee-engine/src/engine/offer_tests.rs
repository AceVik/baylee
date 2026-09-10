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
/// The number is 446 measured on 2026-09-10, less a tenth, so ordinary pool
/// growth never touches it and a setup change that stops part of the pool
/// from arriving at its main phase fails here instead of silently.
const COVERAGE_FLOOR: usize = 400;

/// The floor under the deeds driven all the way back to a quiet priority.
///
/// Pressing a button and watching the engine refuse the very answer it just
/// enumerated is the point of the drive, and a drive that stops at the first
/// question it cannot answer checks nothing past the press. This is the
/// second half of [`COVERAGE_FLOOR`]: that one says the sweep still reaches
/// the pool, this one says it still gets through it. Every one of the 630
/// deeds rested when this was measured on 2026-09-10, with nothing stalled
/// and no question the driver could not answer; the floor is that less a
/// tenth.
const RESTED_FLOOR: usize = 560;

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
/// twenty basics, walked to seat 0's first main phase with every basic
/// tapped for mana.
///
/// The card's own permanent is deliberately left untapped
/// ([`tap_mana_except`]'s reason): one whose interesting ability costs `{T}`
/// would otherwise have spent itself paying for the mana that pays for it.
///
/// Returns the engine, the card's objects (battlefield first, then hand —
/// a fixed order, so a deed found on one board addresses the same object on
/// the next), and the offer standing at that moment. `None` when the board
/// never got there.
fn probe(card: CardIndex) -> Option<(Engine<RegistryLookup>, Vec<ObjectId>, LegalActions)> {
    let seat = PlayerId::new(0);
    let def = baylee_cards::by_index(card)?;
    let mut field = basics();
    if is_permanent(def) {
        field.insert(0, card);
    }
    let filler = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .next()
        .unwrap_or(card);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &field)
        .hand(0, &[card])
        .start();
    if !walk_to_own_main(&mut engine, seat) {
        return None;
    }
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
        if Some(source) == permanent {
            continue;
        }
        // A mana ability the engine offered and then refused is the same
        // contradiction this module is about, so the board is abandoned
        // rather than unwrapped through: these are the basics, and a basic
        // that cannot tap for mana is a failure the mana tests own.
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
