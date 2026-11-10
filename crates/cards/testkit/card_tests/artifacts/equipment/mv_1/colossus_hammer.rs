//! `cards/artifacts/equipment/mv_1/colossus_hammer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn colossus_hammer() -> CardIndex {
    card_index("8ec03b88-8d3a-4a32-8b7c-7da59b0c03d0")
}

/// Colossus Hammer — "Equipped creature gets +10/+10 and loses flying." and
/// "Equip {8}". The Angel is the flyer: it ends up ten bigger and on the
/// ground, while the Elf (no flying to lose) just grows.
#[test]
fn colossus_hammer_grows_its_host_and_grounds_it() {
    let (elf, angel) = (llanowar_elves(), angel_of_mercy());
    let mut engine = rig(16, &[], &[colossus_hammer(), elf, angel], &[]);
    let p0 = PlayerId::new(0);
    let elf = on_battlefield(&engine, p0, elf).expect("the Elf");
    let angel = on_battlefield(&engine, p0, angel).expect("the Angel");
    let (ap, at) = pt(&engine, angel);
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));

    equip(&mut engine, colossus_hammer(), 8, angel);
    assert_eq!(pt(&engine, angel), (ap + 10, at + 10));
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, elf), (1, 1), "the bystander is untouched");

    equip(&mut engine, colossus_hammer(), 8, elf);
    assert_eq!(pt(&engine, elf), (11, 11));
    assert_eq!(pt(&engine, angel), (ap, at), "released: back to the base");
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "and flying again"
    );
}
