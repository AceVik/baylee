//! `cards/sorceries/mv_4/reign_of_chaos.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::savannah_lions;

fn reign_of_chaos() -> CardIndex {
    card_index("436b5989-9b8c-4e03-a82e-046837c891ca")
}

fn storm_crow() -> CardIndex {
    baylee_core::generated::index::STORM_CROW
}

/// Reign of Chaos (`{2}{R}{R}`, `Coverage::Partial`): modal sorcery whose first
/// mode requires target Plains and target white creature (`second_targets`) and
/// whose second mode requires target Island and target blue creature, destroying
/// the targeted land.
#[test]
fn reign_of_chaos_modes_target_land_and_creature_and_destroy_targeted_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Mode 0: target Plains and target white creature.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[plains(), island(), savannah_lions(), storm_crow()])
        .hand(0, &[reign_of_chaos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_plains = on_battlefield(&engine, p1, plains()).unwrap();
    let their_island = on_battlefield(&engine, p1, island()).unwrap();
    let their_lion = on_battlefield(&engine, p1, savannah_lions()).unwrap();

    cast_from_hand(&mut engine, p0, reign_of_chaos());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    let first_menu = aim_at(&mut engine, p0, their_plains);
    assert!(first_menu.contains(&their_plains));
    assert!(!first_menu.contains(&their_island));
    let second_menu = aim_at(&mut engine, p0, their_lion);
    assert_eq!(second_menu, vec![their_lion]);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, plains()).is_none());
    assert!(on_battlefield(&engine, p1, island()).is_some());
}
