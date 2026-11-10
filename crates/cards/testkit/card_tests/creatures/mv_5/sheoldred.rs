//! `cards/creatures/mv_5/sheoldred.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sheoldred // The True Scriptures: "{4}{B}: Exile Sheoldred, then return
/// it to the battlefield transformed ... only if an opponent has eight or
/// more cards in their graveyard." Chapter II: "Each opponent discards three
/// cards, then mills three cards." Chapter III: "Put all creature cards from
/// all graveyards onto the battlefield under your control. Exile this Saga,
/// then return it to the battlefield (front face up)."
///
/// One game plays all of it: the flip on turn one, chapter II on the
/// controller's next turn, chapter III on the one after.
#[allow(clippy::too_many_lines)] // One card's three chapters, played end to end.
#[test]
fn sheoldred_flips_to_a_saga_whose_chapters_two_and_three_discard_mill_and_reanimate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let gy = |e: &Engine<RegistryLookup>, s: PlayerId| {
        e.state().zones.list(ZoneLocation::Graveyard(s)).len()
    };
    let hand =
        |e: &Engine<RegistryLookup>, s: PlayerId| e.state().zones.list(ZoneLocation::Hand(s)).len();
    let mut engine = Duel::new(4301, llanowar_elves())
        .battlefield(
            0,
            &[
                sheoldred_praetor(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(1, &[llanowar_elves(), llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, p1, 8);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sheoldred_praetor(), 1);
    run_answering(&mut engine, stack_is_empty);
    let saga = on_battlefield(&engine, p0, sheoldred_praetor()).expect("the Saga stands");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        1,
        "chapter I's counter"
    );
    assert!(
        !types(&engine, saga).contains(TypeSet::CREATURE),
        "the back face is an enchantment"
    );

    let (hand0, lib0, gy0) = (hand(&engine, p1), their_library(&engine), gy(&engine, p1));
    assert_eq!((hand0, gy0), (3, 8));

    run_answering(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && stack_is_empty(e)
    });
    assert_eq!(counters_on(&engine, saga, CounterKind::Lore), 2);
    assert_eq!(
        hand(&engine, p1),
        hand0 + 1 - 3,
        "one drawn on their turn, three discarded"
    );
    assert_eq!(
        their_library(&engine),
        lib0 - 1 - 3,
        "one drawn, three milled"
    );
    assert_eq!(
        gy(&engine, p1),
        gy0 + 3 + 3,
        "three discarded and three milled"
    );

    run_answering(&mut engine, |e| {
        e.state().turn.number >= 5
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && stack_is_empty(e)
    });
    assert_eq!(
        gy(&engine, p1),
        0,
        "every creature card left their graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        gy0 + 6,
        "every creature card came to the controller's side"
    );
    let front = on_battlefield(&engine, p0, sheoldred_praetor()).expect("Sheoldred is back");
    assert!(
        types(&engine, front).contains(TypeSet::CREATURE),
        "front face up"
    );
    assert_eq!(pt(&engine, front), (4, 5));
}

/// Sheoldred // The True Scriptures: the back face prints no mana cost and a
/// black color indicator, and "an object is each color denoted by its color
/// indicator" (CR 202.2e). Without the indicator the Saga was colorless.
#[test]
fn the_true_scriptures_is_black_after_the_flip() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8305, forest())
        .battlefield(0, &[sheoldred_praetor()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let praetor =
        on_battlefield(&e, p0, sheoldred_praetor()).expect("Sheoldred is on the battlefield");
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(praetor).unwrap().characteristics().colors;
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "Sheoldred is black by her {{3}}{{B}}{{B}} cost"
    );

    let def = baylee_cards::by_index(sheoldred_praetor()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(praetor, def, 1));
    assert_eq!(
        e.state().object(praetor).unwrap().face_index,
        1,
        "The True Scriptures is up"
    );
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "The True Scriptures has no mana cost and a black color indicator, so it is black and only black (CR 202.2e)"
    );
}
