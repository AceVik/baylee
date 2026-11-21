//! `cards/instants/mv_3/shunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e714bcfb-b451-4b7b-ab60-4cb845c75647"
fn shunt() -> CardIndex {
    card_index("e714bcfb-b451-4b7b-ab60-4cb845c75647")
}

/// Shunt ({1}{R}{R}): changes the target of target spell with a single target,
/// and cannot target a spell with two targets.
#[test]
fn shunt_redirects_single_target_spell_and_rejects_two_target_spell() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[shunt()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, lightning_bolt());
    lk::aim(&mut engine, &[], &[P0]);
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, shunt());
    lk::aim(&mut engine, &[bolt], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, P0), 20, "P0 avoided the Bolt");
    assert_eq!(lk::life(&engine, P1), 17, "redirected Bolt hit P1 for 3");

    // Two-target Fire is not a legal target for Shunt.
    let mut two_targets = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[shunt()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[fire()])
        .start();
    keep_mulligans(&mut two_targets);
    reach_their_main_phase(&mut two_targets, P1);
    tap_all_mana(&mut two_targets, P1);
    cast_with_floating(&mut two_targets, P1, fire());
    if matches!(two_targets.pending(), Pending::ChooseCastMode { .. }) {
        let f0 = choose_cast_kind(&two_targets, CastModeKind::Face(0));
        two_targets.apply(P1, PlayerAction::ChooseMode(f0)).unwrap();
    }
    two_targets
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0, P1],
            },
        )
        .unwrap();
    if matches!(two_targets.pending(), Pending::ChooseNumber { .. }) {
        two_targets
            .apply(P1, PlayerAction::ChooseNumber(1))
            .unwrap();
    }
    two_targets.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut two_targets, P0);
    assert!(
        !lk::castable(&two_targets, P0, shunt()),
        "two-target Fire cannot be targeted by Shunt"
    );
}
