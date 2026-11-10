//! `cards/enchantments/auras/mv_2/giant_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Giant Strength — {R}{R} — Aura. It prints exactly two sentences: "Enchant
/// creature" and "Enchanted creature gets +2/+2", and the static is written as
/// `Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])`, so the
/// reading worth playing is the one that tells the creature the Aura *holds*
/// from every other creature on the table. The second Elf under the same seat
/// is the load-bearing bystander: "+2/+2" landing on it too would mean the
/// filter had collapsed to "creatures you control", and the Elf across the
/// table would catch "+2/+2 on each creature". The offer is read before the
/// answer because "enchant creature" names no controller, and a copy of the
/// card that could only point at its caster's own board would print the same
/// text.
#[test]
fn giant_strength_pumps_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[giant_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {R}{R} off the two Mountains, and the offer is read before the answer:
    // targeting happens at CR 601.2c, the costs at CR 601.2h.
    cast_from_hand(&mut engine, p0, giant_strength());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what its Aura enchants");
    assert_eq!((min, max), (1, 1), "an Aura enchants exactly one creature");
    assert!(
        player_options.is_empty(),
        "\"creature\" is no player (CR 115.1): {player_options:?}"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the offer named is a legal target");
    pass_until(&mut engine, stack_is_empty);

    let aura =
        on_battlefield(&engine, p0, giant_strength()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the creature it was cast on"
    );
    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "\"enchanted creature gets +2/+2\""
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as — \
         \"enchanted creature\" is not \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
}
