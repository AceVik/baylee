//! Oracle scenarios for Phantom Whelp.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn phantom_whelp() -> CardIndex {
    card_index("9b020aeb-7b27-43f2-9767-d93dae3d102a")
}

#[test]
fn phantom_whelp_returns_to_hand_at_end_of_combat_after_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(0, &[index::ISLAND, index::ISLAND])
        .hand(0, &[phantom_whelp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, phantom_whelp());
    pass_until(&mut e, stack_is_empty);

    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));

    let whelp = on_battlefield(&e, p0, phantom_whelp()).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(whelp, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(e.state().players[1].life, 18);
    assert!(on_battlefield(&e, p0, phantom_whelp()).is_none());
    assert!(in_hand(&e, p0, phantom_whelp()).is_some());
}
