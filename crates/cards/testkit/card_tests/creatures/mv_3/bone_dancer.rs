//! `cards/creatures/mv_3/bone_dancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bone_dancer() -> CardIndex {
    card_index("13b5c9ef-d0e3-453d-ae5e-8de4f310cc38")
}

/// `Bone Dancer` ({1}{B}{B}, 2/2 Creature — Zombie, `Coverage::Partial`) is
/// played as its 2/2 body: attacking unblocked it asks no question and deals
/// its 2 combat damage (the card's "may … assigns no combat damage" option is
/// not offered, because that was a drawback without the graveyard half).
#[test]
fn bone_dancer_attacks_unblocked_for_two_and_asks_nothing() {
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
    assert_eq!(pt(&engine, dancer), (2, 2));

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
    // `pass_until` answers a "may" with yes and would hide an offered trigger;
    // walking to P1's main phase without a stop proves none stands on the stack
    // by the time combat damage is dealt.
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[p1.get() as usize].life, 18);
    assert_eq!(engine.state().players[p0.get() as usize].life, 20);
}
