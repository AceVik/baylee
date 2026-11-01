//! Liar's Pendulum draws only after a reveal and a wrong guess.

use super::*;

fn liar_s_pendulum() -> CardIndex {
    card_index("4f88ddfa-5478-426a-a978-dfc64c687db1")
}

/// Names `named` with a Forest as the only card in hand, has the opponent
/// guess, answers the reveal, and says whether the Pendulum drew.
fn play(named: CardIndex, guess_in_hand: bool, reveal: bool) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[liar_s_pendulum(), island(), island()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, liar_s_pendulum(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseCardName {
            card: named,
            face: 0,
        },
    )
    .unwrap();
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("expected the opponent's guess")
    };
    assert_eq!(player, p1);
    e.apply(p1, PlayerAction::YesNo(guess_in_hand)).unwrap();
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("expected the optional reveal")
    };
    assert_eq!(player, p0);
    let before = library_size(&e, p0);
    e.apply(p0, PlayerAction::YesNo(reveal)).unwrap();
    pass_until(&mut e, stack_is_empty);
    library_size(&e, p0) < before
}

#[test]
fn liar_s_pendulum_draws_only_when_reveal_proves_the_guess_wrong() {
    assert!(
        play(forest(), false, true),
        "named card in hand, guessed not"
    );
    assert!(!play(forest(), true, true), "guessed right");
    assert!(!play(forest(), false, false), "no reveal, no draw");
}

/// The named card is not in hand, so "in hand" is the wrong guess and
/// "not in hand" the right one. Every case above names a card that is in
/// hand, where a wrong guess and a "no" answer are the same thing.
#[test]
fn liar_s_pendulum_judges_the_guess_against_a_name_not_in_hand() {
    assert!(
        !play(island(), false, true),
        "guessed 'not in hand' and was right: no draw"
    );
    assert!(
        play(island(), true, true),
        "guessed 'in hand' and was wrong: draw"
    );
}
