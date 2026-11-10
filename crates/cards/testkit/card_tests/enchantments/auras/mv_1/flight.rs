//! `cards/enchantments/auras/mv_1/flight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flight is `{U}` Aura — "Enchant creature / Enchanted creature has
/// flying", and both sentences only mean anything on a board with creatures
/// the Aura does *not* hold. The 1/1 a host is printed as carries no
/// evasion until Flight lands on it, while the Elf beside it and the Elf
/// across the table stay grounded — so a static that had lost its
/// `AttachedToBySource` filter would grant flying to the whole table and be
/// caught. The offer is read before the answer too: CR 601.2c picks the
/// target before CR 601.2h pays for it, and "enchant creature" is what puts
/// the opponent's Elf on that list rather than a "you control" that was
/// never printed.
#[test]
fn flight_enchants_only_the_creature_it_lands_on() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "a printed 1/1 with nothing on it flies nowhere yet"
    );

    cast_from_hand(&mut engine, p0, flight());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "enchant creature asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf it was aimed at was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, flight()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4f)"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the static reaches the enchanted creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FLYING),
        "the Aura grants the keyword, it does not keep it"
    );
}
