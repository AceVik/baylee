//! `cards/instants/mv_3/keep_watch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn keep_watch_draws_a_card_for_each_attacking_creature() {
    let keep_watch = card_index("847b5be5-cb0c-4857-bcd9-c5cfe55fdaeb");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), llanowar_elves()])
        .hand(0, &[keep_watch])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0)), (ogre, Defender::Player(p0))],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, keep_watch);
    pass_until(&mut engine, stack_is_empty);

    // Spent 1 card (Keep Watch), drew 2 cards for the 2 attacking creatures -> net +1 card in hand.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
