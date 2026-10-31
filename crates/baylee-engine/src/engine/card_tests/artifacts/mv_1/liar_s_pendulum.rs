//! Liar's Pendulum draws only after a reveal and a wrong guess.

use super::*;

fn liar_s_pendulum() -> CardIndex {
    card_index("4f88ddfa-5478-426a-a978-dfc64c687db1")
}

fn play(guess_in_hand: bool, reveal: bool) -> Engine<RegistryLookup> {
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
            card: forest(),
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
    let expected = if reveal && !guess_in_hand {
        before - 1
    } else {
        before
    };
    assert_eq!(library_size(&e, p0), expected);
    e
}

#[test]
fn liar_s_pendulum_draws_only_when_reveal_proves_the_guess_wrong() {
    play(false, true);
    play(true, true);
    play(false, false);
}
