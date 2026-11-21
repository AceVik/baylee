//! `cards/creatures/mv_4/rampant_elephant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rampant_elephant() -> CardIndex {
    card_index("aaaaf370-6486-4e17-9a93-92e05ebcf29f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Rampant Elephant (aaaaf370-6486-4e17-9a93-92e05ebcf29f) — `{3}{W}` 2/2
/// Creature — Elephant:
/// "`{G}`: Target creature blocks this creature this turn if able."
#[test]
fn rampant_elephant_forces_target_untapped_creature_to_block_it_this_turn_if_able() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[rampant_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(forest()));
    cast_with_floating(&mut engine, P0, rampant_elephant());
    pass_until(&mut engine, stack_is_empty);

    let elephant = on_battlefield(&engine, P0, rampant_elephant()).expect("Elephant entered");
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, elephant), (2, 2));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, rampant_elephant(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elephant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "CR 509.1c: untapped target creature able to block Rampant Elephant must do so"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, elephant)],
            },
        )
        .expect("blocking Rampant Elephant obeys the requirement");
}

#[test]
fn rampant_elephant_if_able_does_not_force_a_tapped_creature_and_expires_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rampant_elephant(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let elephant = on_battlefield(&engine, P0, rampant_elephant()).unwrap();
    let elf = on_battlefield(&engine, P1, llanowar_elves()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, rampant_elephant(), 0);
    aim_at(&mut engine, P0, elf);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elephant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("tapped Llanowar Elves is unable to block");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Next turn cycle: Llanowar Elves untaps, and the previous turn's requirement expired.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elephant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("previous turn's block requirement expired at end of turn");
}
