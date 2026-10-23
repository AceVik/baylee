#[allow(clippy::wildcard_imports)]
use super::*;

fn vug_lizard() -> CardIndex {
    card_index("ac9ec838-fbad-4f01-846b-3084e1d37a44")
}

#[test]
fn vug_lizard_has_mountainwalk_and_echo() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[vug_lizard()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[mountain(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, vug_lizard());
    pass_until(&mut engine, stack_is_empty);

    let liz = on_battlefield(&engine, p0, vug_lizard()).expect("lizard on bf");
    assert!(keywords(&engine, liz).contains(KeywordSet::MOUNTAINWALK));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, vug_lizard()).is_none());
}
