//! `cards/creatures/mv_3/black_widow_deadly_hunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn black_widow_draws_once_for_each_deathtouch_creature_that_hits_a_player() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::BLACK_WIDOW_DEADLY_HUNTER,
                index::TYPHOID_RATS,
                index::HILL_GIANT,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let widow = on_battlefield(&e, P0, index::BLACK_WIDOW_DEADLY_HUNTER).unwrap();
    let rats = on_battlefield(&e, P0, index::TYPHOID_RATS).unwrap();
    let giant = on_battlefield(&e, P0, index::HILL_GIANT).unwrap();
    let hand = hand_size(&e, P0);
    let library = library_size(&e, P0);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (widow, Defender::Player(P1)),
                (rats, Defender::Player(P1)),
                (giant, Defender::Player(P1)),
            ],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(e.state().players[1].life, 13, "3 + 1 + 3 combat damage");
    assert_eq!(hand_size(&e, P0), hand + 2);
    assert_eq!(library_size(&e, P0), library - 2);
    assert_eq!(e.state().players[0].life, 18, "one life lost per trigger");
}

#[test]
fn black_widows_deathtouch_makes_her_nonlethal_damage_lethal() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::BLACK_WIDOW_DEADLY_HUNTER])
        .battlefield(1, &[index::ROOTBREAKER_WURM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let widow = on_battlefield(&e, P0, index::BLACK_WIDOW_DEADLY_HUNTER).unwrap();
    let wurm = on_battlefield(&e, P1, index::ROOTBREAKER_WURM).unwrap();
    let hand = hand_size(&e, P0);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(widow, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wurm, widow)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&e, P1, index::ROOTBREAKER_WURM).is_some());
    assert!(in_graveyard(&e, P0, index::BLACK_WIDOW_DEADLY_HUNTER).is_some());
    assert_eq!(e.state().players[1].life, 20);
    assert_eq!(
        hand_size(&e, P0),
        hand,
        "combat damage to a creature does not draw"
    );
}
