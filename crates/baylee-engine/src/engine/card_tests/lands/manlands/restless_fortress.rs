//! `cards/lands/manlands/restless_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Fortress: "This land enters tapped." / "{T}: Add {W} or {B}." / "{2}{W}{B}: This land becomes a 1/4 white and black Nightmare creature until end of turn. It's still a land."
/// Under `Coverage::Partial`, the attack trigger's life drain targets the opponent seat rather than specifically defending player.
/// Playing this land causes it to enter tapped, and paying `{2}{W}{B}` after untapping animates it into a 1/4 Nightmare creature.
#[test]
fn restless_fortress_animates_into_nightmare() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[plains(), plains(), swamp(), swamp()])
        .hand(0, &[restless_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_fortress());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_fortress(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (1, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}
