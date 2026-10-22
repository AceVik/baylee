//! `cards/creatures/mv_3/bone_dancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bone_dancer() -> CardIndex {
    card_index("13b5c9ef-d0e3-453d-ae5e-8de4f310cc38")
}

/// `Bone Dancer` ({1}{B}{B}, 2/2 Creature — Zombie, `Coverage::Partial`)
/// triggers when it attacks and isn't blocked so you may have it assign no
/// combat damage this turn.
#[test]
fn bone_dancer_triggers_when_unblocked_and_may_assign_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[bone_dancer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bone_dancer());
    pass_until(&mut engine, stack_is_empty);
    let dancer = on_battlefield(&engine, p0, bone_dancer()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dancer, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[p1.get() as usize].life, 20);
}
