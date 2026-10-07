//! `cards/creatures/mv_4/questing_beast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Combat damage to a planeswalker is not combat damage to an opponent: the
/// Beast sent at the opponent's Oko deals it 4 and asks nothing more.
#[test]
fn questing_beast_hitting_a_planeswalker_does_not_trigger() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[questing_beast()])
        .battlefield(1, &[forest(), island(), forest()])
        .hand(1, &[oko_thief_of_crowns()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);
    let theirs = on_battlefield(&engine, p1, oko_thief_of_crowns()).unwrap();
    activate(&mut engine, p1, oko_thief_of_crowns(), 0);
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, questing_beast()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&beast)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Planeswalker(theirs))],
            },
        )
        .expect("at the Oko");
    // `pass_until` panics on a target question, so reaching the next turn
    // is the assertion that none was asked.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::Loyalty),
        2,
        "6 − 4"
    );
}

/// Rock Hydra's counters remove even damage that "can't be prevented":
/// Questing Beast's "Combat damage that would be dealt by creatures you
/// control can't be prevented" reaches every creature its controller
/// controls, not only the Beast itself. Llanowar Elves (no deathtouch, not
/// the Beast) attacks alongside a Questing Beast it shares a controller
/// with; a three-counter Hydra blocks the Elves and is still dealt the
/// full point of combat damage even though the counter ability removes a
/// counter for it (CR 615.12, 615.12a). Counter-check: the identical block
/// without the Beast on the board leaves the same counter lost, but none
/// of the damage marked.
#[test]
fn rock_hydra_takes_unpreventable_combat_damage_from_a_questing_beasts_teammate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let hydra_card = card_index("aff84707-f5f8-4f53-869e-feec78da8d8d");

    // With Questing Beast beside the attacker. The counters are seeded
    // before the first mulligan question, not after: the printed body is a
    // 0/0, and a state-based action would otherwise bury it (CR 704.5f)
    // before this test ever gets to add any.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[questing_beast(), llanowar_elves()])
        .battlefield(1, &[hydra_card])
        .start();
    let hydra = on_battlefield(&engine, p1, hydra_card).expect("the Hydra is seated");
    {
        let state = engine
            .dev_state_mut(p1)
            .expect("the harness may set boards up");
        state
            .object_mut(hydra)
            .expect("seated")
            .counters
            .add(CounterKind::P1P1, 3);
    }
    keep_mulligans(&mut engine);
    assert_eq!(pt(&engine, hydra), (3, 3), "three +1/+1 counters on a 0/0");
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    let blockers = attack_and_collect_blocks(&mut engine, elf, p1);
    assert!(
        blockers
            .iter()
            .any(|o| o.blocker == hydra && o.attackers.contains(&elf)),
        "the Hydra may block the Elves: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(hydra, elf)],
            },
        )
        .expect("the Hydra blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        pt(&engine, hydra),
        (2, 2),
        "one counter removed absorbing the Elves' 1 damage"
    );
    assert_eq!(
        engine.state().object(hydra).map(|o| o.damage),
        Some(1),
        "Questing Beast: the Elves' combat damage to it can't be prevented, \
         so the Hydra is still dealt the point despite the counter's removal"
    );

    // Counter-check: the identical block, minus Questing Beast.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[hydra_card])
        .start();
    let hydra = on_battlefield(&engine, p1, hydra_card).expect("the Hydra is seated");
    {
        let state = engine
            .dev_state_mut(p1)
            .expect("the harness may set boards up");
        state
            .object_mut(hydra)
            .expect("seated")
            .counters
            .add(CounterKind::P1P1, 3);
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    attack_and_collect_blocks(&mut engine, elf, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(hydra, elf)],
            },
        )
        .expect("the Hydra blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        pt(&engine, hydra),
        (2, 2),
        "the same counter is lost absorbing the same 1 damage"
    );
    assert_eq!(
        engine.state().object(hydra).map(|o| o.damage),
        Some(0),
        "without Questing Beast, the 1 damage is preventable, so the \
         counter ability prevents all of it: nothing is marked"
    );
}
