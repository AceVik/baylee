//! `cards/enchantments/mv_3/prismatic_circle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "022dae2e-7fc3-486e-9168-59652d9ab21c"
fn prismatic_circle() -> CardIndex {
    card_index("022dae2e-7fc3-486e-9168-59652d9ab21c")
}

#[test]
fn prismatic_circle_chooses_a_color_and_prevents_damage_from_a_chosen_source_of_that_color() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[prismatic_circle()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, prismatic_circle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    let selected = baylee_core::ids::DamageSourceRef {
        object: giant,
        version: engine.state().object(giant).unwrap().version,
    };

    activate(&mut engine, P0, prismatic_circle(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(
        options,
        vec![selected],
        "the green Grizzly Bears are not a source of the chosen colour"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().shields.len(), 1);
}

fn age_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(baylee_cards_dsl::counters::AGE))
}

/// Cast, name red, and leave the Circle on the battlefield.
fn circle_naming_red(engine: &mut Engine<RegistryLookup>) {
    lk::float(
        engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    cast_with_floating(engine, P0, prismatic_circle());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(engine);
}

/// "Prevent that damage": the red Giant's combat damage is prevented, the
/// green Bears (not the chosen colour) still hit for 2.
#[test]
fn prismatic_circle_prevents_the_chosen_sources_damage_and_only_that() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[prismatic_circle()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    circle_naming_red(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    lk::attack(&mut engine, &[giant, bears]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    let selected = baylee_core::ids::DamageSourceRef {
        object: giant,
        version: engine.state().object(giant).unwrap().version,
    };
    activate(&mut engine, P0, prismatic_circle(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource { choice, .. } = engine.pending().clone() else {
        unreachable!();
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    lk::through_combat(&mut engine);

    assert_eq!(
        lk::life(&engine, P0),
        18,
        "the Giant's 3 is prevented, the Bears' 2 is not"
    );
}

/// "Cumulative upkeep {1}": the first upkeep puts an age counter and asks for
/// {1}; declining sacrifices the Circle.
#[test]
fn prismatic_circle_is_sacrificed_when_its_upkeep_is_declined() {
    let mut engine = lk::game(SEED, &[], &[], &[prismatic_circle()], &[]);
    circle_naming_red(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (3, P0),
        "the upkeep is its controller's own"
    );
    let circle = lk::obj(&engine, P0, prismatic_circle());
    assert_eq!(age_counters(&engine, circle), 1);

    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, prismatic_circle()).is_some());
}

/// Paying {1} keeps it; the second upkeep costs {2}.
#[test]
fn prismatic_circle_paid_upkeep_keeps_it_and_costs_one_more_each_time() {
    let mut engine = lk::game(SEED, &[], &[], &[prismatic_circle()], &[]);
    circle_naming_red(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    let circle = lk::obj(&engine, P0, prismatic_circle());
    assert_eq!(age_counters(&engine, circle), 1);
    assert_eq!(lk::pool_total(&engine, P0), 0, "{{1}} was paid");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.number, 5);
    assert_eq!(age_counters(&engine, circle), 2);
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "two age counters cost {{2}}"
    );
    assert!(on_battlefield(&engine, P0, prismatic_circle()).is_some());
}
