//! `cards/instants/mv_4/agonizing_demise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "41681a1f-0777-4b96-8c7e-d6a5d1b1c8fc"
fn agonizing_demise() -> CardIndex {
    card_index("41681a1f-0777-4b96-8c7e-d6a5d1b1c8fc")
}

/// Agonizing Demise ({3}{B}, Kicker {1}{R}): rejects black creatures, destroys
/// a nonblack creature through an active regeneration shield, and deals 0
/// damage to its controller when unkicked.
#[test]
fn agonizing_demise_unkicked_ignores_regeneration_and_rejects_black() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), index::RIVER_BOA, index::DRUDGE_SKELETONS])
        .hand(0, &[agonizing_demise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let boa = lk::obj(&engine, P1, index::RIVER_BOA);
    let skel = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, agonizing_demise());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&boa));
    assert!(
        !options.contains(&skel),
        "black creature is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![boa],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 activates River Boa's regeneration shield in response.
    tap_all_mana(&mut engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: boa,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(boa)
            .is_some_and(|obj| obj.regeneration_shields == 1)
    });

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P1, index::RIVER_BOA).is_some(),
        "destroyed through regeneration shield"
    );
    assert_eq!(lk::life(&engine, P1), 20, "unkicked deals no damage to P1");
}

/// When kicked for {1}{R}, Agonizing Demise destroys the nonblack creature AND
/// deals damage equal to its power to its controller.
#[test]
fn agonizing_demise_kicked_deals_destroyed_creatures_power_to_its_controller() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[agonizing_demise()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 4), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, agonizing_demise());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "all {{4}}{{B}}{{R}} spent");
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert_eq!(
        lk::life(&engine, P1),
        17,
        "kicked Agonizing Demise deals Hill Giant's power (3) to its controller"
    );
}
