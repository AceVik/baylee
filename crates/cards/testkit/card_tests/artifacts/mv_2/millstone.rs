//! `cards/artifacts/mv_2/millstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Millstone` prints `{{2}}, {{T}}: Target player mills two cards.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Millstone` and two copies of `forest()`.
/// Paying two mana activates `Millstone` targeting seat 1 via `Pending::ChooseTargets`,
/// putting two cards from the opponent's library into their graveyard upon resolution.
#[test]
fn millstone_mills_two_cards_from_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), millstone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mill = on_battlefield(&engine, p0, millstone()).expect("millstone on battlefield");
    let before_lib = library_size(&engine, p1);
    let before_gy = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_all_mana_but(&mut engine, p0, Some(millstone()));
    activate(&mut engine, p0, millstone(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&p1), "opponent is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeted opponent");

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, mill), "`Millstone` is tapped");
    assert_eq!(
        library_size(&engine, p1),
        before_lib - 2,
        "two cards milled from opponent library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        before_gy + 2,
        "two cards placed into opponent graveyard"
    );
}
