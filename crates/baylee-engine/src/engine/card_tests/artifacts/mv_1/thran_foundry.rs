//! `cards/artifacts/mv_1/thran_foundry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::zone::Zone;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn thran_foundry() -> CardIndex {
    card_index("88079144-8d8e-4f98-9bee-3762540c94ca")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn thran_foundry_exiles_itself_to_shuffle_target_players_graveyard_into_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[thran_foundry(), grizzly_bears(), giant_growth()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    hand_to_graveyard(&mut engine, P0, grizzly_bears());
    hand_to_graveyard(&mut engine, P0, giant_growth());
    hand_to_graveyard(&mut engine, P1, hill_giant());

    // Cast Thran Foundry for {1}, leaving one Forest untapped.
    let lands = lands_of(&engine, P0);
    tap_mana_where(&mut engine, P0, |id| id == lands[0]);
    cast_with_floating(&mut engine, P0, thran_foundry());
    pass_until(&mut engine, stack_is_empty);
    let foundry = on_battlefield(&engine, P0, thran_foundry()).expect("Thran Foundry resolved");

    // Negative boundary: without {1} in the pool, the ability is not offered.
    assert!(!lk::offered(&engine, thran_foundry(), P0, 0));

    let p0_lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        1
    );

    tap_mana_where(&mut engine, P0, |id| id == lands[1]);
    assert!(lk::offered(&engine, thran_foundry(), P0, 0));
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: foundry,
                ability_index: 0,
            },
        )
        .expect("activates for {1}, {T}, Exile Thran Foundry");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("targets P0");

    // ExileSelf is paid on activation: Thran Foundry is already in exile, not
    // the graveyard, even before the ability resolves.
    assert!(on_battlefield(&engine, P0, thran_foundry()).is_none());
    assert!(in_graveyard(&engine, P0, thran_foundry()).is_none());
    assert_eq!(lk::zone_of(&engine, foundry), Zone::Exile);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .is_empty(),
        "target player's graveyard was shuffled into their library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        p0_lib_before + 2,
        "both graveyard cards returned to P0's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        1,
        "the non-targeted player's graveyard is untouched"
    );
}
