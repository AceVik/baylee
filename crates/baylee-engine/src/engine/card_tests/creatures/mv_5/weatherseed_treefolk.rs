//! `cards/creatures/mv_5/weatherseed_treefolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn weatherseed_treefolk() -> CardIndex {
    card_index("4e9b878b-80a2-433b-941b-92d854e83d58")
}

#[test]
fn weatherseed_treefolk_returns_to_owners_hand_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[weatherseed_treefolk(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let treefolk = on_battlefield(&engine, p0, weatherseed_treefolk()).unwrap();
    assert!(keywords(&engine, treefolk).contains(KeywordSet::TRAMPLE));

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![treefolk],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, weatherseed_treefolk()).is_none());
    assert!(in_graveyard(&engine, p0, weatherseed_treefolk()).is_none());
    assert!(in_hand(&engine, p0, weatherseed_treefolk()).is_some());
}

#[test]
fn weatherseed_treefolk_tramples_past_a_chump() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[weatherseed_treefolk()],
        &[llanowar_elves()],
        &[],
        &[],
    );
    let treefolk = lk::obj(&engine, lk::P0, weatherseed_treefolk());
    let elf = lk::obj(&engine, lk::P1, llanowar_elves());
    lk::attack(&mut engine, &[treefolk]);
    lk::block(&mut engine, &[(elf, treefolk)]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, lk::P1), 16);
}
