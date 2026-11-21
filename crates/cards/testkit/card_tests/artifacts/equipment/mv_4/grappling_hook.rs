//! `cards/artifacts/equipment/mv_4/grappling_hook.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn grappling_hook() -> CardIndex {
    card_index("24c193cc-3f83-4414-8cac-8b9f48d5bb6d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn equip_hook(engine: &mut Engine<RegistryLookup>, p0: PlayerId, host: ObjectId) {
    let hook = on_battlefield(engine, p0, grappling_hook()).unwrap();
    tap_all_mana(engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hook,
                ability_index: 0,
            },
        )
        .expect("equip {4}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn attack_and_answer_hook(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    p1: PlayerId,
    attacker: ObjectId,
    target_blocker: ObjectId,
    accept: bool,
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_blocker],
            },
        )
        .unwrap();
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    engine.apply(p0, PlayerAction::YesNo(accept)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

/// Grappling Hook (24c193cc-3f83-4414-8cac-8b9f48d5bb6d) — {4} Artifact — Equipment:
/// "Equipped creature has double strike."
/// "Whenever equipped creature attacks, you may have target creature block it
/// this turn if able."
/// "Equip {4}"
#[test]
fn grappling_hook_double_strike_and_forced_block_when_accepted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                grappling_hook(),
                hill_giant(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let their_giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    equip_hook(&mut engine, p0, my_giant);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(keywords(&engine, my_giant).contains(KeywordSet::DOUBLE_STRIKE));

    attack_and_answer_hook(&mut engine, p0, p1, my_giant, their_giant, true);

    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "targeted creature must block equipped creature if able"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_giant, my_giant)],
            },
        )
        .expect("block as forced by Grappling Hook");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, hill_giant()).is_some());
    assert!(on_battlefield(&engine, p0, hill_giant()).is_some());
    assert_eq!(engine.state().object(my_giant).unwrap().damage, 0);
}

/// Declining the "may" trigger allows the target creature not to block, and an
/// unblocked double-striker deals double its power in combat damage.
#[test]
fn grappling_hook_declining_trigger_allows_no_block_and_deals_double_strike_to_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                grappling_hook(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    equip_hook(&mut engine, p0, my_bears);

    attack_and_answer_hook(&mut engine, p0, p1, my_bears, their_bears, false);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining Grappling Hook leaves blocking optional");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        life(&engine, p1),
        16,
        "unblocked 2/2 double strike dealt 4 damage"
    );
}
