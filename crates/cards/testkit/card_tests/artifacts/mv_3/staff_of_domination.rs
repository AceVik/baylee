//! `cards/artifacts/mv_3/staff_of_domination.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Staff of Domination prints five activated abilities and the first one is
/// what makes the rest a loop: `{1}: Untap this artifact` is the only reason a
/// card that must tap to do anything can act more than once in a turn. The
/// scenario plays every printed line for real — one life, a card, a tap and an
/// untap — with the `{1}` between each, so all four `{T}` prices and the untap
/// stand on the board rather than in the card file, inside one main phase
/// (CR 500.5). The only creature on the table is the opponent's, because
/// "target creature" is not "target creature you control": the tap and the
/// untap differ by one word and the elf has to be offered to both.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn staff_of_domination_loops_its_own_tap_for_life_a_card_and_a_tapped_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 20])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[staff_of_domination()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The card arrives the way the card arrives: {3} out of a pool the Forests
    // actually filled, and every line below is paid from what is left of it.
    cast_from_hand(&mut engine, p0, staff_of_domination());
    pass_until(&mut engine, stack_is_empty);
    let staff = on_battlefield(&engine, p0, staff_of_domination()).expect("the Staff resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        17,
        "twenty Forests less the {{3}} the card costs"
    );
    assert!(!is_tapped(&engine, staff), "and it enters untapped");

    // Ability 1: `{2}, {T}: You gain 1 life.`
    activate(&mut engine, p0, staff_of_domination(), 1);
    assert!(
        is_tapped(&engine, staff),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        15,
        "the {{2}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "one activation, one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid, not to the opponent"
    );

    // Ability 0: `{1}: Untap this artifact.` Without it the Staff is spent for
    // the turn after a single line, which is the whole point of the card.
    activate(&mut engine, p0, staff_of_domination(), 0);
    assert!(
        is_tapped(&engine, staff),
        "an untap is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, staff), "{{1}} buys the untap");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        14,
        "and the {{1}} came out of the pool"
    );

    // Ability 4: `{5}, {T}: Draw a card.`
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, staff_of_domination(), 4);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "the {{5}} came out of the pool"
    );

    // Ability 3: `{4}, {T}: Tap target creature.`
    activate(&mut engine, p0, staff_of_domination(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, staff),
        "the second {{1}} stands it up again"
    );

    activate(&mut engine, p0, staff_of_domination(), 3);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&staff),
        "the Staff is an artifact and no creature: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");
    assert!(
        !is_tapped(&engine, elf),
        "CR 601.2h pays last: the target is answered before the {{T}} and the {{4}}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, elf), "\"Tap target creature\"");
    assert!(
        is_tapped(&engine, staff),
        "and the Staff paid its own {{T}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the untap and the {{4}} are both out of the pool"
    );

    // Ability 2: `{3}, {T}: Untap target creature.` — the same elf, back up.
    activate(&mut engine, p0, staff_of_domination(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, staff),
        "and the third {{1}} stands the Staff up for its last line"
    );
    assert!(
        is_tapped(&engine, elf),
        "the Elf is still the creature the previous ability tapped"
    );

    activate(&mut engine, p0, staff_of_domination(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the other way round is the same filter on the same board: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, elf), "\"Untap target creature\"");
    assert!(
        is_tapped(&engine, staff),
        "and the loop ends with the Staff spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the last {{3}} is the last mana the twenty Forests made"
    );
}
