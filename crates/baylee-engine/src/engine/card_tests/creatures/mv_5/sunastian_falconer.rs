//! `cards/creatures/mv_5/sunastian_falconer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunastian Falconer is a `{3}{R}{G}` legendary 4/4 whose whole printed text
/// is "{T}: Add {C}{C}". Both halves of that are the engine's answer rather
/// than the card's, so both are played: three Mountains and two Forests pay
/// the printed `{3}{R}{G}` down to an exactly empty pool, and the Falconer then
/// pays its own tap symbol for two *colourless* mana that no land on this board
/// could have produced — the Forests make green and the Mountains make red.
/// A turn passes before the tap is taken, because a creature that arrived this
/// turn is summoning sick (CR 302.6) and its `{T}` is not payable at all; the
/// untapped Forest read afterwards is what says the two mana came off the
/// creature and not off a land.
#[test]
fn sunastian_falconer_taps_for_two_colorless_off_an_empty_pool() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest(), forest()])
        .hand(0, &[sunastian_falconer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five lands into the pool: exactly the {3}{R}{G} the card costs, so the
    // pool is empty the moment the Falconer lands and nothing floating can be
    // mistaken for the mana its own ability makes below.
    cast_from_hand(&mut engine, p0, sunastian_falconer());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let falconer =
        on_battlefield(&engine, p0, sunastian_falconer()).expect("the Falconer resolved");
    assert_eq!(pt(&engine, falconer), (4, 4), "the body the card prints");
    assert!(
        types(&engine, falconer).contains(TypeSet::CREATURE),
        "and the permanent it arrived as is a creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Mountains and two Forests paid the {{3}}{{R}}{{G}} to the last mana"
    );
    // The land this test reads back afterwards, so that "the two colourless are
    // the Falconer's" is a statement about a board and not a default.
    let land = on_battlefield(&engine, p0, forest()).expect("a Forest is still out");

    // CR 302.6: a creature that came under its controller's control this turn
    // has summoning sickness, so its {T} is not offered until the next turn has
    // begun. The untap step also stands every land back up, which is what makes
    // "no land was tapped for it" a reading rather than a leftover.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, falconer),
        "the Falconer is standing and no longer sick"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0 is the printed "{T}: Add {C}{C}", whose whole price is its own
    // tap — so the claim is made on an empty pool, where the engine reads the
    // offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(falconer, 0)),
        "an untapped Falconer is a paid {{T}}, so the one line the card prints \
         is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sunastian_falconer(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{C}}{{C}}` is fixed, so nothing is asked on the way (CR 605.1), got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the two Forests are untapped and make green besides, so this is no \
         land's mana mislabelled"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(
        is_tapped(&engine, falconer),
        "the Falconer paid its own {{T}}"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Forest beside it never moved, so the two mana have no other source \
         on this board"
    );

    // The tap is spent, so the line is no longer one the seat may take — read
    // off the offer, which is where an unpayable cost goes.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(falconer, 0)),
        "a tapped Falconer has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
