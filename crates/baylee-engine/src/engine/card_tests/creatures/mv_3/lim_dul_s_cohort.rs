//! `cards/creatures/mv_3/lim_dul_s_cohort.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lim_dul_s_cohort() -> CardIndex {
    card_index("1e3b97d2-8fda-4510-9697-f36ca9ca2ab0")
}

/// Lim-Dûl's Cohort (1e3b97d2-8fda-4510-9697-f36ca9ca2ab0) — {1}{B}{B} 2/3 Creature — Zombie:
/// "Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn."
#[test]
fn lim_dul_s_cohort_forbids_regeneration_both_when_it_blocks_and_when_it_becomes_blocked() {
    // 1. Becomes blocked by: P0 attacks with Lim-Dûl's Cohort (2/3); P1 shields Lotleth Troll (2/1)
    // and blocks. The trigger forbids regeneration, so Lotleth Troll dies through its shield.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[lim_dul_s_cohort()])
        .battlefield(1, &[lotleth_troll(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let cohort = on_battlefield(&engine, P0, lim_dul_s_cohort()).unwrap();
    let troll = on_battlefield(&engine, P1, lotleth_troll()).unwrap();

    // P1 raises a regeneration shield on Lotleth Troll when P0 passes priority.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    raise_a_shield(&mut engine, P1, troll, 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cohort, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(troll, cohort)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine, P1, lotleth_troll()).is_some(),
        "Lotleth Troll died through its regeneration shield when blocking Lim-Dûl's Cohort"
    );

    // 2. Blocks: On P1's turn, P1 shields a fresh Lotleth Troll and attacks; P0 blocks with Lim-Dûl's Cohort.
    let mut engine2 = Duel::new(SEED, swamp())
        .battlefield(0, &[lim_dul_s_cohort()])
        .battlefield(1, &[lotleth_troll(), swamp()])
        .start();
    keep_mulligans(&mut engine2);
    reach_their_main_phase(&mut engine2, P1);

    let cohort2 = on_battlefield(&engine2, P0, lim_dul_s_cohort()).unwrap();
    let troll2 = on_battlefield(&engine2, P1, lotleth_troll()).unwrap();
    tap_all_mana_but(&mut engine2, P1, None);
    raise_a_shield(&mut engine2, P1, troll2, 1);

    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine2
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troll2, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine2
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(cohort2, troll2)],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine2, P1, lotleth_troll()).is_some(),
        "Lotleth Troll died through its regeneration shield when blocked by Lim-Dûl's Cohort"
    );
}
