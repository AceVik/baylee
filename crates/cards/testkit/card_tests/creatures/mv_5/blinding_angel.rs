//! `cards/creatures/mv_5/blinding_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn blinding_angel() -> CardIndex {
    card_index("cb479ce1-59e8-4531-9a21-f7c97d334817")
}

#[test]
fn blinding_angel_combat_damage_skips_exactly_the_next_combat_phase() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[blinding_angel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let angel = on_battlefield(&e, p0, blinding_angel()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(angel, Defender::Player(p1))],
        },
    )
    .expect("the Angel attacks");
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
    });
    assert_eq!(
        e.state().players[1].life,
        18,
        "combat damage triggered the ability"
    );

    reach_their_main_phase(&mut e, p1);
    let mut skipped = false;
    for _ in 0..40 {
        if e.state().turn.active == p1 && e.state().turn.phase == Phase::SecondMain {
            skipped = true;
            break;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { .. } => panic!("the next combat phase was not skipped"),
            other => panic!("unexpected skipped-combat question: {other:?}"),
        }
    }
    assert!(skipped, "first main advanced directly to second main");

    reach_their_main_phase(&mut e, p0);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    assert!(
        e.state().turn.active == p1,
        "the following combat is normal"
    );
}
