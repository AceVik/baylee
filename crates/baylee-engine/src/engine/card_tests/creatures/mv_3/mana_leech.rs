#[allow(clippy::wildcard_imports)]
use super::*;

fn mana_leech() -> CardIndex {
    card_index("cad05528-917b-4296-9d23-ba9433612709")
}

#[test]
fn mana_leech_taps_target_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mana_leech()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_land = on_battlefield(&engine, p1, forest()).expect("seated");
    activate(&mut engine, p0, mana_leech(), 1);
    aim_at(&mut engine, p0, opp_land);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, opp_land));
}
