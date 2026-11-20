//! `cards/creatures/mv_2/mistform_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_sliver() -> CardIndex {
    card_index("d29f59a4-b956-40e0-acd4-c2560e538baf")
}

/// Mistform Sliver grants `{1}: This permanent becomes the creature type of
/// your choice in addition to its other types until end of turn` to all Slivers
/// (both players' Slivers, not non-Slivers), and preserves existing subtypes.
#[test]
fn mistform_sliver_grants_additive_type_change_to_all_slivers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), mistform_sliver(), llanowar_elves()],
        )
        .battlefield(1, &[island(), muscle_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sliver0 = on_battlefield(&engine, p0, mistform_sliver()).expect("P0 Mistform Sliver");
    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let sliver1 = on_battlefield(&engine, p1, muscle_sliver()).expect("P1 Muscle Sliver");

    // Non-Sliver Llanowar Elves only has its own {T}: Add {G} mana ability, no Mistform {1} ability.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected P0 priority");
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == elf0),
        "non-Sliver Elf does not receive Mistform Sliver's granted ability"
    );
    let &(sliver_id, ability_index) = legal
        .abilities
        .iter()
        .find(|(id, _)| *id == sliver0)
        .expect("Mistform Sliver has the granted {1} ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sliver_id,
                ability_index,
            },
        )
        .expect("activate granted ability on Mistform Sliver");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(sliver0)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::DRAGON), "gained Dragon");
    assert!(
        subs.contains(subtypes::creature::ILLUSION) && subs.contains(subtypes::creature::SLIVER),
        "kept original Illusion and Sliver subtypes (in addition to)"
    );

    // Opponent's Muscle Sliver also received the granted ability and wears off Dragon on cleanup.
    reach_their_main_phase(&mut engine, p1);
    let subs_after_cleanup = &engine
        .state()
        .object(sliver0)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        !subs_after_cleanup.contains(subtypes::creature::DRAGON),
        "added Dragon wore off at cleanup"
    );

    tap_all_mana(&mut engine, p1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected P1 priority");
    };
    let &(opp_sliver, opp_ability_idx) = legal
        .abilities
        .iter()
        .find(|(id, _)| *id == sliver1)
        .expect("opponent's Muscle Sliver also receives Mistform Sliver's granted ability");
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: opp_sliver,
                ability_index: opp_ability_idx,
            },
        )
        .expect("P1 activates granted ability on Muscle Sliver");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("P1 chooses Elf");
    pass_until(&mut engine, |e| at_rest(e, p1));

    let opp_subs = &engine
        .state()
        .object(sliver1)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(opp_subs.contains(subtypes::creature::ELF));
    assert!(opp_subs.contains(subtypes::creature::SLIVER));
}
