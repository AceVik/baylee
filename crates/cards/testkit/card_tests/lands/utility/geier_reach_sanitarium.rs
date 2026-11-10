//! `cards/lands/utility/geier_reach_sanitarium.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Geier Reach Sanitarium: "{2}, {T}: Each player draws a card, then discards a card."
/// Two Forests pay {2} while Geier Reach Sanitarium taps to activate its symmetrical looting ability.
/// Both players draw a card and are each sequentially prompted to discard a card to their graveyards.
#[test]
fn geier_reach_sanitarium_each_player_draws_and_discards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[geier_reach_sanitarium(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanitarium =
        on_battlefield(&engine, p0, geier_reach_sanitarium()).expect("Sanitarium deployed");
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, sanitarium);
    activate(&mut engine, p0, geier_reach_sanitarium(), 1);

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
        panic!("expected p0 discard prompt, got {:?}", engine.pending())
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
        panic!("expected p1 discard prompt, got {:?}", engine.pending())
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
        "p0 discarded one card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1,
        "p1 discarded one card"
    );
    assert!(is_tapped(&engine, sanitarium));
}
