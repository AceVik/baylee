//! `cards/creatures/mv_4/varchild_s_crusader.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::WALL_OF_WOOD;

#[test]
fn varchild_s_crusader_can_only_be_blocked_by_walls_and_sacrifices_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("45351d9d-453f-4245-84c0-053eb8aca345");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[grizzly_bears(), WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crusader = on_battlefield(&engine, p0, card).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let wall = on_battlefield(&engine, p1, WALL_OF_WOOD).unwrap();

    // Activate {0}: can't be blocked this turn except by Walls; sacrifice at next end step.
    activate(&mut engine, p0, card, 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crusader, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Non-Wall Grizzly Bears cannot block Varchild's Crusader.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bear, crusader)],
                },
            )
            .is_err()
    );

    // Wall of Wood CAN block Varchild's Crusader.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, crusader)],
            },
        )
        .unwrap();

    // Advance past the end step so the delayed sacrifice trigger fires and resolves.
    pass_until(&mut engine, |e| e.state().turn.active == p1);

    assert!(on_battlefield(&engine, p0, card).is_none());
    assert!(in_graveyard(&engine, p0, card).is_some());
}

#[test]
fn varchild_s_crusader_is_blockable_by_any_creature_until_its_ability_is_used() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("45351d9d-453f-4245-84c0-053eb8aca345");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let crusader = on_battlefield(&engine, p0, card).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crusader, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bear, crusader)],
                },
            )
            .is_ok(),
        "without the activation a non-Wall may block"
    );
}

#[test]
fn varchild_s_crusader_is_not_sacrificed_unless_its_ability_is_used() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("45351d9d-453f-4245-84c0-053eb8aca345");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_graveyard(&engine, p0, card).is_none());
}
