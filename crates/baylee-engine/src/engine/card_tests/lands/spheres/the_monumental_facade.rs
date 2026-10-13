//! `cards/lands/spheres/the_monumental_facade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Monumental Facade prints `This land enters with two oil counters on it.`,
/// `{{T}}: Add {{C}}.`, and `{{T}}, Remove an oil counter from this land: Put an oil
/// counter on target artifact or creature you control. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, oil counters have no `CounterKind` constant in the DSL,
/// so the counter-related abilities are omitted. Activating ability 0 adds one colorless
/// mana to the pool and taps the land.
#[test]
fn the_monumental_facade_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_monumental_facade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let facade =
        on_battlefield(&engine, p0, the_monumental_facade()).expect("facade on battlefield");
    activate(&mut engine, p0, the_monumental_facade(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, facade));
}

#[test]
fn facade_enters_with_oil_and_pays_one_counter_to_place_a_doubled_counter() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let oil = baylee_cards_dsl::counters::OIL;
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::DOUBLING_SEASON, index::GRIZZLY_BEARS])
        .hand(0, &[index::THE_MONUMENTAL_FACADE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = play_land(&mut e, p0, index::THE_MONUMENTAL_FACADE);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(counters_on(&e, land, oil), 4);
    activate(&mut e, p0, index::THE_MONUMENTAL_FACADE, 1);
    aim_at(&mut e, p0, bear);
    assert_eq!(counters_on(&e, land, oil), 3);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, oil), 2);
    assert!(is_tapped(&e, land));
}

/// "Put an oil counter on target artifact or creature you control." The menu
/// holds the artifacts and creatures of the activating player and nothing
/// else: not the same types under the opponent, not a noncreature,
/// nonartifact permanent of their own (a land, an enchantment, the Facade
/// itself).
#[test]
fn facade_offers_only_your_own_artifacts_and_creatures_as_targets() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::SOL_RING,
                index::GLORIOUS_ANTHEM,
                forest(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::SOL_RING])
        .hand(0, &[index::THE_MONUMENTAL_FACADE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = play_land(&mut e, p0, index::THE_MONUMENTAL_FACADE);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&e, p0, index::SOL_RING).unwrap();
    let foreign = [
        on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap(),
        on_battlefield(&e, p1, index::SOL_RING).unwrap(),
    ];
    let nonmatching = [
        on_battlefield(&e, p0, index::GLORIOUS_ANTHEM).unwrap(),
        on_battlefield(&e, p0, forest()).unwrap(),
        land,
    ];

    activate(&mut e, p0, index::THE_MONUMENTAL_FACADE, 1);
    let mut options = aim_at(&mut e, p0, bear);
    options.sort();
    let mut wanted = vec![bear, ring];
    wanted.sort();
    assert_eq!(
        options, wanted,
        "target artifact or creature you control: your creature and your artifact, only"
    );
    for id in foreign {
        assert!(
            !options.contains(&id),
            "\"you control\": the opponent's artifact and creature are not on the menu"
        );
    }
    for id in nonmatching {
        assert!(
            !options.contains(&id),
            "\"artifact or creature\": a land or an enchantment of yours is not on the menu"
        );
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, bear, baylee_cards_dsl::counters::OIL),
        1,
        "the counter went to the chosen target"
    );
}

/// "Activate only as a sorcery." The price has no mana in it, so a Facade that
/// is untapped and still holds its oil counters is refused for the timing
/// alone: in the opponent's main phase with an empty stack, on a step that is
/// not a main phase, and in its own main phase over a spell on the stack. The
/// last case ends with the same activation accepted once the stack empties.
#[test]
fn facade_puts_its_oil_counter_only_as_a_sorcery() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let oil = baylee_cards_dsl::counters::OIL;
    let setup = || {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::GRIZZLY_BEARS])
            .hand(0, &[index::THE_MONUMENTAL_FACADE, llanowar_elves()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = play_land(&mut e, p0, index::THE_MONUMENTAL_FACADE);
        (e, land)
    };
    let refused = |e: &mut Engine<RegistryLookup>, land: ObjectId, what: &str| {
        let Pending::Priority { player, legal, .. } = e.pending().clone() else {
            panic!("{what}: expected priority, got {:?}", e.pending())
        };
        assert_eq!(player, p0, "{what}: the Facade's controller holds priority");
        assert!(!is_tapped(e, land), "{what}: the Facade is untapped");
        assert_eq!(counters_on(e, land, oil), 2, "{what}: both oil counters");
        assert!(
            !legal.abilities.contains(&(land, 1)),
            "{what}: the oil ability is not offered"
        );
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .is_err(),
            "{what}: only as a sorcery"
        );
        assert_eq!(counters_on(e, land, oil), 2, "{what}: nothing was paid");
    };

    // The opponent's turn, their main phase, nothing on the stack.
    let (mut e, land) = setup();
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(stack_is_empty(&e));
    refused(&mut e, land, "the opponent's main phase");

    // Our own turn, a step that is no main phase.
    let (mut e, land) = setup();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        !matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain),
        "the pass left the main phase"
    );
    refused(&mut e, land, "outside a main phase");

    // Our own main phase over a spell on the stack.
    let (mut e, land) = setup();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, llanowar_elves());
    assert!(!stack_is_empty(&e), "the Elves wait on the stack");
    refused(&mut e, land, "a spell on the stack");
    pass_until(&mut e, stack_is_empty);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, p0, index::THE_MONUMENTAL_FACADE, 1);
    aim_at(&mut e, p0, bear);
    assert_eq!(counters_on(&e, land, oil), 1, "the counter is the price");
    assert!(is_tapped(&e, land), "{{T}} is the other half of the price");
}
