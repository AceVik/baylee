//! `cards/enchantments/auras/mv_1/robe_of_mirrors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Robe of Mirrors — {U}, Aura: "Enchant creature" and "Enchanted creature
/// has shroud."
///
/// The Aura has to arrive by being *cast*, because the creature it enchants
/// is chosen while the spell is on the stack and the attach is the resolving
/// effect's whole job — a Robe seeded onto a board would enter attached to
/// nobody and its keyword would land nowhere. Shroud is nothing but a
/// targeting restriction (CR 702.18a), so the second printed sentence is read
/// off the opponent's Swords to Plowshares: the menu holds the bare Elf under
/// the same controller and the Elf across the table — two live options — and
/// not the robed one, which is what keeps the absence from being an empty
/// menu, and then the bait Elf really is exiled while its neighbour stands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn robe_of_mirrors_shrouds_only_the_creature_it_is_attached_to() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[robe_of_mirrors()])
        .battlefield(1, &[plains(), llanowar_elves()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "nothing is enchanted yet"
    );

    // {U} off the Island, and the enchanted creature is named while the Aura
    // is still a spell (CR 601.2c).
    tap_all_mana(&mut engine, p0);
    let robe = in_hand(&engine, p0, robe_of_mirrors()).expect("the Robe is in hand");
    cast_with_floating(&mut engine, p0, robe_of_mirrors());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster names what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either Elf you control may wear it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(robe)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "enchanted creature has shroud"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "the static reaches the creature the Robe is attached to and no other"
    );
    assert!(
        !keywords(&engine, robe).contains(KeywordSet::SHROUD),
        "the Aura grants the keyword, it does not keep it"
    );

    // The other side of the table, on its own turn: Swords to Plowshares can
    // point at any creature, and the offer is where shroud is visible.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it chooses");
    assert!(
        options.contains(&bystander),
        "the bare Elf under the same controller is still a legal target: {options:?}"
    );
    assert!(
        options.contains(&their_elf),
        "and so is a creature of the caster's own: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "\"enchanted creature has shroud\": the creature the Robe holds cannot \
         be targeted at all (CR 702.18a): {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the bait Elf was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&host),
        "the shrouded Elf was never a legal target and is untouched"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&bystander),
        "and the Elf that *was* offered is exiled, so the difference between \
         the two was shroud and not a spell that never resolved"
    );
}
