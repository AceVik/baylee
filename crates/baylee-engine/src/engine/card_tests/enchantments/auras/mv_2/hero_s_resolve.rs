//! `cards/enchantments/auras/mv_2/hero_s_resolve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hero's Resolve ({1}{W}, Aura): "Enchant creature" and "Enchanted creature
/// gets +1/+5." Both printed lines are one scenario, because the second is a
/// `Filter::AttachedToBySource` static and only the creature the Aura *lands
/// on* may move. The Elf across the table is the control that separates a pump
/// reaching its host from one reaching every creature, and the target offer is
/// where "enchant creature" — any creature, not "you control" — is read.
/// +1/+5 on a printed 1/1 is `(2, 6)`: a `(1, 6)` would mean the power was
/// never added, a `(2, 2)` that the power was written where the toughness goes.
#[test]
fn hero_s_resolve_pumps_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(81, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[hero_s_resolve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // Two Plains pay {1}{W}. The Elf beside them taps for mana like any other
    // source, which is exactly what `tap_all_mana` does — nothing here needs
    // it untapped afterwards, so the whole tap is one call.
    cast_from_hand(&mut engine, p0, hero_s_resolve());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf it was cast for was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, hero_s_resolve()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4)"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 6),
        "+1/+5 on the creature the Aura holds — not +5/+1, and not nothing"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature the Aura does not hold"
    );
}
