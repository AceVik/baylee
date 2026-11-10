//! `cards/enchantments/auras/mv_1/unholy_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unholy Strength — {B} Aura: "Enchant creature" and "Enchanted creature
/// gets +2/+1".
///
/// Both halves are read off one play. "Enchant creature" is a target on *any*
/// creature in the game, so the Elf across the table is offered and then has
/// to come out unchanged — that is the counter-half of "enchanted creature".
/// The static is keyed on `Filter::AttachedToBySource`, so only the creature
/// the Aura is actually attached to collects the numbers: `(3, 2)` on a
/// printed 1/1 is the only pair that reads the `+2` and the `+1` both, and
/// the Aura itself must stay a printed enchantment instead of growing a body.
#[test]
fn unholy_strength_gives_two_and_one_to_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[unholy_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one it may not touch"
    );

    // {B} off the Swamp alone: the Elf is kept back, because it is the
    // creature this Aura is about to enchant and a host tapped for mana is
    // still a host, but an untapped one keeps the reading unambiguous.
    tap_mana_except(&mut engine, p0, host);
    cast_with_floating(&mut engine, p0, unholy_strength());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura's enchant ability is a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"Enchant creature\" is any creature on the table, either side: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the spell targeted is one of its own options");
    pass_until(&mut engine, stack_is_empty);

    let aura =
        on_battlefield(&engine, p0, unholy_strength()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4a)"
    );
    assert_eq!(
        pt(&engine, host),
        (3, 2),
        "+2/+1 on the creature the Aura is attached to"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
    let kinds = types(&engine, aura);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "the Aura grants the numbers, it does not keep them: {kinds:?}"
    );
}
