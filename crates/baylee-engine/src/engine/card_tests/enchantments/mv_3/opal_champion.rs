#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn opal_champion() -> CardIndex {
    card_index("0d3617c3-1526-40e6-bc90-6139b5d3e1f1")
}

#[test]
fn opal_champion_becomes_3_3_first_strike_knight_when_opponent_casts_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_champion()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let champion = on_battlefield(&engine, p0, opal_champion()).unwrap();
    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(champion)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(pt(&engine, champion), (3, 3));
}

#[test]
fn opal_champion_ignores_own_creatures_and_opponent_noncreatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[opal_champion()],
        &[],
        &[gray_ogre()],
        &[lightning_bolt()],
    );
    let champion = lk::obj(&engine, p0, opal_champion());
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p0, gray_ogre(), &[], &[]);
    assert!(
        engine
            .state()
            .object(champion)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    lk::pass_once(&mut engine);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, p1, lightning_bolt(), &[], &[p0]);
    assert!(
        engine
            .state()
            .object(champion)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
}
