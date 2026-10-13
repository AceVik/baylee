//! `PlayerAction::PayWindow`: a payment window paid in one action.
//!
//! A seat that says it will pay ("pay {1}" for Rhystic Study, Esper
//! Sentinel's {X}, Pact of Negation's upkeep, a spell's own cost) is handed a
//! CR 605.3a window. The client stages its taps and sends them as one
//! action; the engine applies them in order, each through the door it would
//! take alone, then settles the payment, and refuses the whole action —
//! leaving the game exactly as it was — when anything about it fails.

use super::testkit::*;
use super::*;
use crate::object::Status;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPayment};

const SEED: u64 = 4_211;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn rhystic_study() -> CardIndex {
    card_index("53236dd7-845a-444c-96d5-f41ed7325d8f")
}

fn esper_sentinel() -> CardIndex {
    card_index("5def9f38-0a0b-4e8d-9f9d-29dcb46520b4")
}

fn pact_of_negation() -> CardIndex {
    card_index("f3e213a4-ba5a-468a-93b3-c0a34e1bd725")
}

/// "{T}, Pay 1 life: Add one mana of any color."
fn mana_confluence() -> CardIndex {
    card_index("d0ee5bdc-2b69-4b73-9a20-ffcc18783b29")
}

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::TAPPED))
}

/// `seat`'s permanents that are copies of `card`, in battlefield order.
fn all_of(engine: &Engine<RegistryLookup>, owner: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == owner && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

fn hand_size(engine: &Engine<RegistryLookup>, n: u8) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat(n))).len()
}

fn is_tax_question(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::PayTax { .. },
            ..
        }
    )
}

fn tap(id: ObjectId) -> PlayerAction {
    PlayerAction::ActivateManaAbility { source: id }
}

/// Everything a refused action must leave as it was.
fn fingerprint(engine: &Engine<RegistryLookup>) -> (u64, usize, String) {
    (
        engine.snapshot_hash(),
        engine.state().journal.len(),
        format!("{:?}", engine.pending()),
    )
}

/// Seat 0's Rhystic Study; seat 1 on its main phase with two Forests, one of
/// them tapped to cast a Llanowar Elves, and the tax question asked of it.
fn rhystic_tax_asked() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[rhystic_study()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    let forests = all_of(&engine, seat(1), forest());
    let (first, second) = (forests[0], forests[1]);
    engine.apply(seat(1), tap(first)).unwrap();
    cast_with_floating(&mut engine, seat(1), llanowar_elves());
    pass_until(&mut engine, is_tax_question);
    (engine, second)
}

/// Saying yes with an empty pool opens a window owing {1}, and one action
/// that taps the second Forest pays it: the Study's controller is never asked
/// whether to draw, and draws nothing.
#[test]
fn rhystic_study_s_tax_is_paid_by_one_payment_action() {
    let (mut engine, second) = rhystic_tax_asked();
    let before = hand_size(&engine, 0);
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((seat(1), ManaPayment::Fixed(ManaCost::parse("{1}")))),
        "yes with nothing floating opens a window for the {{1}}"
    );

    engine
        .apply(
            seat(1),
            PlayerAction::PayWindow {
                steps: vec![tap(second)],
            },
        )
        .expect("one untapped Forest pays {1}");

    assert!(engine.payment_window().is_none(), "the window is settled");
    assert!(is_tapped(&engine, second), "the Forest was tapped for it");
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, 0),
        before,
        "the tax was paid, so the Study's controller draws nothing"
    );
}

/// A payment that does not cover what is owed is refused, and refused whole:
/// the snapshot, the journal and the question are what they were, and the
/// window still stands for a payment that does.
#[test]
fn a_payment_that_falls_short_changes_nothing() {
    let (mut engine, second) = rhystic_tax_asked();
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    let before = fingerprint(&engine);

    let short = engine.apply(seat(1), PlayerAction::PayWindow { steps: vec![] });
    assert!(short.is_err(), "an empty pool pays nothing: {short:?}");
    assert_eq!(
        fingerprint(&engine),
        before,
        "a refused payment moved nothing"
    );

    // A step that is not a mana ability or an answer to one is refused too,
    // even though passing alone would have been legal here.
    let passing = engine.apply(
        seat(1),
        PlayerAction::PayWindow {
            steps: vec![PlayerAction::PassPriority],
        },
    );
    assert!(
        passing.is_err(),
        "passing is not a payment step: {passing:?}"
    );
    assert_eq!(fingerprint(&engine), before);
    assert!(!is_tapped(&engine, second));

    engine
        .apply(
            seat(1),
            PlayerAction::PayWindow {
                steps: vec![tap(second)],
            },
        )
        .expect("and the window still takes the payment that covers it");
}

/// Outside a payment window there is nothing to settle: the action is
/// refused at plain priority and at the yes-or-no question before it.
#[test]
fn a_payment_action_needs_an_open_window() {
    let (mut engine, second) = rhystic_tax_asked();
    let before = fingerprint(&engine);
    let at_the_question = engine.apply(
        seat(1),
        PlayerAction::PayWindow {
            steps: vec![tap(second)],
        },
    );
    assert!(at_the_question.is_err(), "{at_the_question:?}");
    assert_eq!(fingerprint(&engine), before);
}

/// Esper Sentinel's "unless that player pays {X}": the first noncreature
/// spell's caster says yes, and one action taps what X asks for.
#[test]
fn esper_sentinel_s_tax_is_paid_by_one_payment_action() {
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[esper_sentinel()])
        .battlefield(2, &[forest(), forest()])
        .hand(2, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(2));
    let forests = all_of(&engine, seat(2), forest());
    engine.apply(seat(2), tap(forests[0])).unwrap();
    cast_with_floating(&mut engine, seat(2), sol_ring());
    pass_until(&mut engine, is_tax_question);
    let before = hand_size(&engine, 0);

    engine.apply(seat(2), PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((seat(2), ManaPayment::Fixed(ManaCost::parse("{1}")))),
        "X is the Sentinel's power, 1"
    );
    engine
        .apply(
            seat(2),
            PlayerAction::PayWindow {
                steps: vec![tap(forests[1])],
            },
        )
        .expect("the second Forest pays {1}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, 0),
        before,
        "paid: the Sentinel draws nothing"
    );
    assert!(
        all_of(&engine, seat(2), sol_ring()).len() == 1,
        "and the Sol Ring resolved"
    );
}

/// Seat 0 with Pact of Negation resolved off its own Elves, at the next
/// upkeep, having said it will pay {3}{U}{U}. The board is `lands`.
fn pact_window(lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = seat(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, lands)
        .hand(0, &[llanowar_elves(), pact_of_negation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    // The Elves' {G}: one Forest, the only land the Pact's price cannot use.
    let forest = all_of(&engine, p0, forest())[0];
    engine.apply(p0, tap(forest)).unwrap();
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, pact_of_negation());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayPact { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((p0, ManaPayment::Fixed(ManaCost::parse("{3}{U}{U}"))))
    );
    engine
}

/// Pact of Negation's upkeep price, paid by one action of five Islands: the
/// seat is still in the game. Before it, a four-Island payment is refused
/// and loses nothing — the seat neither loses the game nor taps a land.
#[test]
fn pact_of_negation_s_upkeep_is_paid_by_one_payment_action() {
    let p0 = seat(0);
    let mut lands = vec![island(); 5];
    lands.push(forest());
    let mut engine = pact_window(&lands);
    let islands = all_of(&engine, p0, island());
    let before = fingerprint(&engine);

    let four = engine.apply(
        p0,
        PlayerAction::PayWindow {
            steps: islands[..4].iter().copied().map(tap).collect(),
        },
    );
    assert!(
        four.is_err(),
        "four Islands do not pay {{3}}{{U}}{{U}}: {four:?}"
    );
    assert_eq!(
        fingerprint(&engine),
        before,
        "the short payment moved nothing"
    );
    assert!(islands.iter().all(|&i| !is_tapped(&engine, i)));
    assert!(!engine.state().players[0].has_lost());

    engine
        .apply(
            p0,
            PlayerAction::PayWindow {
                steps: islands.iter().copied().map(tap).collect(),
            },
        )
        .expect("five Islands pay it");
    assert!(engine.payment_window().is_none());
    assert!(!engine.state().players[0].has_lost(), "paid, so not lost");
    assert!(islands.iter().all(|&i| is_tapped(&engine, i)));
}

/// A mana ability that asks a colour and pays life is a step followed by its
/// answer, and its life is paid in the listed order and only when the whole
/// payment goes through: a refused payment costs no life.
#[test]
fn a_step_s_question_is_answered_inside_the_payment_and_its_cost_is_undone_with_it() {
    let p0 = seat(0);
    let mut lands = vec![island(); 3];
    lands.extend([mana_confluence(), mana_confluence(), forest()]);
    let mut engine = pact_window(&lands);
    let islands = all_of(&engine, p0, island());
    let confluences = all_of(&engine, p0, mana_confluence());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the window is a priority offer: {:?}", engine.pending())
    };
    let index = legal
        .abilities
        .iter()
        .find(|(source, _)| *source == confluences[0])
        .map(|(_, index)| *index)
        .expect("Mana Confluence is offered in the window");
    let confluence = |id: ObjectId| PlayerAction::ActivateAbility {
        source: id,
        ability_index: index,
    };
    let life = engine.state().players[0].life;

    // The colour left unanswered: refused, nothing paid.
    let before = fingerprint(&engine);
    let open = engine.apply(
        p0,
        PlayerAction::PayWindow {
            steps: vec![confluence(confluences[0])],
        },
    );
    assert!(open.is_err(), "a question left open refuses it: {open:?}");
    assert_eq!(fingerprint(&engine), before);
    assert_eq!(engine.state().players[0].life, life, "no life was paid");

    let mut steps: Vec<PlayerAction> = islands.iter().copied().map(tap).collect();
    for &c in &confluences {
        steps.push(confluence(c));
        steps.push(PlayerAction::ChooseColor(ManaColor::Blue));
    }
    engine
        .apply(p0, PlayerAction::PayWindow { steps })
        .expect("three Islands and two blue from the Confluences pay {3}{U}{U}");
    assert!(!engine.state().players[0].has_lost());
    assert_eq!(
        engine.state().players[0].life,
        life - 2,
        "each Confluence charged its life as it was activated"
    );
}

/// The window a spell's own cost opens (CR 601.2g) takes the same action: a
/// Llanowar Elves cast with nothing floating is paid by one action, and a
/// payment that is short leaves the card in the hand and the window open.
#[test]
fn a_spell_s_own_payment_window_takes_the_same_action() {
    let p0 = seat(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let forest = all_of(&engine, p0, forest())[0];
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .expect("a payable card may be cast before its mana is made");
    assert_eq!(
        engine.payment_window(),
        Some((p0, ManaPayment::Fixed(ManaCost::parse("{G}"))))
    );

    let before = fingerprint(&engine);
    assert!(
        engine
            .apply(p0, PlayerAction::PayWindow { steps: vec![] })
            .is_err()
    );
    assert_eq!(fingerprint(&engine), before, "short: nothing moved");
    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());

    engine
        .apply(
            p0,
            PlayerAction::PayWindow {
                steps: vec![tap(forest)],
            },
        )
        .expect("the Forest pays {G}");
    assert!(engine.payment_window().is_none());
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the cast is complete and the Elves are on the stack"
    );
}
