//! Oracle: Other creatures you control get +1/+1.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("2cc439e8-d112-44e6-bc5a-6e99333c519a")
}

// Checklist: play source; own recipient, self inclusion/exclusion, opponent and
// nonmatching bystanders; exact stats/keyword; real bounce ends continuous grant.
#[test]
fn grant_obeys_recipients_and_ends_when_source_leaves() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::GRAY_OGRE,
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE, index::HILL_GIANT])
        .hand(0, &[subject(), unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let ally = on_battlefield(&e, p0, index::GRAY_OGRE).unwrap();
    let foe = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    assert_eq!(pt(&e, ally), (2, 2));
    let borrowed = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(borrowed)
        .unwrap()
        .set_controller(p0);
    e.refresh_offer();
    assert_eq!(e.state().object(borrowed).unwrap().owner, p1);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let source = on_battlefield(&e, p0, subject()).unwrap();
    assert_eq!(
        pt(&e, on_battlefield(&e, p1, index::HILL_GIANT).unwrap()),
        (3, 3)
    );
    assert_eq!(pt(&e, source), (3, 3));
    assert_eq!(pt(&e, ally), (3, 3));
    assert_eq!(pt(&e, foe), (3, 3)); // controller, not owner, receives anthem
    cast_with_floating(&mut e, p0, unsummon());
    aim_at(&mut e, p0, source);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, subject()).is_some());
    assert_eq!(pt(&e, ally), (2, 2));
    assert_eq!(pt(&e, borrowed), (2, 2));
}
