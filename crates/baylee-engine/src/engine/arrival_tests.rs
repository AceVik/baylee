//! When a permanent arrived, and the two rules that read it.
//!
//! [`Filter::EnteredThisTurn`] is the first filter in the vocabulary that is
//! **history** rather than a characteristic: nothing on a `GameObject` says
//! it. It is answered from `per_turn.entered_battlefield`, the turn's record
//! of arrivals, beside `per_turn.life_lost` for "lost life this turn". Both
//! used to be scans of the journal from the turn's start, which
//! `snapshot_hash` does not read (#241).
//!
//! Asked of `eval` directly rather than through one of the nine cards that
//! print it. A card test would prove the filter *and* an ability *and* a
//! cost at once, and would be a test of a card somebody else's lane is
//! writing; what is here is the predicate on its own, with the three answers
//! that can differ:
//!
//! 1. a permanent seeded by `SeatSpec::starting_battlefield` → **no**, and
//!    not because `Cause::Setup` is filtered out. It is not: a seeded
//!    permanent is recorded like any other arrival while the game is built,
//!    and the answer is no because the first turn start, after the
//!    mulligans, clears the record. The exclusion was written first and
//!    taken out again when injecting its removal left this test green. Both
//!    halves are asserted below rather than assumed, because together they
//!    are the whole reason.
//! 2. a land actually played this turn → **yes**.
//! 3. the same land on the next turn → **no**. That is the clause the whole
//!    filter is named for, and a record cleared at the game's first turn
//!    and never again would pass 1 and 2 and fail only here.
//!
//! [`Condition::Any`] is beside it because the Gathering Place cycle needs
//! both at once: "activate only if this land entered this turn **or** if you
//! control a basic land". Its own three answers are one half true, the other
//! half true, and neither — the last being the one a combinator written as
//! `parts.iter().all(..)` still passes.

use super::testkit::{
    Duel, RegistryLookup, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_cards_dsl::{Condition, Filter};
use baylee_core::ids::{CardIndex, ObjectId};

/// `Condition::Any` takes a `&'static [Condition]`, so the parts are items
/// rather than locals — the same shape a card file writes them in.
static ENTERED: Condition = Condition::SourceMatches(&Filter::EnteredThisTurn);
static BASIC: Condition = Condition::ControlCount(&Filter::BASIC_LAND, 1);
static MANY: Condition = Condition::ControlCount(&Filter::BASIC_LAND, 9);
static EITHER: [Condition; 2] = [ENTERED, BASIC];
static REVERSED: [Condition; 2] = [BASIC, ENTERED];
static NEITHER: [Condition; 2] = [ENTERED, MANY];
static EMPTY: [Condition; 0] = [];

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// Whether the engine says this object entered the battlefield this turn.
fn arrived(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    let obj = engine.state().object(id).expect("the object is in play");
    crate::eval::matches(
        &Filter::EnteredThisTurn,
        engine.state(),
        obj,
        obj.controller,
        id,
    )
}

/// All three answers, in one game, because the third is only reachable from
/// the second.
#[test]
fn a_permanent_entered_this_turn_only_on_the_turn_it_was_played() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4_401, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[forest()])
        .start();

    // *Why* the seeded Forest will answer no, which the answer cannot say
    // on its own: it is recorded like any other arrival while the game is
    // built, and the first turn start clears the record. Asserted rather
    // than trusted, because the whole filter rests on it and nothing else
    // in the engine would notice a setup path that ran after that clearing.
    let seeded = on_battlefield(&engine, p0, forest()).expect("the bench seated one Forest");
    assert!(
        engine
            .state()
            .per_turn
            .entered_battlefield
            .contains(&seeded),
        "no `Cause` is filtered out: the seating is recorded as an arrival"
    );
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        !arrived(&engine, seeded),
        "a permanent the preset seated did not enter this turn"
    );

    // A real `PlayLand` and not a second seeding, which is the whole point.
    let card = in_hand(&engine, p0, forest()).expect("a Forest is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop in a first main phase on an empty board");
    let played = card;
    assert_ne!(played, seeded, "two different Forests, to tell them apart");
    assert!(
        arrived(&engine, played),
        "a real `PlayLand` is the entry this filter is about"
    );
    assert!(
        !arrived(&engine, seeded),
        "and the one that was already there is still not"
    );

    // On to the next turn: the land is the same object and the answer has
    // to change, which is what clearing the record at every turn start is
    // for. The next seat's hand is taken first, so the card its draw step
    // brings can be told apart below.
    let p1 = PlayerId::new(1);
    let held = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active != p0
    });
    assert_eq!(
        engine.state().turn.active,
        p1,
        "two seats, so the next is p1"
    );
    assert!(
        !arrived(&engine, played),
        "a new turn began, so nothing entered *this* one"
    );
    assert!(stack_is_empty(&engine), "and nothing is waiting to resolve");

    // The destination half, which no permanent can fail: to *be* on the
    // battlefield, an object's last move was to the battlefield, so a record
    // written on every move, and not only on arrivals, would agree with this
    // one everywhere above. A hand is where the two come apart — the draw
    // step moved a card this turn, and "entered the battlefield this turn"
    // has to be false of it.
    let moved_this_turn: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p1))
        .iter()
        .copied()
        .filter(|id| !held.contains(id))
        .collect();
    assert!(
        !moved_this_turn.is_empty(),
        "the draw step moved a card into the active seat's hand this turn, \
         which is what makes the next assertion about something"
    );
    for id in moved_this_turn {
        assert!(
            !arrived(&engine, id),
            "a card drawn this turn changed zone this turn and did **not** \
             enter the battlefield: {id:?}"
        );
    }
}

/// `Condition::Any` holds when either part does, and not when neither does.
#[test]
fn any_condition_holds_when_one_of_its_parts_does() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4_402, forest())
        .battlefield(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    // A basic Forest is on the battlefield and it did not enter this turn,
    // so the two halves of the printed disjunction disagree — which is what
    // makes each of them readable off this one board.
    let holds = |c: Condition| crate::eval::condition_holds(engine.state(), p0, land, c);

    assert!(!holds(ENTERED), "the Forest was seated, not played");
    assert!(holds(BASIC), "and a basic land is what this seat controls");

    assert!(
        holds(Condition::Any(&EITHER)),
        "either part is enough — the false one stands first, so a combinator \
         that stopped at the first answer would say no"
    );
    assert!(
        holds(Condition::Any(&REVERSED)),
        "and the order does not matter"
    );
    assert!(
        !holds(Condition::Any(&NEITHER)),
        "neither part, so neither does the whole"
    );
    assert!(
        !holds(Condition::Any(&EMPTY)),
        "and an empty list is no reason at all — `any` over nothing is false, \
         which is the answer that keeps a mis-written card refused rather \
         than always on"
    );
}
