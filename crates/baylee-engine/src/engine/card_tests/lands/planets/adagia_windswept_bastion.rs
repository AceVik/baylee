//! `cards/lands/planets/adagia_windswept_bastion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Planet enters tapped and its ordinary mana ability produces white.
#[test]
fn adagia_windswept_bastion_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(218, forest())
        .hand(0, &[adagia_windswept_bastion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, adagia_windswept_bastion());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, adagia_windswept_bastion(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn adagia_stations_and_makes_a_legendary_copy_whose_supertype_is_copiable() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::types::SupertypeSet;
    let mut e = kit::game(
        978,
        &[adagia_windswept_bastion(), serra_angel(), sol_ring()],
        &[sol_ring()],
        &[baylee_core::generated::index::COPY_ARTIFACT],
        &[],
    );
    let planet = kit::obj(&e, kit::P0, adagia_windswept_bastion());
    let angel = kit::obj(&e, kit::P0, serra_angel());
    let ring = kit::obj(&e, kit::P0, sol_ring());
    crate::replacement::put_counters(&mut e.state, planet, CounterKind::Charge, 8);
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 4)]);
    assert!(!priority_offer(&e).abilities.contains(&(planet, 2)));
    activate(&mut e, kit::P0, adagia_windswept_bastion(), 1);
    e.apply(
        kit::P0,
        PlayerAction::ChooseObjects {
            objects: vec![angel],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, planet, CounterKind::Charge), 12);
    activate(&mut e, kit::P0, adagia_windswept_bastion(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("copy target")
    };
    assert_eq!(options, &[ring]);
    kit::aim(&mut e, &[ring], &[]);
    pass_until(&mut e, stack_is_empty);
    let token = tokens_of(&e, kit::P0)[0];
    assert!(
        e.state
            .object(token)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 2)]);
    let copy = in_hand(&e, kit::P0, baylee_core::generated::index::COPY_ARTIFACT).unwrap();
    cast_with_floating(
        &mut e,
        kit::P0,
        baylee_core::generated::index::COPY_ARTIFACT,
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        kit::P0,
        PlayerAction::ChooseObjects {
            objects: vec![token],
        },
    )
    .unwrap();
    // Copying the token copies legendary, causing a legend-rule choice.
    let Pending::LegendChoice { options, .. } = e.pending() else {
        panic!("legend rule: {:?}", e.pending())
    };
    assert!(options.contains(&token) && options.contains(&copy));
    e.apply(
        kit::P0,
        PlayerAction::ChooseObjects {
            objects: vec![copy],
        },
    )
    .unwrap();
    assert!(
        e.state
            .object(copy)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );
}
