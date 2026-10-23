#[allow(clippy::wildcard_imports)]
use super::*;

fn veiled_serpent() -> CardIndex {
    card_index("ff59a95c-28de-44cf-abbe-772417851ffa")
}

#[test]
fn veiled_serpent_becomes_serpent_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veiled_serpent()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let serpent = on_battlefield(&engine, p0, veiled_serpent()).expect("serpent on bf");
    assert_eq!(pt(&engine, serpent), (4, 4));
}
