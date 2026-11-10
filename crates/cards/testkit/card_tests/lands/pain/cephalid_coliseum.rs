//! `cards/lands/pain/cephalid_coliseum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cephalid Coliseum's mana ability is a whole sentence and not the usual
/// "{T}: Add {U}": the same tap also deals 1 damage to the land's own
/// controller. Because that tap is a printed ability rather than a basic
/// land type (CR 305.6), `tap_all_mana` never finds it and the land is
/// pressed by index — one activation then has to do three things at once,
/// put the {U} in the pool with no stack (CR 605.3b), take the life off
/// `p0` alone, and leave the Forest beside it standing, which is what says
/// the mana and the damage both came off the Coliseum and not off the
/// board. The threshold half of the card is the `Coverage::Partial` gap and
/// is deliberately never pressed.
#[test]
fn cephalid_coliseum_taps_for_blue_and_deals_its_own_controller_one_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cephalid_coliseum()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop, so a replacement effect would be seen: the printed
    // card has no enters-tapped clause and the land arrives upright.
    let land = play_land(&mut engine, p0, cephalid_coliseum());
    let bystander = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert!(
        !is_tapped(&engine, land),
        "no enters-tapped clause, so the land drop leaves it upright"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the tap"
    );

    // Ability 0 is "{T}: Add {U}. This land deals 1 damage to you."
    activate(&mut engine, p0, cephalid_coliseum(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "{{T}}: Add {{U}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !is_tapped(&engine, bystander),
        "and the Forest beside it never moved, so the {{U}} has no other \
         source on this board"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the land's controller pays \
         for the mana it makes"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is \"you\": the damage never crosses the table"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

/// Cephalid Coliseum: "Target player draws three cards, then discards three
/// cards." Both halves read the *same* seat, which is what
/// `PlayerRel::Chosen` is for — a card that drew for one player and made
/// another discard would pass a test that only counted the draw.
#[test]
fn cephalid_coliseum_draws_and_discards_three_at_threshold() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(19, forest())
        .battlefield(0, &[cephalid_coliseum(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, cephalid_coliseum()).expect("in play");
    tap_mana_except(&mut engine, p0, land);
    seed_graveyard(&mut engine, p0, 7);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("it targets a player, got {:?}", engine.pending())
    };
    assert!(player_options.contains(&p0));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p0],
            },
        )
        .unwrap();
    drive_to_rest(&mut engine, p0);
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 3,
        "three cards left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "three drawn and three discarded is a hand the same size — which \
         is the assertion a draw-only reading would fail"
    );
}
