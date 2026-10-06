//! `cards/lands/manlands/restless_vents.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Vents: "This land enters tapped." / "{T}: Add {B} or {R}." / "{1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land."
/// Under `Coverage::Partial`, the attack trigger tying discard to draw is omitted.
/// Playing this land causes it to enter tapped, and paying `{1}{B}{R}` after untapping animates it into a 2/3 Insect creature with menace.
#[test]
fn restless_vents_animates_into_insect_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(317, forest())
        .battlefield(0, &[swamp(), mountain(), forest()])
        .hand(0, &[restless_vents()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_vents());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_vents(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
}
