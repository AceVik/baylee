#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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

#[test]
fn veil_of_birds_does_not_animate_for_its_controllers_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(SEED, &[veil_of_birds()], &[], &[gray_ogre()], &[]);
    let veil = lk::obj(&engine, p0, veil_of_birds());
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p0, gray_ogre(), &[], &[]);
    assert!(
        engine
            .state()
            .object(veil)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(
        !engine
            .state()
            .object(veil)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}
