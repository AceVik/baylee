//! `cards/enchantments/auras/mv_1/holy_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Holy Strength is a {W} Aura printing "Enchant creature" and "Enchanted
/// creature gets +1/+2", and only the second line is a static — a
/// `Filter::AttachedToBySource`, so the one reading worth playing is the one
/// that tells the creature the Aura *holds* from every other creature on the
/// table. A printed 1/1 Elf becomes a 2/3 while an Elf of mine nobody
/// enchanted and an Elf across the table both stay 1/1, which is why the
/// board carries all three. The target menu is read as well: "Enchant
/// creature" reaches either side of the table and never a land, so its own
/// options are where a filter that had quietly narrowed to "you control" or
/// widened to `Filter::Any` would show up.
#[test]
fn holy_strength_pumps_the_creature_it_enchant_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[holy_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, plains()).expect("a Plains is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    cast_from_hand(&mut engine, p0, holy_strength());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Plains is no creature and cannot be enchanted: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, holy_strength()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "the Aura is attached to the creature it named"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature it enchants"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
}
