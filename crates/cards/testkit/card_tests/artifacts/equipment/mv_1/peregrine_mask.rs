//! `cards/artifacts/equipment/mv_1/peregrine_mask.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn peregrine_mask() -> CardIndex {
    card_index("a742308c-4047-487f-b95b-0fa5bec2f09a")
}

/// Peregrine Mask — "Equipped creature has defender, flying, and first
/// strike." and "Equip {2}". The equipped Elf has all three and, for the
/// defender, is no longer offered as an attacker; the bare Elf still is.
#[test]
fn peregrine_mask_grants_defender_flying_and_first_strike() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[peregrine_mask(), elf, elf], &[]);
    let p0 = PlayerId::new(0);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    equip(&mut engine, peregrine_mask(), 2, host);
    let granted = keywords(&engine, host);
    assert!(granted.contains(KeywordSet::DEFENDER));
    assert!(granted.contains(KeywordSet::FLYING));
    assert!(granted.contains(KeywordSet::FIRST_STRIKE));
    let bare = keywords(&engine, other);
    assert!(!bare.contains(KeywordSet::DEFENDER));
    assert!(!bare.contains(KeywordSet::FLYING));
    assert!(!bare.contains(KeywordSet::FIRST_STRIKE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(!attackers.contains(&host), "defender cannot attack");
    assert!(attackers.contains(&other), "the bare Elf can");
}
