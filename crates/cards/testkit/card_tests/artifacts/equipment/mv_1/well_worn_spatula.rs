//! `cards/artifacts/equipment/mv_1/well_worn_spatula.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn well_worn_spatula() -> CardIndex {
    card_index("3cc333a1-c854-4b7c-8002-910e48222371")
}

/// Well-Worn Spatula — "When this Equipment enters, you gain 2 life.
/// Equipped creature gets +1/+1." and "Equip {1}". Cast for {1}: 22 life,
/// then the Elf wearing it is a 2/2.
#[test]
fn well_worn_spatula_gains_two_life_when_it_enters() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[well_worn_spatula()], &[elf], &[]);
    let p0 = PlayerId::new(0);
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    assert_eq!(engine.state().players[0].life, 20);

    float(&mut engine, 1);
    cast_with_floating(&mut engine, p0, well_worn_spatula());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, well_worn_spatula()).is_some());
    assert_eq!(engine.state().players[0].life, 22);

    equip(&mut engine, well_worn_spatula(), 1, host);
    assert_eq!(pt(&engine, host), (2, 2));
}
