#[allow(clippy::wildcard_imports)]
use super::*;

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
    assert_eq!(pt(&engine, sentry), (1, 1));
}
