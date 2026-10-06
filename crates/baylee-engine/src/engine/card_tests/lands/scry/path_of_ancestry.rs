//! `cards/lands/scry/path_of_ancestry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Path of Ancestry` is a utility land under `Coverage::Implemented`.
/// It prints "This land enters tapped." and "{T}: Add one mana of any color in your commander's
/// color identity. When that mana is spent to cast a creature spell that shares a creature type
/// with your commander, scry 1."
/// When played with a mono-blue commander like `jin_gitaxias()`, it enters tapped, untaps on the
/// next turn, and makes ordinary blue mana: the rider restricts nothing (#232), so the unit is in
/// the plain counters and carries its rider beside them.
#[test]
fn path_of_ancestry_enters_tapped_and_makes_ordinary_commander_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[jin_gitaxias()])
        .hand(0, &[path_of_ancestry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, path_of_ancestry());
    assert!(is_tapped(&engine, land), "Path of Ancestry enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, path_of_ancestry(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one ordinary blue mana for the commander's identity"
    );
    assert!(pool.restricted().is_empty(), "nothing restricts it");
    assert_eq!(pool.ridden().len(), 1, "the unit carries the scry");
    assert!(is_tapped(&engine, land), "Path of Ancestry is now tapped");
}

/// Path of Ancestry's mana pays for a spell its rider does not name, and no
/// scry goes off (#232). Read as "Spend this mana only", it paid for
/// nothing but the creature spells the rider names.
#[test]
fn path_of_ancestry_pays_for_a_spell_its_rider_does_not_name() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[venser_shaper_savant()])
        .battlefield(0, &[path_of_ancestry()])
        .hand(0, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, path_of_ancestry(), 0);
    cast_with_floating(&mut engine, p0, brainstorm());
    let stack = engine.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 1, "Brainstorm and no scry: {stack:?}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "Path's blue paid for it"
    );
}

/// Path of Ancestry's rider goes off for a creature spell that shares a
/// creature type with the commander, once for each unit spent on it
/// (CR 106.6a): Venser and Snapcaster Mage are both Human Wizards, and two
/// Paths paying for the Mage scry twice.
#[test]
fn path_of_ancestry_scries_for_each_unit_spent_on_a_creature_it_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[venser_shaper_savant()])
        .battlefield(0, &[path_of_ancestry(), path_of_ancestry()])
        .hand(0, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, path_of_ancestry(), 0);
    activate(&mut engine, p0, path_of_ancestry(), 0);
    cast_with_floating(&mut engine, p0, snapcaster_mage());
    let mage = on_stack(&engine, snapcaster_mage()).expect("the Mage is cast");
    let scries = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| loc.source == mage)
        })
        .count();
    assert_eq!(scries, 2, "one scry for each of Path's units");
}
