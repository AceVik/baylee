//! `cards/artifacts/mv_2/amulet_of_kroog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Amulet of Kroog: "{2}, {T}: Prevent the next 1 damage that would be dealt
/// to any target this turn."
///
/// The printed number is read off a Bolt: 3 damage leaves the 3/3 Giant with
/// 2 marked, which is true only if exactly one point was prevented (CR
/// 615.1's prevention shield). "Any target" (CR 115.4) reaches the creature
/// and either player, and the {{2}}, {{T}} price is read off the tapped
/// Amulet.
#[test]
fn amulet_of_kroog_prevents_exactly_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                amulet_of_kroog(),
                mountain(),
                mountain(),
                mountain(),
                hill_giant(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet = on_battlefield(&engine, p0, amulet_of_kroog()).expect("the Amulet is out");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("the Giant is out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amulet_of_kroog(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("any target is a question, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&giant),
        "a creature is any target: {options:?}"
    );
    assert_eq!(player_options.len(), 2, "and so is either player");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().shields.len(), 1, "one prevention shield");
    assert!(is_tapped(&engine, amulet), "the {{2}}, {{T}} cost was paid");

    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant is still there")
            .damage,
        2,
        "3 dealt, 1 prevented"
    );
    assert!(engine.state().shields.is_empty(), "and the shield is spent");
}
