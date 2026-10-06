//! `cards/lands/manlands/hive_of_the_eye_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hive of the Eye Tyrant: "If you control two or more other lands, this land enters tapped." / "{T}: Add {B}." / "{3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature with menace..."
/// Under `Coverage::Partial`, the attack trigger targets an opponent's graveyard card rather than specifically the defending player's graveyard.
/// Playing this land while controlling two or more other lands enters tapped, and paying `{3}{B}` after untapping animates it into a 3/3 Beholder creature with menace.
#[test]
fn hive_of_the_eye_tyrant_animates_into_beholder_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hive_of_the_eye_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hive_of_the_eye_tyrant());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
}
