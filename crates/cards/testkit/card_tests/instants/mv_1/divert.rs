//! `cards/instants/mv_1/divert.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "561aed01-322b-4699-b7b0-f4416075aa9b"
fn divert() -> CardIndex {
    card_index("561aed01-322b-4699-b7b0-f4416075aa9b")
}

/// Divert ({U}): redirects a single-target spell when its controller declines
/// to pay {2}, leaves the target unchanged when they pay {2}, and cannot
/// target a spell with two targets.
#[test]
fn divert_changes_single_target_unless_controller_pays_two() {
    // 1. Declining {2} redirects Lightning Bolt from P0 back onto P1.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[divert()])
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
    cast_with_floating(&mut engine, P0, divert());
    lk::aim(&mut engine, &[bolt], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::PayTax { mana: 2 }, .. } if *player == P1
    ));
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
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

    assert_eq!(lk::life(&engine, P0), 20, "P0 dodged the Bolt");
    assert_eq!(lk::life(&engine, P1), 17, "redirected Bolt hit P1 for 3");

    // 2. Paying {2} keeps Lightning Bolt aimed at P0.
    let mut paid = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[divert()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut paid);
    reach_their_main_phase(&mut paid, P1);

    tap_all_mana(&mut paid, P1);
    cast_with_floating(&mut paid, P1, lightning_bolt());
    lk::aim(&mut paid, &[], &[P0]);
    let bolt = on_stack(&paid, lightning_bolt()).unwrap();
    paid.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut paid, P0);
    cast_with_floating(&mut paid, P0, divert());
    lk::aim(&mut paid, &[bolt], &[]);
    pass_until(&mut paid, |e| matches!(e.pending(), Pending::YesNo { .. }));
    paid.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut paid, stack_is_empty);

    assert_eq!(
        lk::pool_total(&paid, P1),
        0,
        "P1 spent {{2}} to stop Divert"
    );
    assert_eq!(lk::life(&paid, P0), 17, "original Bolt still hit P0");
    assert_eq!(lk::life(&paid, P1), 20);
}

/// Divert cannot target a spell that has two targets (e.g., Fire dividing 2
/// damage between two targets).
#[test]
fn divert_cannot_target_a_spell_with_two_targets() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[divert()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, fire());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let f0 = choose_cast_kind(&engine, CastModeKind::Face(0));
        engine.apply(P1, PlayerAction::ChooseMode(f0)).unwrap();
    }
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0, P1],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(P1, PlayerAction::ChooseNumber(1)).unwrap();
    }
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    assert!(
        !lk::castable(&engine, P0, divert()),
        "two-target Fire is not a spell with a single target"
    );
}
