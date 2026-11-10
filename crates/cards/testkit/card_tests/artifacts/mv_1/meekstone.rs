//! `cards/artifacts/mv_1/meekstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Meekstone: "Creatures with power 3 or greater don't untap during their
/// controllers' untap steps." A power-4 Golem stays tapped across an untap
/// step; a power-1 Elf beside it untaps normally.
#[test]
fn meekstone_keeps_power_3_or_more_creatures_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[meekstone(), obsianus_golem(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, obsianus_golem()).expect("seated");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    for id in [golem, elf] {
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up")
            .object_mut(id)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
    });
    assert!(is_tapped(&engine, golem), "power 4: stays tapped");
    assert!(!is_tapped(&engine, elf), "power 1: untaps normally");
}

/// Meekstone binds "their controllers' untap steps", whoever's they are: with
/// the stone on our side, the opponent's power-4 Golem and power-3 Giant stay
/// tapped through *their* untap step (3 is "3 or greater"), while their power-2
/// Ogre untaps normally.
#[test]
fn meekstone_keeps_the_opponents_big_creatures_tapped_at_the_boundary() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[meekstone()])
        .battlefield(1, &[obsianus_golem(), hill_giant(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("power 4");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("power 3");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("power 2");
    assert_eq!(
        pt(&engine, giant).0,
        3,
        "the boundary case is a power-3 creature"
    );
    assert_eq!(
        pt(&engine, ogre).0,
        2,
        "and the one below it a power-2 creature"
    );
    for id in [golem, giant, ogre] {
        engine
            .dev_state_mut(p1)
            .expect("the harness may set boards up")
            .object_mut(id)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, golem), "power 4: stays tapped");
    assert!(is_tapped(&engine, giant), "power 3: stays tapped");
    assert!(!is_tapped(&engine, ogre), "power 2: untaps normally");
}
