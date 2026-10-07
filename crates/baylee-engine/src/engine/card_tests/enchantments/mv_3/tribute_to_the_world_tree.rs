//! `cards/enchantments/mv_3/tribute_to_the_world_tree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tribute to the World Tree` is an enchantment costing `{G}{G}{G}` under `Coverage::Implemented`.
/// It prints "Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it."
/// When a 1/1 creature like `llanowar_elves()` enters, its power is below 3,
/// so it receives two `CounterKind::P1P1` counters and grows to a 3/3 without drawing a card.
#[test]
fn tribute_to_the_world_tree_adds_counters_to_small_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tribute_to_the_world_tree(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves entered the battlefield");
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        2,
        "gained two +1/+1 counters"
    );
    assert_eq!(pt(&engine, elf), (3, 3), "power and toughness are 3/3");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "no card was drawn because entering power was less than 3"
    );
}
