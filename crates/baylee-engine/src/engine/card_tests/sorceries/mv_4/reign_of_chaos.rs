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
    assert!(
        on_battlefield(&engine, p1, savannah_lions()).is_some(),
        "NOT SUPPORTED: the targeted white creature is not destroyed (Effect::Destroy reads only the first target)"
    );
    assert!(on_battlefield(&engine, p1, storm_crow()).is_some());
}

/// The second mode: target Island and target blue creature. The menus are
/// the mirror of the first mode's (the Plains and the white creature are not
/// on them), and only the Island goes.
#[test]
fn reign_of_chaos_second_mode_targets_island_and_blue_creature_and_destroys_the_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
    let their_crow = on_battlefield(&engine, p1, storm_crow()).unwrap();

    cast_from_hand(&mut engine, p0, reign_of_chaos());
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    let first_menu = aim_at(&mut engine, p0, their_island);
    assert!(first_menu.contains(&their_island));
    assert!(
        !first_menu.contains(&their_plains),
        "mode 2 names an Island, not a Plains"
    );
    let second_menu = aim_at(&mut engine, p0, their_crow);
    assert_eq!(
        second_menu,
        vec![their_crow],
        "only the blue creature may be the second target"
    );
    assert!(!second_menu.contains(&their_lion));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, island()).is_none());
    assert!(
        on_battlefield(&engine, p1, plains()).is_some(),
        "the Plains is not the chosen land"
    );
    assert!(on_battlefield(&engine, p1, savannah_lions()).is_some());
    assert!(
        on_battlefield(&engine, p1, storm_crow()).is_some(),
        "NOT SUPPORTED: the targeted blue creature is not destroyed (Effect::Destroy reads only the first target)"
    );
}
