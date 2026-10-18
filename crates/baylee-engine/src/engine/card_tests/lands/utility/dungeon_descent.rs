//! `cards/lands/utility/dungeon_descent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Both lands enter tapped and make their printed mana, and both offer their
/// second activated ability when its complete activation price is floating.
#[test]
fn dungeon_and_howltooth_enter_tapped_and_offer_their_implemented_abilities() {
    let p0 = PlayerId::new(0);

    for (card, colour, payment) in [
        (
            dungeon_descent(),
            ManaColor::Colorless,
            &[forest(), forest(), forest(), forest(), jin_gitaxias()][..],
        ),
        (howltooth_hollow(), ManaColor::Black, &[swamp()][..]),
    ] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, payment)
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let land = play_land(&mut engine, p0, card);
        assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

        cross_into_the_next_own_main(&mut engine, p0);
        // The price is **floated**, not merely stood on the board:
        // `can_afford` reads the pool, so an ability with any mana cost at
        // all is absent from an offer taken over untapped lands — which is
        // how this pin first passed for the wrong reason. Everything but the
        // land itself is tapped, because the land is what both the mana
        // ability and the second ability tap.
        tap_mana_except(&mut engine, p0, land);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert_eq!(
            legal.abilities.iter().filter(|(id, _)| *id == land).count(),
            2,
            "the complete activation price is floating in the pool"
        );

        let before = engine.state().players[0].mana_pool.available(colour);
        let before_total = engine.state().players[0].mana_pool.total();
        activate(&mut engine, p0, card, 0);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            before + 1,
            "the one colour its mana ability prints"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            before_total + 1
        );
    }
}

#[test]
fn dungeon_descent_taps_legendary_creature_and_ventures_into_the_dungeon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dungeon_descent(),
                forest(),
                forest(),
                forest(),
                forest(),
                jin_gitaxias(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let descent =
        on_battlefield(&engine, p0, dungeon_descent()).expect("Dungeon Descent on battlefield");
    let jin = on_battlefield(&engine, p0, jin_gitaxias()).expect("Jin-Gitaxias on battlefield");

    tap_mana_except(&mut engine, p0, descent);
    activate(&mut engine, p0, dungeon_descent(), 1);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected TapOther choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert!(options.contains(&jin));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![jin] })
        .expect("tap Jin-Gitaxias to pay Dungeon Descent activation cost");

    assert!(is_tapped(&engine, descent));
    assert!(
        is_tapped(&engine, jin),
        "untapped legendary creature was tapped to pay the cost"
    );

    // Room 1 of Lost Mine of Phandelver (Cave Entrance) triggers Scry 1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        cards,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected scry from Cave Entrance, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(piles, scry_piles(1));
    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("finish scry 1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].dungeon_room,
        1,
        "player ventured into room 1 of the dungeon"
    );
}
