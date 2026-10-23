#[allow(clippy::wildcard_imports)]
use super::*;

fn repopulate() -> CardIndex {
    card_index("3a225d39-c1f8-4ab0-98ec-dad63bf268db")
}

#[test]
fn repopulate_shuffles_graveyard_into_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[repopulate(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, gray_ogre());

    cast_from_hand(&mut engine, p0, repopulate());
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, gray_ogre()).is_none());
}
