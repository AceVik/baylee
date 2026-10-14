//! `cards/artifacts/equipment/mv_1/hedgewitch_s_mask.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hedgewitch_s_mask() -> CardIndex {
    card_index("c74f6a83-a280-450e-969d-6b3fa28c8744")
}

fn can_block(blocks: &[crate::choice::BlockOption], blocker: ObjectId, attacker: ObjectId) -> bool {
    blocks
        .iter()
        .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
}

/// Hedgewitch's Mask — "Equipped creature gets +1/+1. Equipped creature
/// can't be blocked by creatures with power 4 or greater." and "Equip {2}".
/// A 4/4 Serra Angel and a 3/3 Angel of Mercy face an Elf: unequipped, both
/// may block it; with the Mask (a 2/2), only the 3/3 may.
#[test]
fn hedgewitch_s_mask_keeps_big_creatures_from_blocking() {
    let (elf, serra, mercy) = (llanowar_elves(), serra_angel(), angel_of_mercy());
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Control: no Mask, no restriction.
    let mut bare = rig(0, &[], &[elf], &[serra, mercy]);
    let attacker = on_battlefield(&bare, p0, elf).expect("the Elf");
    let big = on_battlefield(&bare, p1, serra).expect("Serra");
    let blocks = attack_and_collect_blocks(&mut bare, attacker, p1);
    assert!(
        can_block(&blocks, big, attacker),
        "Serra may block a bare Elf"
    );

    let mut engine = rig(2, &[], &[hedgewitch_s_mask(), elf], &[serra, mercy]);
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    let big = on_battlefield(&engine, p1, serra).expect("Serra");
    let small = on_battlefield(&engine, p1, mercy).expect("Angel of Mercy");
    equip(&mut engine, hedgewitch_s_mask(), 2, host);
    assert_eq!(pt(&engine, host), (2, 2));
    let blocks = attack_and_collect_blocks(&mut engine, host, p1);
    assert!(can_block(&blocks, small, host), "power 3 may block");
    assert!(!can_block(&blocks, big, host), "power 4 may not");
}
