//! `cards/sorceries/mv_7/sea_gate_restoration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sea Gate Restoration` // `Sea Gate, Reborn`: "Draw cards equal to the
/// number of cards in your hand plus one. You have no maximum hand size for the rest of the game.
/// // As this land enters, you may pay 3 life. If you don't, it enters tapped. {T}: Add {U}."
///
/// The back face; the front face's draw is the next test. The test plays the land face via
/// `play_land_face`, pays 3 life on the `EnterModifier::TappedOrPayLife(3)` prompt to enter
/// untapped, and immediately activates its mana ability to add `{U}` to the pool.
#[test]
fn sea_gate_reborn_pays_three_life_to_enter_untapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(sea_gate_restoration(), 1).expect("plays as Sea Gate, Reborn");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life to enter untapped"
    );

    let blue_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);
    activate(&mut engine, p0, sea_gate_restoration(), 0);

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        blue_before + 1,
        "adds one blue mana to the pool"
    );
}

/// Sea Gate Restoration's front face: "Draw cards equal to the number of
/// cards in your hand plus one."
///
/// Three cards stay in hand while it resolves, so it draws four and not
/// three (the "plus one") and not five (the spell itself is on the stack,
/// not in the hand). The number is read once: a draw that re-read the hand
/// after each card would never stop growing, and the library says it did.
#[test]
fn sea_gate_restoration_draws_the_hand_plus_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 7])
        .hand(0, &[sea_gate_restoration(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let held = hand(&engine) - 1;
    let library = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, sea_gate_restoration());
    assert_eq!(hand(&engine), held, "the spell left the hand for the stack");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library - (held + 1),
        "{held} cards in hand, plus one"
    );
    assert_eq!(hand(&engine), held + held + 1);
}
