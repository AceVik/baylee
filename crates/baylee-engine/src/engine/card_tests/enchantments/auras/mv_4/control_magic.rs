//! `cards/enchantments/auras/mv_4/control_magic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Control Magic: "Enchant creature" / "You control enchanted creature."
/// Control moves to the caster while attached and reverts once the Aura is
/// gone (a layer-2 effect over its source, `Modifier::GainControl`).
#[test]
fn control_magic_takes_the_creature_and_gives_it_back_when_it_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let control_magic = control_magic();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[control_magic])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("their creature");

    cast_from_hand(&mut engine, p0, control_magic);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    let controller = |e: &Engine<RegistryLookup>| e.state().object(creature).unwrap().controller;
    assert_eq!(controller(&engine), p0, "you control enchanted creature");

    let aura = on_battlefield(&engine, p0, control_magic).expect("Control Magic is attached");
    kill(&mut engine, aura);
    assert_eq!(
        controller(&engine),
        p1,
        "with the Aura gone, the creature goes home"
    );
}
