//! `cards/creatures/artifacts/mv_3/prakhata_pillar_bug.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prakhata_pillar_bug() -> CardIndex {
    card_index("1ec95969-4a6f-49f3-af23-1680276e7da2")
}

/// Prakhata Pillar-Bug — "{B}: This creature gains lifelink until end of
/// turn." After paying, its 2 damage to the opponent is 2 life for its
/// controller; it has no lifelink before.
#[test]
fn prakhata_pillar_bug_gains_lifelink_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[prakhata_pillar_bug(), swamp()]);
    let bug = on_battlefield(&engine, p0, prakhata_pillar_bug()).expect("seated");
    assert_eq!(pt(&engine, bug), (2, 3));
    assert!(!keywords(&engine, bug).contains(KeywordSet::LIFELINK));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, prakhata_pillar_bug(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bug).contains(KeywordSet::LIFELINK));

    attack_to_blocks(&mut engine, &[bug]);
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(life_of(&engine, 1), 18);
    assert_eq!(life_of(&engine, 0), 22, "lifelink");
}
