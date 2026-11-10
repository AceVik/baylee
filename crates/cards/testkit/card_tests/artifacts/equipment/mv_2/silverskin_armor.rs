//! `cards/artifacts/equipment/mv_2/silverskin_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn silverskin_armor() -> CardIndex {
    card_index("4eb24ae3-da1a-4565-934f-e25f56036d71")
}

/// Silverskin Armor: "Equipped creature gets +1/+1 and is an artifact in
/// addition to its other types. Equip {2}".
#[test]
fn silverskin_armor_makes_the_host_an_artifact_creature() {
    let mut r = rig(silverskin_armor(), llanowar_elves(), 2);
    assert!(!types(&r.engine, r.host).contains(TypeSet::ARTIFACT));
    r.equip(2);
    assert_eq!(pt(&r.engine, r.host), (2, 2), "+1/+1");
    let t = types(&r.engine, r.host);
    assert!(t.contains(TypeSet::ARTIFACT), "an artifact: {t:?}");
    assert!(
        t.contains(TypeSet::CREATURE),
        "in addition to being a creature: {t:?}"
    );
    assert!(
        !types(&r.engine, r.bystander).contains(TypeSet::ARTIFACT),
        "the unequipped creature is no artifact"
    );
    assert_eq!(pt(&r.engine, r.bystander), (2, 1));
}
