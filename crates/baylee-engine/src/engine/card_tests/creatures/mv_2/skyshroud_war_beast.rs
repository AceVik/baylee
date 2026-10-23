#[allow(clippy::wildcard_imports)]
use super::*;

fn skyshroud_war_beast() -> CardIndex {
    card_index("c6ad434b-6e34-439b-8046-f0047c06e371")
}

#[test]
fn skyshroud_war_beast_scales_with_opponent_nonbasic_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[skyshroud_war_beast()])
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[badlands(), badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, skyshroud_war_beast());
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, skyshroud_war_beast()).expect("seated");
    assert_eq!(pt(&engine, beast), (2, 2));
}
