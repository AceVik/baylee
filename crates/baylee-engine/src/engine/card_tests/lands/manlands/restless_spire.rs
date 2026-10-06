//! `cards/lands/manlands/restless_spire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Spire: "This land enters tapped." / "{T}: Add {U} or {R}." / "{U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature..."
/// Under `Coverage::Partial`, the granted first strike during your turn is omitted because no modifier grants turn-conditional keywords.
/// Playing this land causes it to enter tapped, and paying `{U}{R}` after untapping animates it into a 2/1 Elemental creature.
#[test]
fn restless_spire_animates_into_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(316, forest())
        .battlefield(0, &[island(), mountain()])
        .hand(0, &[restless_spire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_spire());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_spire(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 1));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}
