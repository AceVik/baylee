//! `cards/artifacts/equipment/mv_1/vibranium_energy_daggers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vibranium_energy_daggers() -> CardIndex {
    card_index("be0b86fc-8450-41d4-9836-6de11cecb857")
}

/// Vibranium Energy Daggers — "Equipped creature gets +2/+2." and "Equip {3}".
#[test]
fn vibranium_energy_daggers_give_plus_two_plus_two_for_equip_three() {
    plain_pump(vibranium_energy_daggers(), 3, 2, 2);
}

/// "Indestructible": a destroy effect leaves the Daggers on the battlefield
/// and on their host, while the same effect kills the Elf beside it.
#[test]
fn vibranium_energy_daggers_survive_destruction() {
    let elf = llanowar_elves();
    let mut engine = rig(3, &[], &[vibranium_energy_daggers(), elf, elf], &[]);
    let p0 = PlayerId::new(0);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    let daggers = equip(&mut engine, vibranium_energy_daggers(), 3, host);
    assert!(keywords(&engine, daggers).contains(KeywordSet::INDESTRUCTIBLE));

    kill(&mut engine, daggers);
    assert!(
        engine
            .state()
            .object(daggers)
            .is_some_and(|o| o.attached_to == Some(host)),
        "the Daggers are still out and still on their host"
    );
    kill(&mut engine, other);
    assert_eq!(
        all_on_battlefield(&engine, p0, elf).len(),
        1,
        "while the unprotected Elf died to the same effect"
    );
}
