//! The three effect-level `if`s that ask about the turn or the event:
//! `IfCreaturesDiedAtLeast`, `IfNotLostLifeThisTurn` and
//! `IfEventPowerAtLeast`.
//!
//! Each is a condition the resolution asks as it reaches it, not an
//! intervening `if` (CR 603.4, which `condition_tests` plays): the ability
//! is on the stack either way, and the branch decides what it does. So each
//! test reads the effect's result, on both sides of the threshold, and on
//! the threshold itself — the one place a `>=` and a `>` disagree.

use super::*;
use baylee_cards_dsl::{Amount, CostPart, CounterKind, Filter, Trigger};

const TALLY: u32 = 7120;
const MOTE: u32 = 7121;
const ASCENT: u32 = 7122;
const TOLL: u32 = 7123;
const WORLD_TREE: u32 = 7124;
const OAK: u32 = 7125;
const SEEDLING: u32 = 7126;

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];

/// "{0}: If two or more creatures died this turn, you gain 1 life."
static TALLY_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::IfCreaturesDiedAtLeast {
        n: 2,
        then: GAIN_ONE,
    }],
    None,
)];

/// "Sacrifice this creature: nothing" — a way to make a creature die that
/// costs no mana and asks no question.
static MOTE_ABILITIES: &[AbilityDef] = &[paid(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::SacrificeSelf],
    },
    &[],
    None,
)];

/// "{0}: If you haven't lost life this turn, you gain 1 life."
static ASCENT_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::IfNotLostLifeThisTurn { then: GAIN_ONE }],
    None,
)];

/// "Pay 1 life: nothing" — a life loss (CR 119.4) that is nothing else.
static TOLL_ABILITIES: &[AbilityDef] = &[paid(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::PayLife(1)],
    },
    &[],
    None,
)];

static YOUR_CREATURE: Filter = Filter::YOUR_CREATURE;

/// Tribute to the World Tree's sentence: "Whenever a creature you control
/// enters, draw a card if its power is 3 or greater. Otherwise, put two
/// +1/+1 counters on it."
static WORLD_TREE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::EntersBattlefield(&YOUR_CREATURE),
    &[Effect::IfEventPowerAtLeast {
        n: 3,
        then: &[Effect::draw(1)],
        otherwise: &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
    }]
)];

fn cards() -> Vec<&'static CardDef> {
    let land = super::super::synthetic::land;
    vec![
        land(TALLY, "Tally", TALLY_ABILITIES),
        card(
            MOTE,
            creature_face("Mote", "{1}", 1, 1),
            KeywordSet::EMPTY,
            MOTE_ABILITIES,
        ),
        land(ASCENT, "Ascent", ASCENT_ABILITIES),
        land(TOLL, "Toll", TOLL_ABILITIES),
        land(WORLD_TREE, "World Tree", WORLD_TREE_ABILITIES),
        card(
            OAK,
            creature_face("Oak", "{0}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SEEDLING,
            creature_face("Seedling", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

/// Activates `source`'s only ability and lets it resolve.
fn run(engine: &mut Bench, seat: PlayerId, source: ObjectId) {
    activate(engine, seat, source, 0);
    let active = engine.state().turn.active;
    settle(engine, active);
}

// ------------------------------------------------- creatures died this turn

/// Nothing below the threshold, the effect at it, and the count is this
/// turn's: on the next turn nobody has died yet.
#[test]
fn a_death_count_branch_runs_from_its_threshold_on_and_only_this_turn() {
    let mut engine = bench(
        7120,
        cards(),
        [Seat::with(&[TALLY, MOTE, MOTE]), Seat::default()],
    );
    to_main(&mut engine, me());
    let tally = the(&engine, ZoneLocation::Battlefield, TALLY);
    let motes = objects(&engine, ZoneLocation::Battlefield, MOTE);
    let start = life(&engine, me());

    run(&mut engine, me(), tally);
    assert_eq!(life(&engine, me()), start, "no creature has died");

    run(&mut engine, me(), motes[0]);
    assert_eq!(engine.state().per_turn.creatures_died, 1);
    run(&mut engine, me(), tally);
    assert_eq!(life(&engine, me()), start, "one death is below two");

    run(&mut engine, me(), motes[1]);
    assert_eq!(engine.state().per_turn.creatures_died, 2);
    run(&mut engine, me(), tally);
    assert_eq!(
        life(&engine, me()),
        start + 1,
        "two deaths are \"two or more\""
    );

    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Upkeep)
    });
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    run(&mut engine, me(), tally);
    assert_eq!(
        life(&engine, me()),
        start + 1,
        "the deaths were last turn's, and this is another turn"
    );
}

// --------------------------------------------------- life lost this turn

/// The branch runs until its controller loses life, gaining life is not
/// losing it, and a life payment is a loss (CR 119.4: "the player loses
/// that much life").
#[test]
fn a_life_loss_this_turn_closes_the_branch_and_a_gain_does_not() {
    let mut engine = bench(
        7121,
        cards(),
        [Seat::with(&[ASCENT, TOLL]), Seat::default()],
    );
    to_main(&mut engine, me());
    let ascent = the(&engine, ZoneLocation::Battlefield, ASCENT);
    let toll = the(&engine, ZoneLocation::Battlefield, TOLL);
    let start = life(&engine, me());

    run(&mut engine, me(), ascent);
    run(&mut engine, me(), ascent);
    assert_eq!(
        life(&engine, me()),
        start + 2,
        "no life lost, twice — and the first gain was not a loss"
    );

    run(&mut engine, me(), toll);
    assert_eq!(life(&engine, me()), start + 1, "one life paid");
    run(&mut engine, me(), ascent);
    assert_eq!(
        life(&engine, me()),
        start + 1,
        "life was lost this turn, so the branch does nothing"
    );
}

/// "You" is the ability's controller: an opponent's loss is not theirs.
#[test]
fn another_players_life_loss_leaves_the_branch_open() {
    let mut engine = bench(7122, cards(), [Seat::with(&[ASCENT]), Seat::with(&[TOLL])]);
    to_main(&mut engine, me());
    let ascent = the(&engine, ZoneLocation::Battlefield, ASCENT);
    let toll = the(&engine, ZoneLocation::Battlefield, TOLL);
    let (mine, theirs) = (life(&engine, me()), life(&engine, them()));

    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    run(&mut engine, them(), toll);
    assert_eq!(life(&engine, them()), theirs - 1, "the opponent paid");

    run(&mut engine, me(), ascent);
    assert_eq!(
        life(&engine, me()),
        mine + 1,
        "and it was not my life that was lost"
    );
}

// ------------------------------------------------------ the event's power

/// The creature that entered is what the branch measures, and what its
/// `otherwise` puts counters on: the entering creature, not the permanent
/// whose ability it is.
#[test]
fn an_event_power_branch_reads_the_entering_creature_on_both_sides() {
    let mut engine = bench(
        7123,
        cards(),
        [
            Seat::with(&[WORLD_TREE]).holding(&[OAK, SEEDLING]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let tree = the(&engine, ZoneLocation::Battlefield, WORLD_TREE);
    let plus = |e: &Bench, id: ObjectId| {
        e.state()
            .object(id)
            .map_or(0, |o| o.counters.get(CounterKind::P1P1))
    };

    let oak = the(&engine, ZoneLocation::Hand(me()), OAK);
    engine
        .apply(me(), PlayerAction::CastSpell { card: oak })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(triggered(&engine, tree), 1, "the Oak's arrival triggered");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        2,
        "power 3 is \"3 or greater\": a card drawn beside the Seedling"
    );
    assert_eq!(plus(&engine, oak), 0, "and no counters on the Oak");

    let seedling = the(&engine, ZoneLocation::Hand(me()), SEEDLING);
    engine
        .apply(me(), PlayerAction::CastSpell { card: seedling })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(triggered(&engine, tree), 2, "the Seedling's arrival too");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        1,
        "power 2 draws nothing"
    );
    assert_eq!(
        plus(&engine, seedling),
        2,
        "\"otherwise, put two +1/+1 counters on it\": on the Seedling"
    );
    assert_eq!(plus(&engine, tree), 0, "and not on the Tree");
}
