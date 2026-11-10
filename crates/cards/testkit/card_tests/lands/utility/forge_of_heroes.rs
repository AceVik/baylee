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

/// "Choose target commander that entered this turn": a commander that entered
/// on an earlier turn and a noncommander creature that entered this turn are
/// both outside the target options; the commander that entered this turn is
/// the only one inside.
#[test]
fn forge_of_heroes_options_are_only_the_commander_that_entered_this_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::SAVANNAH_LIONS, index::WHITE_KNIGHT])
        .battlefield(0, &[forge_of_heroes()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lions = e.state().commanders[0][0].object;
    let knight = e.state().commanders[0][1].object;

    // Turn one: the first commander enters.
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    e.apply(p0, PlayerAction::CastSpell { card: lions })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(lions).unwrap().zone,
        Zone::Battlefield,
        "the first commander is on the battlefield"
    );

    // The next turn of the same player: the second commander and a
    // noncommander creature enter.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let state = e.dev_state_mut(p0).unwrap();
    state.players[0].mana_pool.add(ManaColor::White, 2);
    state.players[0].mana_pool.add(ManaColor::Green, 2);
    e.refresh_offer();
    e.apply(p0, PlayerAction::CastSpell { card: knight })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears entered");
    assert_eq!(e.state().object(knight).unwrap().zone, Zone::Battlefield);

    activate(&mut e, p0, forge_of_heroes(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected a target choice, got {:?}", e.pending());
    };
    assert!(
        options.contains(&knight),
        "the commander that entered this turn is a legal target"
    );
    assert!(
        !options.contains(&lions),
        "\"entered this turn\": a commander that entered on an earlier turn is not"
    );
    assert!(
        !options.contains(&bears),
        "\"target commander\": a noncommander creature that entered this turn is not"
    );
    assert_eq!(options.len(), 1, "exactly one legal target: {options:?}");
}
