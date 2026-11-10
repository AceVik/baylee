//! `cards/creatures/mv_4/flametongue_kavu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flametongue Kavu prints a 4/2 body and one triggered ability: "When this
/// creature enters, it deals 4 damage to target creature."
///
/// The filter names no controller, so the board carries a creature of mine
/// beside the Elves across the table and the trigger's menu has to offer all
/// three — a `Filter::YOUR_CREATURE` or an opponent-only filter would satisfy
/// every other assertion here. The Elf that was named dies while the Elf
/// nobody named and my own stand: that is what tells a targeted ability from
/// one that sweeps a side of the table. The 4/2 body is read off the permanent
/// so the trigger is not the only thing the card is claimed to do.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn flametongue_kavu_shoots_the_one_creature_it_names_and_leaves_the_rest_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[flametongue_kavu()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (victim, bystander) = (theirs[0], theirs[1]);

    // Four Mountains and only those: the Elf is named as the printing kept
    // back because it is one of the creatures the trigger's menu is read
    // against, and {3}{R} wants four mana and no green at all.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Mountains, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, flametongue_kavu());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the Kavu's controller names the target");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&mine),
        "\"target creature\" is not \"target creature an opponent controls\": {options:?}"
    );
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "and both Elves across the table are on the same menu: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target creature\" names no player, so the other half of the choice stays empty: \
         {player_options:?}"
    );
    for option in &options {
        assert!(
            types(&engine, *option).contains(TypeSet::CREATURE),
            "only creatures are offered: {options:?}"
        );
    }

    // CR 601.2c: the target is named as the trigger is announced, so nothing
    // has been dealt yet — and the Kavu is already a permanent, which is what
    // puts it on the battlefield its own 4/2 body is read from.
    let kavu = on_battlefield(&engine, p0, flametongue_kavu()).expect("the Kavu resolved");
    assert_eq!(pt(&engine, kavu), (4, 2), "the body the card prints");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&victim),
        "the Elf it is aimed at is still standing while the question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf the question offered was chosen");
    assert!(
        !stack_is_empty(&engine),
        "the trigger is what resolves, not the announcement"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&victim),
        "four damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine
            .state()
            .object(bystander)
            .expect("the other Elf is an object")
            .zone,
        Zone::Battlefield,
        "the Elf nobody named never moved: the ability targets one creature, not a side \
         of the table"
    );
    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("my Elf is an object")
            .zone,
        Zone::Battlefield,
        "and neither did the creature of mine the menu offered"
    );
    assert!(
        on_battlefield(&engine, p0, flametongue_kavu()).is_some(),
        "the Kavu outlives the damage its own trigger dealt"
    );
}
