//! `cards/artifacts/mv_1/candelabra_of_tawnos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Candelabra of Tawnos: "{X}, {T}: Untap X target lands."
///
/// X is announced before targets (CR 601.2b, through CR 602.2b) and is the
/// count: at X = 2 the question is two lands, not one and not three, and the
/// same land cannot fill both slots (CR 115.3). The 2004 rulings leave the
/// menu wide — an untapped land and an opponent's land are both legal — and
/// the untapping happens on resolution, because this is no mana ability.
#[test]
fn candelabra_of_tawnos_untaps_exactly_the_x_lands_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[candelabra_of_tawnos(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cane = on_battlefield(&engine, p0, candelabra_of_tawnos()).expect("the Candelabra is out");

    // Two Forests pay {2}; the third stays up as the untapped target the
    // first ruling allows, and p1's Forest is the second half of it.
    let mine = all_on_battlefield(&engine, p0, forest());
    let (left_tapped, right_tapped, untouched) = (mine[0], mine[1], mine[2]);
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    tap_mana_except(&mut engine, p0, untouched);
    assert!(is_tapped(&engine, left_tapped) && is_tapped(&engine, right_tapped));

    activate(&mut engine, p0, candelabra_of_tawnos(), 0);
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "X is a question the activator answers, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(min, 0, "X = 0 is legal");
    assert!(max >= 2, "two green is floating, so X = 2 is affordable");
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("X = 2 asks for two lands, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (2, 2), "X target lands is exactly X");
    for land in [left_tapped, right_tapped, untouched, theirs] {
        assert!(options.contains(&land), "a land is a land: {options:?}");
    }
    assert!(
        !options.contains(&cane),
        "the Candelabra is an artifact, not a land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![left_tapped, left_tapped],
                players: vec![],
            },
        )
        .expect_err("one land cannot fill both of the two slots (CR 115.3)");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![left_tapped],
                players: vec![],
            },
        )
        .expect_err("X = 2 is two lands, not one");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![left_tapped, right_tapped, theirs],
                players: vec![],
            },
        )
        .expect_err("and not three");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![left_tapped, right_tapped],
                players: vec![],
            },
        )
        .expect("the two lands X names");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, left_tapped) && !is_tapped(&engine, right_tapped),
        "both named lands untap on resolution"
    );
    assert!(
        is_tapped(&engine, cane),
        "the Candelabra tapped for its own cost"
    );
    assert!(
        !is_tapped(&engine, untouched) && !is_tapped(&engine, theirs),
        "the lands the ability did not name are untouched"
    );
}
