//! `cards/creatures/mv_2/pygmy_hippo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn pygmy_hippo() -> CardIndex {
    card_index("0d39513a-1f05-4d9a-a772-745f4f1a3471")
}

/// `Pygmy Hippo` ({G}{U}, 2/2 Creature — Hippo, `Coverage::Partial`) is played
/// as a 2/2: unblocked it asks no question about the defending player's mana
/// and deals its 2 combat damage (the trigger that would assign none is left
/// out with the rest of the sentence).
#[test]
fn pygmy_hippo_attacks_unblocked_for_two_and_asks_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), island()])
        .battlefield(1, &[island()])
        .hand(0, &[pygmy_hippo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pygmy_hippo());
    pass_until(&mut engine, stack_is_empty);
    let hippo = on_battlefield(&engine, p0, pygmy_hippo()).unwrap();
    assert_eq!(pt(&engine, hippo), (2, 2));

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
                attackers: vec![(hippo, Defender::Player(p1))],
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

    assert_eq!(engine.state().players[1].life, 18);
}
