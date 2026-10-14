//! `cards/creatures/mv_8/benthic_behemoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn benthic_behemoth() -> CardIndex {
    card_index("aea98950-536e-451d-bd25-ae6e3e459ffb")
}

/// Benthic Behemoth is a 7/6 Islandwalker for {5}{U}{U}{U}.
#[test]
fn benthic_behemoth_is_cast_for_eight_with_islandwalk() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 8])
        .hand(0, &[benthic_behemoth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, PlayerId::new(0)));
    let serpent = cast_and_settle(&mut engine, benthic_behemoth());
    assert_eq!(pt(&engine, serpent), (7, 6));
    assert!(keywords(&engine, serpent).contains(KeywordSet::ISLANDWALK));
}

/// Islandwalk: the defending player's Island makes the Behemoth unblockable
/// for their bear; the same bear blocks it when the defender has only a
/// Forest.
#[test]
fn benthic_behemoth_cannot_be_blocked_only_when_the_defender_has_an_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (land, blockable) in [(island(), false), (forest(), true)] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[benthic_behemoth()])
            .battlefield(1, &[grizzly_bears(), land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let serpent = on_battlefield(&engine, p0, benthic_behemoth()).expect("it is out");
        let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the bear is out");
        let blocks = attack_player(&mut engine, serpent);
        assert_eq!(
            blocks.iter().any(|b| b.blocker == bear),
            blockable,
            "with {land:?} under the defender: {blocks:?}"
        );
    }
}
