//! `cards/enchantments/auras/mv_2/creature_bond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Creature Bond targets a creature, never Sol Ring, and attaches to the Elves.
#[test]
fn creature_bond_attaches_only_to_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), llanowar_elves(), sol_ring()])
        .hand(0, &[creature_bond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    let rock = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    attaches_only_to(&mut engine, p0, creature_bond(), elf, rock);
}
