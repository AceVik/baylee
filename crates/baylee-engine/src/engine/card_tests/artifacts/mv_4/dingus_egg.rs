//! `cards/artifacts/mv_4/dingus_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dingus Egg: "Whenever a land is put into a graveyard from the
/// battlefield, this artifact deals 2 damage to that land's controller."
/// Destroyed the harness way (`bury`), which is the graveyard door this
/// sentence names — a bounced or exiled land never triggers it.
#[test]
fn dingus_egg_deals_2_when_a_land_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dingus_egg(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, mountain()).expect("the Mountain is seated");
    let before = life_of(&engine, p0);
    kill(&mut engine, land);
    assert_eq!(
        life_of(&engine, p0),
        before - 2,
        "its controller took 2 as it hit the graveyard"
    );
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
}
