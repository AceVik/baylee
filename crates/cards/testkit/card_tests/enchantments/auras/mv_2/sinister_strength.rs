//! `cards/enchantments/auras/mv_2/sinister_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sinister Strength — {1}{B} Aura: "Enchant creature. Enchanted creature
/// gets +3/+1 and is black."
///
/// The two clauses of the second sentence live in two different layers, so
/// each gets its own bystander. One of two printed 1/1 Elves becomes a 4/2
/// while the other stays a 1/1 and the Elf across the table stays a 1/1 too,
/// which is what says the static reads `AttachedToBySource` rather than
/// "creatures you control" or the whole table; and the host reads black while
/// the bare Elf beside it and the one across the table stay green, which is
/// the colour half and not a side effect of the pump. The target menu is read
/// before the answer, because the printed word is "creature" and not
/// "creature you control": the opponent's Elf is offered.
#[test]
fn sinister_strength_pumps_and_blackens_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[swamp(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sinister_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {1}{B}: the Swamp pays the black, the Forest the generic.
    cast_from_hand(&mut engine, p0, sinister_strength());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what to enchant");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under your own control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"Enchant creature\" is any creature and not only yours: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the Aura's own question offered");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, sinister_strength()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature its target became"
    );

    assert_eq!(
        pt(&engine, host),
        (4, 2),
        "+3/+1 on the creature it is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and nothing at all on the Elf beside it"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor on the one across the table"
    );

    // The layer that no power/toughness reading can see.
    let host_colors = engine
        .state()
        .object(host)
        .expect("the host is on the battlefield")
        .characteristics()
        .colors;
    assert!(
        host_colors.contains(baylee_core::color::Color::Black),
        "\"and is black\" — the enchanted creature takes the colour"
    );
    for (id, what) in [
        (bystander, "the bare Elf beside it"),
        (theirs, "the Elf across the table"),
    ] {
        let colors = engine
            .state()
            .object(id)
            .expect("the bystander is on the battlefield")
            .characteristics()
            .colors;
        assert!(
            !colors.contains(baylee_core::color::Color::Black),
            "{what} is not the enchanted creature and stays the green 1/1 it was printed as"
        );
    }
}
