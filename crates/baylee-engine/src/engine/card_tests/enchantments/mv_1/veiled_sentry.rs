#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn veiled_sentry() -> CardIndex {
    card_index("0b9b8e60-51eb-4d4a-95c1-48ca5b8153df")
}

#[test]
fn veiled_sentry_becomes_illusion_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veiled_sentry()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let sentry = on_battlefield(&engine, p0, veiled_sentry()).expect("sentry on bf");
    assert_eq!(pt(&engine, sentry), (3, 3));
}

#[test]
fn veiled_sentry_ignores_own_spells_and_uses_zero_for_a_zero_mana_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let ornithopter = baylee_core::generated::index::ORNITHOPTER;
    let mut engine = lk::game(
        SEED,
        &[veiled_sentry()],
        &[],
        &[gray_ogre()],
        &[ornithopter],
    );
    let sentry = lk::obj(&engine, p0, veiled_sentry());
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p0, gray_ogre(), &[], &[]);
    assert!(
        engine
            .state()
            .object(sentry)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    reach_their_main_phase(&mut engine, p1);
    lk::cast_at(&mut engine, p1, ornithopter, &[], &[]);
    assert!(
        in_graveyard(&engine, p0, veiled_sentry()).is_some(),
        "zero mana value produces a 0/0, not a 1/1"
    );
}
