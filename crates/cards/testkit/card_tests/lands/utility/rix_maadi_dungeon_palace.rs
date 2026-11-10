//! `cards/lands/utility/rix_maadi_dungeon_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rix Maadi, Dungeon Palace: "{T}: Add {C}." / "{1}{B}{R}, {T}: Each player discards a card. Activate only as a sorcery."
/// Paid with three lands, the sorcery-speed activation triggers symmetrical discard.
/// Both players are prompted in order to choose a card to discard to their graveyards.
#[test]
fn rix_maadi_dungeon_palace_forces_each_player_to_discard_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(129, forest())
        .battlefield(
            0,
            &[rix_maadi_dungeon_palace(), swamp(), mountain(), forest()],
        )
        // The kit deals no opening hand, and a player with nothing to discard
        // is not asked. Two cards each, so the prompt is a real choice.
        .hand(0, &[forest(), forest()])
        .hand(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace =
        on_battlefield(&engine, p0, rix_maadi_dungeon_palace()).expect("Rix Maadi deployed");
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, palace);
    activate(&mut engine, p0, rix_maadi_dungeon_palace(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 discard prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        player: p1_player,
        options: p1_options,
        min: p1_min,
        max: p1_max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 discard prompt, got {:?}", engine.pending());
    };
    assert_eq!(p1_player, p1);
    assert_eq!((p1_min, p1_max), (1, 1));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        p0_gy_before + 1,
        "p0 discarded 1 card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1,
        "p1 discarded 1 card"
    );
    assert!(is_tapped(&engine, palace));
}
