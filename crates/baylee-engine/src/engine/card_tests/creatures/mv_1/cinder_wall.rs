//! `cards/creatures/mv_1/cinder_wall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CINDER_WALL, HILL_GIANT};

/// `Cinder Wall` ({R}, Creature — Wall 3/3) has defender and is destroyed at
/// end of combat when it blocks.
#[test]
fn cinder_wall_is_destroyed_at_end_of_combat_when_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CINDER_WALL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, CINDER_WALL);
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, CINDER_WALL).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, giant)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&engine, p0, CINDER_WALL).is_some());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
