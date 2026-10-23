#[allow(clippy::wildcard_imports)]
use super::*;

fn disorder() -> CardIndex {
    card_index("586da84b-1384-4da2-983d-340c22b73eb8")
}

#[test]
fn disorder_damages_white_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[disorder()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, disorder());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, ondu_cleric()).is_none());
}
