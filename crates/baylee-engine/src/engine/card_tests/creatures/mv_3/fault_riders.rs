//! `cards/creatures/mv_3/fault_riders.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fault Riders — {2}{R}, a 2/2 — "Sacrifice a land: This creature gets +2/+0
/// and gains first strike until end of turn. Activate only once each turn."
///
/// The cost names no particular land, so the engine has to ask which one, and
/// that menu is half the card: all four Mountains this seat controls are on it
/// (a Mountain tapped for the cast is still a permanent its controller may
/// sacrifice), while the Elf beside them is a creature and the Forest across
/// the table is not this seat's. The pump needs a bystander of its own — a
/// second creature under the same seat that must stay a printed 1/1 with no
/// keyword — and the "only once each turn" clause is read off a board where
/// three lands still stand to pay it, so an absent second offer can only be
/// the printed limit.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fault_riders_eats_a_land_for_two_power_and_first_strike_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_creature(),
            ],
        )
        // A land across the table, so "you control" is read and not assumed.
        .battlefield(1, &[forest()])
        .hand(0, &[fault_riders()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One Mountain is left standing: {2}{R} is what the Riders cost, and every
    // other source on the board pays it, so the lands the ability is about to
    // eat are all still on the table afterwards.
    let kept_land = on_battlefield(&engine, p0, mountain()).expect("a Mountain is out");
    tap_mana_except(&mut engine, p0, kept_land);
    cast_with_floating(&mut engine, p0, fault_riders());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let riders = on_battlefield(&engine, p0, fault_riders()).expect("the Riders resolved");
    let bystander = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(
        pt(&engine, riders),
        (2, 2),
        "the printed 2/2, with nothing given up yet"
    );
    assert!(
        !keywords(&engine, riders).contains(KeywordSet::FIRST_STRIKE),
        "and no keyword on a card whose own text is the only source of one"
    );

    let lands = lands_of(&engine, p0);
    assert_eq!(lands.len(), 4, "all four Mountains are still on the table");
    let fodder = lands[0];
    let pool_before = engine.state().players[0].mana_pool.total();

    activate(&mut engine, p0, fault_riders(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost names a land and not a particular one, so it asks which: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        4,
        "the four Mountains this seat controls and nothing else: {options:?}"
    );
    for land in &lands {
        assert!(
            options.contains(land),
            "every land under my control is on the menu, tapped or not: {options:?}"
        );
    }
    assert!(
        !options.contains(&bystander),
        "the Elf is a creature, and the cost names a land: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's Forest is not yours to sacrifice: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the land the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&fodder),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "exactly one land was given up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before,
        "the price is a land and no mana, so the pool is exactly where it was"
    );

    assert_eq!(
        pt(&engine, riders),
        (4, 2),
        "+2/+0 on the creature the ability names"
    );
    assert!(
        keywords(&engine, riders).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike arrives with it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf beside it is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "\"This creature\" is not \"creatures you control\""
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the land across the table never moved"
    );

    // The second activation. The price is still payable — three Mountains
    // stand, and the cost asks for no mana and no untapped permanent — so an
    // absent offer here is the printed limit and nothing else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(riders, 0)),
        "\"Activate only once each turn\": the ability is off the offer with \
         three lands still standing to pay it: {:?}",
        legal.abilities
    );
}
