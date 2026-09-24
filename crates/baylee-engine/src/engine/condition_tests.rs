//! The intervening-`if` clause, asked twice (CR 603.4).
//!
//! "At the beginning of your upkeep, **if this land is tapped**, put a
//! storage counter on it" is one sentence with two checks in it: the
//! ability does not trigger at all while the clause is false, and an
//! ability that did trigger "is removed from the stack and does nothing"
//! if the clause has stopped being true by the time it would resolve. The
//! rule says this mirrors the check for legal targets, and the two halves
//! fail differently — a missing first check puts an ability on the stack
//! that a player may respond to, and a missing second one resolves an
//! ability whose reason for existing has gone.
//!
//! Synthetic lands rather than the five Fallen Empires storage lands,
//! because those need two more things this commit does not have (a printed
//! sentence `landgen` can read, and their untap-step clause) and a test
//! that waited for them would be proving three rules at once. The bench is
//! [`super::synthetic`].

use super::synthetic::{
    SyntheticLookup, forest, keep_mulligans, land, permanents, preset, tapped, walk_past,
};
use super::*;
use crate::turn::Step;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Amount, Condition, Cost, Effect,
    Filter, ManaColor, PlayerRel, StepKind, Trigger, counters,
};

// ---------------------------------------------------------------- fixtures

/// A land whose upkeep trigger asks to be **untapped** — the clause that
/// is true on an ordinary turn, so it can be made false by tapping the
/// land in response to its own trigger.
const WAKING_BASIN: u32 = 1110;
/// The storage lands' own clause, asking to be **tapped**: false on an
/// ordinary turn, so the ability must never trigger at all.
const BANKING_BASIN: u32 = 1111;
/// A clause about the *player* rather than about the source, which is the
/// only shape that can see which player the clause was asked of.
const COUNTING_BASIN: u32 = 1112;

static UNTAPPED_F: Filter = Filter::Untapped;
static TAPPED_F: Filter = Filter::Tapped;

const UPKEEP: Trigger = Trigger::StepBegin {
    step: StepKind::Upkeep,
    whose: PlayerRel::You,
};

static BANK_ONE: &[Effect] = &[Effect::AddCounter {
    kind: counters::STORAGE,
    amount: Amount::Fixed(1),
}];

static LAND_F: Filter = Filter::LAND;

static MANA: &[Effect] = &[Effect::mana(ManaColor::Colorless, 1)];

const TAP_FOR_MANA: AbilityDef = AbilityDef::Activated {
    cost: Cost::TAP,
    effects: MANA,
    targets: None,
    second_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: true,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
};

static WAKING_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Triggered {
        trigger: UPKEEP,
        effects: BANK_ONE,
        targets: None,
        once_per_turn: false,
        condition: Some(Condition::SourceMatches(&UNTAPPED_F)),
    },
    TAP_FOR_MANA,
];

static BANKING_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Triggered {
        trigger: UPKEEP,
        effects: BANK_ONE,
        targets: None,
        once_per_turn: false,
        condition: Some(Condition::SourceMatches(&TAPPED_F)),
    },
    TAP_FOR_MANA,
];

static COUNTING_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Triggered {
        trigger: UPKEEP,
        effects: BANK_ONE,
        targets: None,
        once_per_turn: false,
        condition: Some(Condition::ControlCount(&LAND_F, 2)),
    },
    TAP_FOR_MANA,
];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        land(WAKING_BASIN, "Waking Basin", WAKING_ABILITIES),
        land(BANKING_BASIN, "Banking Basin", BANKING_ABILITIES),
        land(COUNTING_BASIN, "Counting Basin", COUNTING_ABILITIES),
    ])
}

// ----------------------------------------------------------------- reading

fn storage(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(counters::STORAGE))
}

/// Whether an ability of this permanent was ever put on the stack.
///
/// This is what tells the rule's two checks apart. A first check that is
/// missing still ends with no counter on the land — the second check takes
/// the ability away again — so counting counters alone would pass against
/// half the rule.
fn ever_triggered(engine: &Engine<SyntheticLookup>, permanent: ObjectId) -> bool {
    engine.journal().entries().iter().any(|e| {
        matches!(
            e.event,
            crate::event::GameEvent::AbilityTriggered { source, .. } if source == permanent
        )
    })
}

fn journal_has(engine: &Engine<SyntheticLookup>, event: &crate::event::GameEvent) -> bool {
    engine.journal().entries().iter().any(|e| &e.event == event)
}

// ----------------------------------------------------------------- driving

/// Walks to the point in `seat`'s own turn where its upkeep is over.
///
/// The step and not a number of passes: the starting player skips the
/// first draw step, so "upkeep is over" is `Draw` on one turn and `Main`
/// on another, and a test that counted priorities would be reading a
/// different moment depending on who won the die roll.
fn past_own_upkeep(engine: &mut Engine<SyntheticLookup>, seat: PlayerId) {
    for _ in 0..400 {
        let turn = &engine.state().turn;
        if turn.active == seat && matches!(turn.step, Step::Draw | Step::Main) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected question: {pending:?}"
        );
    }
    panic!("the seat's own upkeep never came round");
}

/// Passes until `seat` holds priority with something on the stack, and
/// answers with the object that is on it.
fn trigger_on_stack(engine: &mut Engine<SyntheticLookup>, seat: PlayerId) -> ObjectId {
    for _ in 0..400 {
        if let Pending::Priority { player, .. } = engine.pending().clone() {
            let top = engine
                .state()
                .zones
                .list(ZoneLocation::Stack)
                .last()
                .copied();
            if let Some(top) = top
                && player == seat
            {
                return top;
            }
            engine.apply(player, PlayerAction::PassPriority).unwrap();
            continue;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected question: {pending:?}"
        );
    }
    panic!("no ability reached the stack");
}

/// Taps a permanent for mana through the offer, the way a player would.
fn tap_for_mana(engine: &mut Engine<SyntheticLookup>, seat: PlayerId, source: ObjectId) {
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("mana is activated at priority");
    };
    assert_eq!(player, seat, "it is this seat's priority");
    let (id, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == source)
        .expect("the engine offers the land's mana ability while the trigger waits");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index,
            },
        )
        .expect("the engine takes the ability it offered");
}

/// Lets the stack drain, whatever that turns out to mean for what is on it.
fn drain_the_stack(engine: &mut Engine<SyntheticLookup>) {
    for _ in 0..400 {
        if engine.state().zones.list(ZoneLocation::Stack).is_empty() {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected question: {pending:?}"
        );
    }
    panic!("the stack never drained");
}

// ------------------------------------------------------------------- tests

/// The first check, with its own counter-check on the same board: two
/// lands, the same trigger, opposite clauses, one upkeep.
///
/// Both halves matter and they fail in opposite directions. Without the
/// check at all, the banking basin's ability goes on the stack; with a
/// check that answers `false` to everything, the waking basin's never
/// does. Asserting on the journal rather than on the counters is what
/// makes this a test of the *first* check: with only the second one, both
/// lands still end the upkeep with the counters they have now.
#[test]
fn only_the_trigger_whose_clause_is_true_reaches_the_stack() {
    let f = forest();
    let mut engine = Engine::new(&preset(21, &[WAKING_BASIN, BANKING_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let waking = permanents(&engine, WAKING_BASIN)[0];
    let banking = permanents(&engine, BANKING_BASIN)[0];
    let seat = engine
        .state()
        .object(waking)
        .expect("the basin is on the battlefield")
        .controller;
    assert!(
        !tapped(&engine, waking) && !tapped(&engine, banking),
        "both basins start untapped, which is what makes the clauses differ"
    );

    past_own_upkeep(&mut engine, seat);

    assert!(
        ever_triggered(&engine, waking),
        "the clause was true, so the ability triggered"
    );
    assert!(
        !ever_triggered(&engine, banking),
        "the clause was false, so the ability never went on the stack at all"
    );
    assert_eq!(storage(&engine, waking), 1, "and the true one resolved");
    assert_eq!(
        storage(&engine, banking),
        0,
        "and the false one did nothing"
    );
}

/// The second check: the clause was true when the ability triggered and is
/// false by the time it would resolve, so the ability is removed from the
/// stack and does nothing.
///
/// The land taps itself for mana in response to its own trigger, which is
/// the cheapest way a player can change the answer between the two checks
/// — a mana ability does not use the stack (CR 605.1), so nothing else
/// joins the queue to muddy what is being read.
#[test]
fn a_clause_that_stops_being_true_takes_the_ability_off_the_stack() {
    let f = forest();
    let mut engine = Engine::new(&preset(22, &[WAKING_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let waking = permanents(&engine, WAKING_BASIN)[0];
    let seat = engine
        .state()
        .object(waking)
        .expect("the basin is on the battlefield")
        .controller;

    let ability = trigger_on_stack(&mut engine, seat);
    assert!(
        ever_triggered(&engine, waking),
        "the clause was true when the upkeep began"
    );

    tap_for_mana(&mut engine, seat, waking);
    assert!(tapped(&engine, waking), "and is no longer true");

    drain_the_stack(&mut engine);

    assert!(
        journal_has(
            &engine,
            &crate::event::GameEvent::StackObjectDidNotResolve { object: ability }
        ),
        "the ability left the stack without resolving"
    );
    assert!(
        !journal_has(
            &engine,
            &crate::event::GameEvent::StackObjectResolved { object: ability }
        ),
        "and the journal does not also say it resolved"
    );
    assert!(
        !journal_has(
            &engine,
            &crate::event::GameEvent::SpellCountered { object: ability }
        ),
        "nothing countered it — CR 603.4 is not a counter, and a card that \
         cares about being countered must not read one as the other"
    );
    assert_eq!(
        storage(&engine, waking),
        0,
        "an ability that does nothing puts no counter on anything"
    );
}

/// The counter-check for the one above: the same board, the same walk,
/// nothing tapped in response — and the ability resolves.
///
/// Without it, an engine that removed *every* conditional trigger from the
/// stack would pass the test above and be entirely broken.
#[test]
fn a_clause_still_true_at_resolution_resolves_as_any_other_ability() {
    let f = forest();
    let mut engine = Engine::new(&preset(22, &[WAKING_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let waking = permanents(&engine, WAKING_BASIN)[0];
    let seat = engine
        .state()
        .object(waking)
        .expect("the basin is on the battlefield")
        .controller;

    let ability = trigger_on_stack(&mut engine, seat);
    drain_the_stack(&mut engine);

    assert!(
        journal_has(
            &engine,
            &crate::event::GameEvent::StackObjectResolved { object: ability }
        ),
        "nothing changed between the checks, so it resolved"
    );
    assert_eq!(storage(&engine, waking), 1, "and put its counter on");
}

/// Whose game the clause is a sentence about: the controller of the
/// ability, at both checks.
///
/// "If you control two or more lands" is the only shape here that can tell
/// — a clause about the source answers the same for everyone. The board is
/// built so the two readings disagree: the basin's controller has two
/// lands and the other seat has none, so an engine asking the wrong player
/// would never trigger it, and one asking the wrong player *only at the
/// second check* would put it on the stack and take it away again. Both
/// halves of the assertion are needed to tell those two apart.
#[test]
fn the_clause_is_asked_of_the_ability_s_controller() {
    let f = forest();
    let mut engine = Engine::new(&preset(23, &[COUNTING_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let basin = permanents(&engine, COUNTING_BASIN)[0];
    let seat = engine
        .state()
        .object(basin)
        .expect("the basin is on the battlefield")
        .controller;
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller != seat))
            .count(),
        0,
        "the other seat controls nothing, so the two readings disagree"
    );

    let ability = trigger_on_stack(&mut engine, seat);
    drain_the_stack(&mut engine);

    assert!(
        journal_has(
            &engine,
            &crate::event::GameEvent::StackObjectResolved { object: ability }
        ),
        "the clause was asked of the seat that controls the lands, twice"
    );
    assert_eq!(storage(&engine, basin), 1, "and it did what it says");
}
