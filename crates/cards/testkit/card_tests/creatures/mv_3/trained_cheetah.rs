//! `cards/creatures/mv_3/trained_cheetah.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn trained_cheetah() -> CardIndex {
    card_index("dad3b459-725e-4bf5-bf7a-bd89866ea8c4")
}

/// Trained Cheetah ({2}{G} 2/2 Cat):
/// "Whenever this creature becomes blocked, it gets +1/+1 until end of turn."
#[test]
fn trained_cheetah_gets_plus_one_plus_one_once_when_blocked_and_not_when_unblocked() {
    let mut e = lk::game(
        SEED,
        &[trained_cheetah()],
        &[llanowar_elves(), llanowar_elves()],
        &[],
        &[],
    );
    let cheetah = lk::obj(&e, lk::P0, trained_cheetah());
    let elves: Vec<ObjectId> = e
        .state()
        .battlefield_seen()
        .filter(|&id| e.state().object(id).unwrap().controller == lk::P1)
        .collect();

    let _ = lk::attack(&mut e, &[cheetah]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, cheetah), (2, 2));
    assert_eq!(lk::life(&e, lk::P1), 18);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[cheetah]);
    lk::block(&mut e, &[(elves[0], cheetah), (elves[1], cheetah)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, cheetah), (3, 3));

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseNumber { .. })
            || eng.state().turn.phase == Phase::SecondMain
    });
    if let Pending::ChooseNumber { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    }
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, trained_cheetah()).is_some());
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, cheetah), (2, 2));
}
