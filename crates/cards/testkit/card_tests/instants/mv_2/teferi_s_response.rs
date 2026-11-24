//! `cards/instants/mv_2/teferi_s_response.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit::castable;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn teferi_s_response() -> CardIndex {
    card_index("b2faa8b6-e433-4171-9774-9170484530c4")
}

/// A spell an opponent controls is countered when it targets our land. Its
/// source is not a permanent to destroy, and the two-card rider still happens.
#[test]
fn counters_an_opponents_land_destroying_spell_and_draws_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[teferi_s_response()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let land = on_battlefield(&engine, P0, forest()).expect("our Forest is out");
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, stone_rain());
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("Stone Rain targets our Forest");
    let stone = on_stack(&engine, stone_rain()).expect("Stone Rain is on the stack");
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, teferi_s_response());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a stack target, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![stone]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![stone],
            },
        )
        .unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, stone_rain()).is_some());
    assert!(on_battlefield(&engine, P0, forest()).is_some());
    assert!(in_graveyard(&engine, P0, teferi_s_response()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 2
    );
}

/// Countering the activated ability also destroys its source. The targeted
/// land remains, and the ability never taps it as its effect.
#[test]
fn counters_a_land_ability_destroys_rishadan_port_and_draws_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[teferi_s_response()])
        .battlefield(1, &[rishadan_port(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let land = on_battlefield(&engine, P0, forest()).expect("our Forest is out");
    let port = on_battlefield(&engine, P1, rishadan_port()).expect("the Port is out");
    tap_all_mana_but(&mut engine, P1, Some(rishadan_port()));
    activate(&mut engine, P1, rishadan_port(), 1);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the Port targets our Forest");
    assert!(is_tapped(&engine, port), "the Port paid its tap cost");
    let ability = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, teferi_s_response());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected an ability target, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ability]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ability],
            },
        )
        .unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, rishadan_port()).is_some());
    assert!(on_battlefield(&engine, P0, forest()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 2
    );
}

/// The controller and target restrictions are independent: neither our own
/// spell nor an opponent's spell aimed at that opponent's land is legal.
#[test]
fn rejects_our_spell_and_a_spell_targeting_our_opponents_land() {
    let mut own_spell = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                forest(),
            ],
        )
        .hand(0, &[stone_rain(), teferi_s_response()])
        .start();
    keep_mulligans(&mut own_spell);
    reach_main_phase(&mut own_spell, P0);
    let our_land = on_battlefield(&own_spell, P0, forest()).expect("our Forest is out");
    tap_all_mana(&mut own_spell, P0);
    cast_with_floating(&mut own_spell, P0, stone_rain());
    own_spell
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![our_land],
            },
        )
        .unwrap();
    assert!(!castable(&own_spell, P0, teferi_s_response()));

    let mut their_land = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[teferi_s_response()])
        .battlefield(1, &[mountain(), mountain(), mountain(), forest()])
        .hand(1, &[stone_rain()])
        .start();
    keep_mulligans(&mut their_land);
    reach_their_main_phase(&mut their_land, P1);
    let opponents_land = on_battlefield(&their_land, P1, forest()).expect("their Forest is out");
    tap_all_mana(&mut their_land, P1);
    cast_with_floating(&mut their_land, P1, stone_rain());
    their_land
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![opponents_land],
            },
        )
        .unwrap();
    their_land.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut their_land, P0);
    assert!(!castable(&their_land, P0, teferi_s_response()));
}
