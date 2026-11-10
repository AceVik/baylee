//! `cards/enchantments/auras/mv_2/alexi_s_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alexi's Cloak is `{1}{U}` Aura with flash and one printed static:
/// "Enchanted creature has shroud." The static is `Filter::AttachedToBySource`,
/// so the only reading worth playing is the one that tells the creature the Aura
/// *holds* from every other creature in the game — which is why a bare Elf beside
/// the host is read, and why the opponent's own removal spell is what says the
/// shroud is real rather than decorative: Swords to Plowshares offers the bare Elf
/// and refuses the enchanted one in the very same question, so a filter that had
/// simply dropped the keyword would fail one half or the other.
#[test]
fn alexis_cloak_wards_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[alexi_s_cloak()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "nothing is enchanted yet"
    );

    // {1}{U} off the two Islands, and "enchant creature" is a target choice
    // (CR 601.2c) answered before the cost is read out of the pool.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, alexi_s_cloak());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "target creature reaches either Elf on this board: {options:?}"
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

    let cloak = on_battlefield(&engine, p0, alexi_s_cloak()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(cloak).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it enchanted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"enchanted creature has shroud\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "the static reaches the enchanted creature and no other"
    );

    // The other side of the table, which is where "can't be the target of
    // spells" is visible at all: the opponent's own removal spell asks once
    // and has to name the bare Elf and not the cloaked one.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bystander),
        "the control: the Elf the Cloak does not hold is still a legal target: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "shroud: the enchanted creature cannot be the target of a spell at all: {options:?}"
    );
}
