//! `cards/lands/utility/hall_of_oracles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hall of Oracles prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Add one mana of any color.`, and `{{T}}: Put a +1/+1 counter on target creature. Activate only as a sorcery and only if you've cast an instant or sorcery spell this turn.`
///
/// Without an instant or sorcery cast this turn, only the mana abilities are offered.
/// Activating ability 1 filters the floating mana through `Pending::ChooseColor` into one blue mana.
#[test]
fn hall_of_oracles_filters_mana_before_an_instant_or_sorcery_is_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hall_of_oracles(), forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall =
        on_battlefield(&engine, p0, hall_of_oracles()).expect("hall of oracles on battlefield");

    // Float the Forest and the Elves' own {{G}} while keeping Hall of
    // Oracles untapped: two, and the counter ability charges nothing but a
    // tap, so the floating price is the {{1}} of ability 1.
    tap_mana_except(&mut engine, p0, hall);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );
    assert!(!is_tapped(&engine, hall));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(hall, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(hall, 1)),
        "ability 1 ({{1}}, {{T}}: Add one mana of any color) is offered"
    );
    assert!(
        !legal.abilities.contains(&(hall, 2)),
        "ability 2 requires an instant or sorcery cast this turn"
    );

    activate(&mut engine, p0, hall_of_oracles(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the {{1}} came out of the two floating greens, and one is left"
    );
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, hall));
}

#[test]
fn hall_counts_a_cast_even_when_countered_but_not_an_artifact_or_a_previous_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for spell in [index::DARK_RITUAL, index::MIND_STONE] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    hall_of_oracles(),
                    index::GRIZZLY_BEARS,
                    index::DOUBLING_SEASON,
                ],
            )
            .hand(0, &[spell])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let hall = on_battlefield(&e, p0, hall_of_oracles()).unwrap();
        let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 2);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(hall, 2)));
        cast_with_floating(&mut e, p0, spell);
        let card = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(
            legal.abilities.contains(&(hall, 2)),
            spell == index::DARK_RITUAL
        );
        if spell == index::DARK_RITUAL {
            activate(&mut e, p0, hall_of_oracles(), 2);
            aim_at(&mut e, p0, bear);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 2);
        }
        reach_their_main_phase(&mut e, p1);
        reach_their_main_phase(&mut e, p0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(hall, 2)));
    }
}

/// "Activate only as a sorcery and only if you've cast an instant or sorcery
/// spell this turn": the cast spell alone is not enough. With the instant
/// still on the stack and again in combat the counter ability is not offered;
/// in a main phase with the stack empty it is.
#[test]
fn hall_counter_ability_waits_for_an_empty_stack_in_a_main_phase() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = kit::game(
        1208,
        &[hall_of_oracles(), index::GRIZZLY_BEARS],
        &[],
        &[index::DARK_RITUAL],
        &[],
    );
    let bear = kit::obj(&e, p0, index::GRIZZLY_BEARS);
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut e, p0, index::DARK_RITUAL);
    assert!(
        !stack_is_empty(&e),
        "the instant is still on the stack, waiting for the pass"
    );
    assert!(
        !kit::offered(&e, hall_of_oracles(), p0, 2),
        "\"as a sorcery\": not while a spell is on the stack, though the instant has been cast"
    );

    pass_until(&mut e, stack_is_empty);
    assert!(
        kit::offered(&e, hall_of_oracles(), p0, 2),
        "main phase, empty stack, an instant cast this turn: offered"
    );

    kit::to_step(&mut e, p0, crate::turn::Step::CombatBegin);
    assert!(
        !kit::offered(&e, hall_of_oracles(), p0, 2),
        "\"as a sorcery\": not in combat, though an instant was cast this turn"
    );

    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        kit::offered(&e, hall_of_oracles(), p0, 2),
        "the second main phase with an empty stack is a sorcery window too"
    );
    activate(&mut e, p0, hall_of_oracles(), 2);
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 1);
}
