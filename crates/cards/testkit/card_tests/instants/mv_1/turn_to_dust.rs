//! `cards/instants/mv_1/turn_to_dust.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Turn to Dust — {G} instant: "Destroy target Equipment. Add {G}."
///
/// Two Equipments stand on the table, one under each seat, and one tapped
/// Forest is the whole board's mana: the offer naming exactly those two
/// artifacts is the printed word "Equipment" being read — it reaches across
/// the table (CR 115.1) and leaves the land beside it alone — and answering
/// with the one across the table is what says so. The single Forest is also
/// what makes "Add {G}" legible: it is spent paying for the spell, so once
/// the target question closes the pool is empty and the green standing in it
/// after resolution has no source on this board but the spell itself.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn turn_to_dust_destroys_the_equipment_it_names_and_adds_a_green_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), basilisk_collar()])
        .hand(0, &[turn_to_dust()])
        // An Equipment on the other side of the table: "target Equipment" is
        // not "an Equipment you control".
        .battlefield(1, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, basilisk_collar()).expect("my Equipment is out");
    let theirs = on_battlefield(&engine, p1, lightning_greaves()).expect("their Equipment is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        1,
        "the Forest is the only mana source on the board"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest, one green"
    );

    // Mana before the claim: `castable` is filtered through `can_afford`,
    // which reads the pool and not the untapped land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, turn_to_dust()).expect("the instant is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{G}} is in the pool, so the instant is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, turn_to_dust());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Equipment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one Equipment");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "an Equipment on either side of the table is a legal target: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is no Equipment, so the filter is read and not skipped: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2c before CR 601.2h: the target is named while the {{G}} is still floating"
    );
    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_some(),
        "and nothing has been destroyed while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Equipment across the table was one of the options");

    // CR 601.2h: the cost is the last step, so the {G} is gone the moment
    // the target question closes — and the spell is on the stack, not done.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} is paid on announcement, after the target"
    );
    assert!(
        !stack_is_empty(&engine),
        "Turn to Dust is no mana ability, so it is waiting to resolve"
    );
    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_some(),
        "and the targeted Equipment is still on the table"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_none(),
        "the targeted Equipment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, lightning_greaves()).is_some(),
        "a destroyed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, basilisk_collar()).is_some(),
        "and only the Equipment that was named: mine still stands"
    );
    assert!(
        in_graveyard(&engine, p0, turn_to_dust()).is_some(),
        "the instant itself went to its owner's graveyard"
    );
    assert!(
        is_tapped(&engine, land),
        "the Forest paid the {{G}}, so it is tapped and can produce nothing more"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"Add {{G}}\" — one green in the pool, and the board's only land is spent"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
}
