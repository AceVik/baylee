//! `cards/creatures/artifacts/mv_6/phyrexian_devourer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phyrexian_devourer() -> CardIndex {
    card_index("28b1ed69-784c-4642-86c7-104792d5afc2")
}

/// Phyrexian Devourer ({6} 1/1 Artifact Creature — Phyrexian Construct, Partial):
/// "When this creature's power is 7 or greater, sacrifice it."
#[test]
fn phyrexian_devourer_sacrifices_itself_when_its_power_reaches_seven() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phyrexian_devourer(), forest(), forest()])
        .hand(0, &[giant_growth(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let devourer = on_battlefield(&engine, p0, phyrexian_devourer()).unwrap();
    assert_eq!(pt(&engine, devourer), (1, 1));

    tap_all_mana(&mut engine, p0);

    // First Giant Growth: 4/4 (remains on battlefield).
    cast_with_floating(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![devourer],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, devourer), (4, 4));
    assert!(on_battlefield(&engine, p0, phyrexian_devourer()).is_some());

    // Second Giant Growth: 7/7 -> state trigger sacrifices Phyrexian Devourer.
    cast_with_floating(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![devourer],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, phyrexian_devourer()).is_some());
}
