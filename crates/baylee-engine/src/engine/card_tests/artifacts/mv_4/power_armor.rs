//! `cards/artifacts/mv_4/power_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Power Armor ({4}, artifact) prints one line: "{3}, {T}: Target creature
/// gets +1/+1 until end of turn for each basic land type among lands you
/// control." That number *is* the card, so the board is built to make three
/// readings disagree: this seat controls seven lands holding four basic land
/// types between them (Plains, Island, Swamp, Mountain — the second Island
/// and two of the Mountains repeat a type), while the opponent's Forest is
/// the one basic type this side is missing. Counting lands would give
/// +7/+7, reading the whole table's types +5/+5, and the printed sentence
/// +4/+4 — which is what has to land on the 1/1 Elf.
#[test]
fn power_armor_pumps_for_the_basic_land_types_you_control_and_for_no_other_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                island(),
                island(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[power_armor()])
        // The basic type this side lacks, and a creature "target creature"
        // has to offer and must not pump.
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: {4} off the seven lands,
    // leaving the {3} the ability charges beside it in the pool. A pool
    // survives until the step ends (CR 500.5), and the whole test plays in
    // this one main phase.
    cast_from_hand(&mut engine, p0, power_armor());
    pass_until(&mut engine, stack_is_empty);
    let armor = on_battlefield(&engine, p0, power_armor()).expect("the Armor resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(!is_tapped(&engine, armor), "and the Armor is untapped");

    activate(&mut engine, p0, power_armor(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&armor),
        "the Armor is an artifact and no creature: {options:?}"
    );

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so the tap
    // and the {3} are read here rather than before the answer.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, armor), "{{T}} is paid for the ability");
    assert!(!stack_is_empty(&engine), "and it is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (5, 5),
        "+1/+1 for each of the four basic land types this seat's lands hold — \
         not seven for the lands, and not five for the type the opponent's \
         Forest would add"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
}
