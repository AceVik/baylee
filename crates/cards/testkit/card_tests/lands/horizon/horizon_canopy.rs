//! `cards/lands/horizon/horizon_canopy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Horizon Canopy prints two activated abilities and the card is only itself
/// when both are paid for: "{T}, Pay 1 life: Add {G} or {W}" and "{1}, {T},
/// Sacrifice this land: Draw a card." The mana ability's whole price is not
/// its own tap, so `tap_all_mana` never presses it (#159) — the tap and the
/// life are this scenario's own doing — and the question it asks has to be
/// exactly two colours wide, because a five-colour `ChooseColor` would mean
/// the card had been read as "any colour". The draw is played a turn later on
/// the same land, since a tapped land gets no abilities back until the untap
/// step (CR 502.3), and the half that has to show is the sacrifice: the
/// library shrinks by one and the Canopy itself is in the graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn horizon_canopy_pays_life_for_green_or_white_and_later_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[horizon_canopy(), forest()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let canopy = on_battlefield(&engine, p0, horizon_canopy()).expect("the Canopy is out");
    assert!(
        !is_tapped(&engine, canopy),
        "nothing on the card taps it on the way in"
    );
    assert_eq!(engine.state().players[0].life, 20, "nothing is paid yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the Forest beside it is untapped, so the pool is empty"
    );

    // Ability 0: "{T}, Pay 1 life: Add {G} or {W}."
    activate(&mut engine, p0, horizon_canopy(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two kinds of mana are printable, so the engine asks rather than \
         picking one: {options:?}"
    );
    for color in [ManaColor::Green, ManaColor::White] {
        assert!(
            options.contains(&color),
            "the card prints {color:?}: {options:?}"
        );
    }
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the two");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one activation: the Forest was never tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is the other half of the price"
    );
    assert!(is_tapped(&engine, canopy), "and {{T}} is one of them");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // The untap step is the only way a tapped land gets its abilities back,
    // so a whole turn has to pass before the second line can be paid for.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, canopy),
        "the untap step stood the Canopy back up"
    );

    // Ability 1: "{1}, {T}, Sacrifice this land: Draw a card." The {1} comes
    // off the Forest, and the Canopy is named as the source kept back so that
    // the tap is the ability's own payment rather than the helper's.
    let taken = tap_mana_except(&mut engine, p0, canopy);
    assert_eq!(taken, 1, "the one Forest, with the Canopy kept untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}} in the pool and nothing else — the mana the first ability made \
         is gone with the phase it was made in"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(canopy, 1)),
        "a floating {{1}} is what makes the draw affordable: {:?}",
        legal.abilities
    );

    let library = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, horizon_canopy(), 1);
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, horizon_canopy()).is_some(),
        "\"Sacrifice this land\" — the card it pays with is itself"
    );
    assert!(
        on_battlefield(&engine, p0, horizon_canopy()).is_none(),
        "so the Canopy is off the battlefield rather than merely tapped"
    );
    assert_eq!(library_size(&engine, p0), library - 1, "\"Draw a card\"");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "and the card is in hand — a card that left the library for anywhere \
         else would satisfy the count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} in the pool was the price"
    );
}
