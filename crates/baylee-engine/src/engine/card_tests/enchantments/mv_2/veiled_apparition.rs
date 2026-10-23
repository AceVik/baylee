#[allow(clippy::wildcard_imports)]
use super::*;

fn veiled_apparition() -> CardIndex {
    card_index("4187fd6d-5660-4ea7-a9dd-402487ec8721")
}

#[test]
fn veiled_apparition_becomes_illusion_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veiled_apparition()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let illusion = on_battlefield(&engine, p0, veiled_apparition()).expect("apparition on bf");
    assert_eq!(pt(&engine, illusion), (3, 3));
    assert!(keywords(&engine, illusion).contains(KeywordSet::FLYING));
}
