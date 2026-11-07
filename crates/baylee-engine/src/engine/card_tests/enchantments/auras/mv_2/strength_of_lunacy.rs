//! `cards/enchantments/auras/mv_2/strength_of_lunacy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn strength_of_lunacy() -> CardIndex {
    card_index("02a95ea7-cfb9-497c-a004-3e783856d79c")
}

/// "Enchant creature" accepts a creature on either side, and only the creature
/// actually attached to the resolved Aura receives the printed +2/+1.
#[test]
fn strength_of_lunacy_pumps_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp(), swamp(), grizzly_bears(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[strength_of_lunacy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = all_on_battlefield(&engine, p0, grizzly_bears());
    assert_eq!(bears.len(), 2, "one host and one unenchanted bystander");
    let (host, bystander) = (bears[0], bears[1]);
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("an opposing creature");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, strength_of_lunacy());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("enchant creature asks for a target: {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(player_options.is_empty(), "a creature is not a player");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "enchant creature is not restricted by controller: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the offered creature is a legal target");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the ordinary cast spends exactly {{1}}{{B}}"
    );
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, strength_of_lunacy())
        .expect("the Aura resolves onto the battlefield");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host)
    );
    assert_eq!(pt(&engine, host), (4, 3), "+2/+1 on a printed 2/2");
    assert_eq!(pt(&engine, bystander), (2, 2), "no controller-wide pump");
    assert_eq!(pt(&engine, theirs), (2, 2), "no battlefield-wide pump");
}

/// Protection from white excludes the enchanted creature from a white spell's
/// targets, while leaving another creature legal (CR 702.16b).
#[test]
fn strength_of_lunacy_protects_against_white_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                plains(),
                plains(),
                quiet_creature(),
                festering_goblin(),
            ],
        )
        .hand(0, &[strength_of_lunacy(), swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the protected creature");
    let bystander = on_battlefield(&engine, p0, festering_goblin()).expect("a legal bystander");

    tap_all_mana(&mut engine, p0);
    cast_on(&mut engine, strength_of_lunacy(), creature);
    cast_with_floating(&mut engine, p0, swords_to_plowshares());
    let offered = target_menu(&engine);
    assert!(offered.contains(&bystander), "the spell has a legal target");
    assert!(
        !offered.contains(&creature),
        "a white spell cannot target the protected creature: {offered:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![creature],
                    players: vec![],
                },
            )
            .is_err(),
        "naming the protected creature anyway is refused"
    );
}

/// Protection from white prevents enchantment by white Auras. Unholy Strength
/// is the control: being black, it must still be able to enchant the creature.
#[test]
fn strength_of_lunacy_keeps_white_auras_off_the_creature() {
    a_ward_keeps_auras_of_its_color_off(
        strength_of_lunacy(),
        holy_armor(),
        plains(),
        unholy_strength(),
    );
}

/// Protection from white prevents all damage from a white Savannah Lions,
/// with a lethal unprotected control and zero damage under the Aura.
#[test]
fn strength_of_lunacy_prevents_damage_from_white_sources() {
    a_ward_prevents_damage_from_its_color(strength_of_lunacy(), index::SAVANNAH_LIONS, false);
    a_ward_prevents_damage_from_its_color(strength_of_lunacy(), index::SAVANNAH_LIONS, true);
}

/// Protection from white means white creatures cannot block the enchanted
/// creature, checked against the same attack without the Aura.
#[test]
fn strength_of_lunacy_cannot_be_blocked_by_white_creatures() {
    a_warded_creature_slips_past_its_color(strength_of_lunacy(), index::SAVANNAH_LIONS, false);
    a_warded_creature_slips_past_its_color(strength_of_lunacy(), index::SAVANNAH_LIONS, true);
}

/// Madness {B} casts the exact discarded incarnation during the opponent's
/// turn. Prying Eyes supplies the discard; five Swamps and two Islands pay its
/// {4}{U}{U} while leaving exactly the one black mana needed for Madness.
#[test]
fn strength_of_lunacy_madness_enchants_a_creature_for_one_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[strength_of_lunacy(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the opposing host");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let aura = in_hand(&engine, p0, strength_of_lunacy()).expect("the card to discard");
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != aura)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![aura, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == aura
        )
    });
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Prying Eyes leaves the black mana for Madness"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let offered = aim_at(&mut engine, p0, bear);
    assert_eq!(offered, vec![bear], "Madness still uses the Aura target");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "Madness spends exactly {{B}}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "Madness ignored timing");
    let resolved = on_battlefield(&engine, p0, strength_of_lunacy())
        .expect("the discarded Aura was cast and resolved");
    assert_eq!(
        engine.state().object(resolved).and_then(|o| o.attached_to),
        Some(bear)
    );
    assert_eq!(pt(&engine, bear), (4, 3));
    assert!(mine(&engine, p0, strength_of_lunacy(), Zone::Exile).is_empty());
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
}
