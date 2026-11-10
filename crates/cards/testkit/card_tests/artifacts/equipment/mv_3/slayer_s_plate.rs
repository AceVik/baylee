//! `cards/artifacts/equipment/mv_3/slayer_s_plate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn slayer_s_plate() -> CardIndex {
    card_index("05004b4a-2e2c-4a88-a994-8576684fb64a")
}

/// Slayer's Plate: "Equipped creature gets +4/+2. Whenever equipped
/// creature dies, if it was a Human, create a 1/1 white Spirit creature token
/// with flying. Equip {3}".
#[test]
fn slayer_s_plate_makes_a_spirit_when_an_equipped_human_dies() {
    let p0 = PlayerId::new(0);
    let mut r = rig(slayer_s_plate(), jasmine_boreal(), 3);
    r.equip(3);
    assert_eq!(pt(&r.engine, r.host), (8, 7), "+4/+2");
    assert!(tokens_of(&r.engine, p0).is_empty());
    let host = r.host;
    kill(&mut r.engine, host);
    let tokens = tokens_of(&r.engine, p0);
    assert_eq!(tokens.len(), 1, "one Spirit");
    assert_eq!(pt(&r.engine, tokens[0]), (1, 1));
    assert!(keywords(&r.engine, tokens[0]).contains(KeywordSet::FLYING));
    assert!(
        subtypes_of(&r.engine, tokens[0])
            .contains(baylee_core::generated::subtypes::creature::SPIRIT)
    );
}

#[test]
fn slayer_s_plate_makes_nothing_when_a_nonhuman_dies() {
    let p0 = PlayerId::new(0);
    let mut r = rig(slayer_s_plate(), llanowar_elves(), 3);
    r.equip(3);
    assert_eq!(pt(&r.engine, r.host), (5, 3), "+4/+2");
    let host = r.host;
    kill(&mut r.engine, host);
    assert!(
        on_battlefield(&r.engine, p0, llanowar_elves()).is_none(),
        "the Elf died"
    );
    assert!(tokens_of(&r.engine, p0).is_empty(), "\"if it was a Human\"");
}
