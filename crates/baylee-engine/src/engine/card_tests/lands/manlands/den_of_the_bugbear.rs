//! `cards/lands/manlands/den_of_the_bugbear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Den of the Bugbear: "If you control two or more other lands, this land enters tapped." / "{T}: Add {R}." / "{3}{R}: Until end of turn, this land becomes a 3/2 red Goblin creature..."
/// Under `Coverage::Partial`, the attack trigger creating an attacking Goblin token is omitted.
/// Playing this land while controlling two or more other lands enters tapped, and paying `{3}{R}` after untapping animates it into a 3/2 Goblin creature.
#[test]
fn den_of_the_bugbear_animates_into_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(310, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[den_of_the_bugbear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, den_of_the_bugbear());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, den_of_the_bugbear(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 2));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}
