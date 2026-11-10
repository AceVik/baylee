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

/// "Deals 2 damage to that land's controller": the land's, not the Egg's. An
/// opponent's land that dies costs the opponent 2 and our life stays put; a
/// creature of theirs that dies — not a land — costs nothing.
#[test]
fn dingus_egg_hurts_the_controller_of_the_land_that_died() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dingus_egg()])
        .battlefield(1, &[mountain(), mountain(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let (ours_before, theirs_before) = (life_of(&engine, p0), life_of(&engine, p1));
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their creature");
    kill(&mut engine, elf);
    assert_eq!(
        life_of(&engine, p1),
        theirs_before,
        "a creature is not a land"
    );

    let lands = all_on_battlefield(&engine, p1, mountain());
    kill(&mut engine, lands[0]);
    assert_eq!(
        life_of(&engine, p1),
        theirs_before - 2,
        "the land's controller, the opponent, took 2"
    );
    kill(&mut engine, lands[1]);
    assert_eq!(life_of(&engine, p1), theirs_before - 4, "once per land");
    assert_eq!(
        life_of(&engine, p0),
        ours_before,
        "the Egg's own controller is not the one hurt"
    );
}
