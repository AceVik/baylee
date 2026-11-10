//! `cards/lands/loci/trenchpost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Trenchpost: "{T}: Add {C}." / "{3}, {T}: Target player mills a card for each Locus you control."
/// With Trenchpost and a second Locus under control, the activated ability is pointed at the opponent.
/// Upon resolution, the opponent mills two cards into their graveyard.
#[test]
fn trenchpost_mills_target_player_per_locus_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(107, forest())
        .battlefield(
            0,
            &[trenchpost(), glimmerpost(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let trench = on_battlefield(&engine, p0, trenchpost()).expect("Trenchpost deployed");
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, trench);
    activate(&mut engine, p0, trenchpost(), 1);

    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(player_options.contains(&p1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 2,
        "opponent milled 2 cards for 2 Loci you control"
    );
    assert!(is_tapped(&engine, trench));
}
