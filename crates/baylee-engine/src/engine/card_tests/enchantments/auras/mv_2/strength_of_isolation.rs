//! `cards/enchantments/auras/mv_2/strength_of_isolation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn strength_of_isolation() -> CardIndex {
    card_index("87aad9fb-e234-4171-a4ae-c686f613d37e")
}

/// "Enchant creature" accepts a creature on either side, and only the creature
/// actually attached to the resolved Aura receives the printed +1/+2.
#[test]
fn strength_of_isolation_pumps_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), grizzly_bears(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[strength_of_isolation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = all_on_battlefield(&engine, p0, grizzly_bears());
    assert_eq!(bears.len(), 2, "one host and one unenchanted bystander");
    let (host, bystander) = (bears[0], bears[1]);
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("an opposing creature");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, strength_of_isolation());
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
        "the ordinary cast spends exactly {{1}}{{W}}"
    );
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, strength_of_isolation())
        .expect("the Aura resolves onto the battlefield");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host)
    );
    assert_eq!(pt(&engine, host), (3, 4), "+1/+2 on a printed 2/2");
    assert_eq!(pt(&engine, bystander), (2, 2), "no controller-wide pump");
    assert_eq!(pt(&engine, theirs), (2, 2), "no battlefield-wide pump");
}

/// Protection from black excludes the enchanted creature from a black spell's
/// targets, while leaving another creature legal (CR 702.16b).
#[test]
fn strength_of_isolation_protects_against_black_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
                festering_goblin(),
            ],
        )
        .hand(0, &[strength_of_isolation(), hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the protected creature");
    let bystander = on_battlefield(&engine, p0, festering_goblin()).expect("a legal bystander");

    tap_all_mana(&mut engine, p0);
    cast_on(&mut engine, strength_of_isolation(), creature);
    cast_with_floating(&mut engine, p0, hero_s_downfall());
    let offered = target_menu(&engine);
    assert!(offered.contains(&bystander), "the spell has a legal target");
    assert!(
        !offered.contains(&creature),
        "a black spell cannot target the protected creature: {offered:?}"
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

/// Protection from black also prevents enchantment by black Auras.
#[test]
fn strength_of_isolation_keeps_black_auras_off_the_creature() {
    a_ward_keeps_auras_of_its_color_off(
        strength_of_isolation(),
        unholy_strength(),
        swamp(),
        holy_armor(),
    );
}

/// Protection from black prevents damage from black sources, with a lethal
/// unprotected control and a surviving protected creature.
#[test]
fn strength_of_isolation_prevents_damage_from_black_sources() {
    a_ward_prevents_damage_from_its_color(strength_of_isolation(), bile_urchin(), false);
    a_ward_prevents_damage_from_its_color(strength_of_isolation(), bile_urchin(), true);
}

/// Protection from black means black creatures cannot block the enchanted
/// creature, again checked against the same attack without the Aura.
#[test]
fn strength_of_isolation_cannot_be_blocked_by_black_creatures() {
    a_warded_creature_slips_past_its_color(strength_of_isolation(), bile_urchin(), false);
    a_warded_creature_slips_past_its_color(strength_of_isolation(), bile_urchin(), true);
}

/// Madness {W} casts the exact discarded incarnation during the opponent's
/// turn. Prying Eyes supplies the discard; five Plains and two Islands pay its
/// {4}{U}{U} while leaving exactly the one white mana needed for Madness.
#[test]
fn strength_of_isolation_madness_enchants_a_creature_for_one_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[strength_of_isolation(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
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
    let aura = in_hand(&engine, p0, strength_of_isolation()).expect("the card to discard");
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
            .available(ManaColor::White),
        1,
        "Prying Eyes leaves the white mana for Madness"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let offered = aim_at(&mut engine, p0, bear);
    assert_eq!(offered, vec![bear], "Madness still uses the Aura target");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "Madness spends exactly {{W}}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "Madness ignored timing");
    let resolved = on_battlefield(&engine, p0, strength_of_isolation())
        .expect("the discarded Aura was cast and resolved");
    assert_eq!(
        engine.state().object(resolved).and_then(|o| o.attached_to),
        Some(bear)
    );
    assert_eq!(pt(&engine, bear), (3, 4));
    assert!(mine(&engine, p0, strength_of_isolation(), Zone::Exile).is_empty());
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
}
