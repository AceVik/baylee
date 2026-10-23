#[allow(clippy::wildcard_imports)]
use super::*;

fn veil_of_birds() -> CardIndex {
    card_index("fc6278e5-7eb1-4590-880e-fe9efa0ec4d9")
}

#[test]
fn veil_of_birds_becomes_bird_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veil_of_birds()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, veil_of_birds()).expect("veil on bf");
    assert_eq!(pt(&engine, bird), (1, 1));
    assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
}
