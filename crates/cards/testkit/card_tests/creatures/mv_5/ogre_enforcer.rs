//! `cards/creatures/mv_5/ogre_enforcer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ogre_enforcer() -> CardIndex {
    card_index("86cc83d8-4a70-4b97-aa4d-160afd4cd6cb")
}

/// `Ogre Enforcer` ({3}{R}{R}, 4/4 Creature — Ogre, `Coverage::Partial`) is
/// played as a 4/4: cast with five Mountains, it attacks unblocked on the next
/// turn and deals 4 combat damage. Its damage-resistance sentence is not
/// played (the body is only less than printed).
#[test]
fn ogre_enforcer_attacks_unblocked_for_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ogre_enforcer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ogre_enforcer());
    pass_until(&mut engine, stack_is_empty);
    let ogre = on_battlefield(&engine, p0, ogre_enforcer()).unwrap();
    assert_eq!(pt(&engine, ogre), (4, 4));

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
                attackers: vec![(ogre, Defender::Player(p1))],
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

    assert_eq!(engine.state().players[1].life, 16);
}
