#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::NATURE_S_CLAIM;

fn subject() -> CardIndex {
    card_index("0e0bef57-d977-48d1-be78-9a4f0aacd9c0")
}

// Scenario: flying is unconditional; own artifact turns +1/+0 on, while an
// opponent artifact alone does not. Toughness remains one throughout.
#[test]
fn foundry_screecher_needs_our_artifact_for_its_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .hand(0, &[subject(), quiet_artifact()])
        .hand(1, &[NATURE_S_CLAIM])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let bat = on_battlefield(&e, p0, subject()).unwrap();
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());
    assert_eq!(pt(&e, bat), (2, 1));
    assert!(keywords(&e, bat).contains(KeywordSet::FLYING));
    cast_with_floating(&mut e, p0, quiet_artifact());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bat), (3, 1));
    assert!(keywords(&e, bat).contains(KeywordSet::FLYING));
    let artifact = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, NATURE_S_CLAIM);
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![artifact],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bat), (2, 1));
    assert!(keywords(&e, bat).contains(KeywordSet::FLYING));
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());
    assert!(in_graveyard(&e, p0, quiet_artifact()).is_some());
}
