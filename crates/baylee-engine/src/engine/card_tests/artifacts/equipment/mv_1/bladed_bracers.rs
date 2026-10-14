//! `cards/artifacts/equipment/mv_1/bladed_bracers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bladed_bracers() -> CardIndex {
    card_index("24d373cd-245a-4051-b187-ba8cc83b0c48")
}

/// Bladed Bracers — "Equipped creature gets +1/+1. As long as equipped
/// creature is a Human or an Angel, it has vigilance." and "Equip {2}". An
/// Elf gets the +1/+1 and no vigilance; the Human and the Angel get both.
#[test]
fn bladed_bracers_grant_vigilance_to_humans_and_angels_only() {
    let (poet, elf, angel) = (tragic_poet(), llanowar_elves(), angel_of_mercy());
    let mut engine = rig(6, &[], &[bladed_bracers(), poet, elf, angel], &[]);
    let p0 = PlayerId::new(0);
    let human = on_battlefield(&engine, p0, poet).expect("the Human");
    let elf = on_battlefield(&engine, p0, elf).expect("the Elf");
    let angel = on_battlefield(&engine, p0, angel).expect("the Angel");
    let angel_base = pt(&engine, angel);
    assert!(!keywords(&engine, angel).contains(KeywordSet::VIGILANCE));

    equip(&mut engine, bladed_bracers(), 2, elf);
    assert_eq!(pt(&engine, elf), (2, 2));
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::VIGILANCE),
        "an Elf is neither"
    );

    equip(&mut engine, bladed_bracers(), 2, human);
    assert_eq!(pt(&engine, human), (2, 2));
    assert!(keywords(&engine, human).contains(KeywordSet::VIGILANCE));
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::VIGILANCE),
        "the Elf let go"
    );
    assert_eq!(pt(&engine, elf), (1, 1));

    equip(&mut engine, bladed_bracers(), 2, angel);
    assert_eq!(pt(&engine, angel), (angel_base.0 + 1, angel_base.1 + 1));
    assert!(keywords(&engine, angel).contains(KeywordSet::VIGILANCE));
    assert!(
        !keywords(&engine, human).contains(KeywordSet::VIGILANCE),
        "the Human let go"
    );
}
