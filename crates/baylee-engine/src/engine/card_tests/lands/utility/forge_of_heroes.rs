//! `cards/lands/utility/forge_of_heroes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Forge's colorless mana pays for a real Sol Ring spell.
#[test]
fn forge_of_heroes_taps_for_colourless_mana_that_pays_for_a_one_cost_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forge_of_heroes(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, forge_of_heroes());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Forge is tapped"
    );
    assert!(
        !is_tapped(&engine, land),
        "the card prints no enters-tapped clause, so the land drop leaves it standing"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the Forge is the only mana source on this board — and a nonbasic land \
         printing its own `{{T}}: Add {{C}}` is a route the kit has to find (#159)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "`{{T}}: Add {{C}}`"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not the Forests the library is made of"
    );
    assert!(is_tapped(&engine, land), "the Forge paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );

    // The half a bare pool count cannot show: the {C} is spent.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the {{C}} paid a {{1}} cost, so what the Forge made was usable \
         colourless mana and not a colour it does not print"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty because the spell took it"
    );
    assert!(
        on_battlefield(&engine, p0, forge_of_heroes()).is_some(),
        "the land itself is untouched by the tap that paid for the artifact"
    );
}

#[test]
fn forge_marks_a_commander_that_entered_this_turn_by_its_current_card_types() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (card, color, n, kind, initial) in [
        (
            index::SAVANNAH_LIONS,
            ManaColor::White,
            1,
            CounterKind::P1P1,
            0,
        ),
        (
            index::JACE_THE_MIND_SCULPTOR,
            ManaColor::Blue,
            4,
            CounterKind::Loyalty,
            3,
        ),
    ] {
        let mut e = Duel::new(SEED, forest())
            .commander(0, &[card])
            .battlefield(0, &[forge_of_heroes(), quiet_creature()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let commander = e.state().commanders[0][0].object;
        let other = on_battlefield(&e, p0, quiet_creature()).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(color, n);
        e.refresh_offer();
        e.apply(p0, PlayerAction::CastSpell { card: commander })
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        activate(&mut e, p0, forge_of_heroes(), 1);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            unreachable!()
        };
        assert!(options.contains(&commander));
        assert!(!options.contains(&other));
        aim_at(&mut e, p0, commander);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, commander, kind), initial + 1);
    }
}
