//! `cards/creatures/mv_3/norwood_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn norwood_warrior() -> CardIndex {
    card_index("b3ae1481-9fa7-45dd-bbf5-de2d815c4227")
}

/// Norwood Warrior ({2}{G} 2/2 Elf Warrior):
/// "Whenever this creature becomes blocked, it gets +1/+1 until end of turn."
#[test]
fn norwood_warrior_gets_plus_one_plus_one_once_when_blocked_and_not_when_unblocked() {
    let mut e = lk::game(
        SEED,
        &[norwood_warrior()],
        &[llanowar_elves(), llanowar_elves()],
        &[],
        &[],
    );
    let warrior = lk::obj(&e, lk::P0, norwood_warrior());
    let elves: Vec<ObjectId> = e
        .state()
        .battlefield_seen()
        .filter(|&id| e.state().object(id).unwrap().controller == lk::P1)
        .collect();

    // Unblocked: stays 2/2 and deals 2 damage.
    let _ = lk::attack(&mut e, &[warrior]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, warrior), (2, 2));
    assert_eq!(lk::life(&e, lk::P1), 18);

    // Double-blocked next turn: triggers once (+1/+1 -> 3/3), kills both 1/1s, survives.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[warrior]);
    lk::block(&mut e, &[(elves[0], warrior), (elves[1], warrior)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, warrior), (3, 3));

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseNumber { .. })
            || eng.state().turn.phase == Phase::SecondMain
    });
    if let Pending::ChooseNumber { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    }
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, norwood_warrior()).is_some());
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, warrior), (2, 2));
}
