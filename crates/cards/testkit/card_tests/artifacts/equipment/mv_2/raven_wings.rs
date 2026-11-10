//! `cards/artifacts/equipment/mv_2/raven_wings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn raven_wings() -> CardIndex {
    card_index("2d668f67-7cde-447a-8eb3-35c0c5c5ab69")
}

/// Raven Wings: "Equipped creature gets +1/+0, has flying, and is a Bird in
/// addition to its other types. Equip {2}".
#[test]
fn raven_wings_makes_the_host_a_flying_bird_and_keeps_its_types() {
    let mut r = rig(raven_wings(), llanowar_elves(), 2);
    let bird = baylee_core::generated::subtypes::creature::BIRD;
    assert!(!subtypes_of(&r.engine, r.host).contains(bird));
    r.equip(2);
    assert_eq!(pt(&r.engine, r.host), (2, 1), "+1/+0");
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::FLYING));
    let subtypes = subtypes_of(&r.engine, r.host);
    assert!(subtypes.contains(bird), "a Bird");
    assert!(
        subtypes.contains(baylee_core::generated::subtypes::creature::ELF),
        "and still an Elf"
    );
    assert!(!subtypes_of(&r.engine, r.bystander).contains(bird));
}
