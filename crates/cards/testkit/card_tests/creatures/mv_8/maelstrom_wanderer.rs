//! `cards/creatures/mv_8/maelstrom_wanderer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maelstrom Wanderer: "creatures you control have haste", which is only
/// visible on a creature that has just arrived.
///
/// A creature cast this turn attacking is the shape that separates a granted
/// haste from a board the harness happened to seat early. The two cascades are
/// the test after this one.
#[test]
fn maelstrom_wanderer_gives_a_freshly_cast_creature_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(396, forest())
        .battlefield(0, &[maelstrom_wanderer(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf arrived this turn");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::HASTE),
        "the Wanderer grants haste to the creatures you control"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attacker declaration")
    };
    assert!(
        attackers.contains(&elf),
        "and CR 302.6 lets it attack the turn it arrived: {attackers:?}"
    );
    let _ = p1;
}

/// "Cascade, cascade": two triggers as the Wanderer is cast (CR 702.85c).
///
/// The library is Forest, Llanowar Elves, Forest, Air Elemental from the top.
/// The first cascade exiles the Forest and stops on the Elves (a nonland card
/// under eight), and its "you may cast it without paying its mana cost" is
/// taken: the Elves go on the stack above the second trigger — a creature
/// spell cast while the stack is full, and for no mana — and the Forest goes
/// to the bottom. The second cascade exiles the other Forest and stops on the
/// Air Elemental; declined, both go to the bottom. The Wanderer resolves last
/// and the Elves have haste.
#[test]
fn maelstrom_wanderer_cascades_twice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(396, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                maelstrom_wanderer(),
                air_elemental(),
                llanowar_elves(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elemental = hand_to_library_top(&mut engine, p0, air_elemental());
    let second_cover = hand_to_library_top(&mut engine, p0, forest());
    let elves = hand_to_library_top(&mut engine, p0, llanowar_elves());
    let first_cover = hand_to_library_top(&mut engine, p0, forest());
    let library = ZoneLocation::Library(p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, maelstrom_wanderer());
    let wanderer = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .first()
        .copied()
        .expect("the Wanderer is on the stack");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        3,
        "two cascade triggers above the spell"
    );

    assert_eq!(
        cascade_offer(&mut engine),
        elves,
        "the first nonland card under eight is the Elves"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        3,
        "the Elves, the second cascade, the Wanderer"
    );
    assert_eq!(stack.last(), Some(&elves), "cast as the cascade resolved");
    assert_eq!(stack.first(), Some(&wanderer));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "without paying its mana cost"
    );
    assert_eq!(
        engine.state().zones.list(library).first(),
        Some(&first_cover),
        "the Forest it passed goes to the bottom"
    );

    assert_eq!(cascade_offer(&mut engine), elemental);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&elves),
        "the Elves resolved before the second cascade"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let bottom = &engine.state().zones.list(library)[..3];
    assert!(
        bottom.contains(&elemental) && bottom.contains(&second_cover),
        "declined, the Air Elemental goes to the bottom with the Forest"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, maelstrom_wanderer()).is_some());
    assert!(
        keywords(&engine, elves).contains(KeywordSet::HASTE),
        "creatures you control have haste"
    );
}
