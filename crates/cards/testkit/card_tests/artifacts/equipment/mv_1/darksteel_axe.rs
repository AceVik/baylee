//! `cards/artifacts/equipment/mv_1/darksteel_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darksteel_axe() -> CardIndex {
    card_index("3b7ea3ac-ac0a-40aa-b743-d153e1c47d8c")
}

/// Darksteel Axe — "Equipped creature gets +2/+0." and "Equip {2}".
#[test]
fn darksteel_axe_gives_plus_two_power_for_equip_two() {
    plain_pump(darksteel_axe(), 2, 2, 0);
}

/// "Indestructible": a destroy effect leaves the Axe on the battlefield and
/// on its host, while the same effect kills the unprotected Elf beside it.
#[test]
fn darksteel_axe_survives_destruction() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[darksteel_axe(), elf, elf], &[]);
    let p0 = PlayerId::new(0);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    let axe = equip(&mut engine, darksteel_axe(), 2, host);
    assert!(keywords(&engine, axe).contains(KeywordSet::INDESTRUCTIBLE));

    kill(&mut engine, axe);
    let state = engine.state();
    assert!(
        state
            .object(axe)
            .is_some_and(|o| o.attached_to == Some(host)),
        "the Axe is still out and still on its host"
    );
    kill(&mut engine, other);
    assert!(
        on_battlefield(&engine, p0, darksteel_axe()).is_some(),
        "the Axe stays"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, elf).len(),
        1,
        "while the unprotected Elf died to the same effect"
    );
}
