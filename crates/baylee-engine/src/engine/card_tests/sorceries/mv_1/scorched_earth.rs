//! `cards/sorceries/mv_1/scorched_earth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scorched_earth() -> CardIndex {
    card_index("fd1ce3e0-9c2d-48b6-9a14-4e71921cdfa4")
}

/// `Scorched Earth` ({X}{R}, Sorcery, `Coverage::Partial`):
/// discards one land card as an additional cost and destroys X target lands.
#[test]
fn scorched_earth_discards_a_land_and_destroys_x_target_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[plains(), island()])
        .hand(0, &[scorched_earth(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_plains = on_battlefield(&engine, p1, plains()).unwrap();
    let p1_island = on_battlefield(&engine, p1, island()).unwrap();
    let hand_land = in_hand(&engine, p0, forest()).unwrap();

    cast_from_hand(&mut engine, p0, scorched_earth());
    // Choose X = 2 ({2}{R} = 3 mana).
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_plains, p1_island],
                players: vec![],
            },
        )
        .unwrap();
    // Mandatory additional cost: discard a land card.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![hand_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p1, plains()).is_none());
    assert!(on_battlefield(&engine, p1, island()).is_none());
}
