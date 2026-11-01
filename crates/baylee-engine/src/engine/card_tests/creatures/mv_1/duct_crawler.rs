//! `cards/creatures/mv_1/duct_crawler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn duct_crawler() -> CardIndex {
    card_index("d47a7aa5-3b64-4ff8-bb10-e46a34d1993c")
}

/// Duct Crawler (`d47a7aa5-3b64-4ff8-bb10-e46a34d1993c`) — {R} 1/1 Creature — Insect (Implemented):
/// "{1}{R}: Target creature can't block this creature this turn."
#[test]
fn duct_crawler_prevents_target_creature_from_blocking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), duct_crawler()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let crawler = on_battlefield(&engine, P0, duct_crawler()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, duct_crawler(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crawler, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn restriction_is_one_blocker_one_attacker_and_one_turn() {
    let mut e = lk::game(
        SEED,
        &[duct_crawler(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let crawler = lk::obj(&e, P0, duct_crawler());
    let friend = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&e, P1, index::HILL_GIANT);
    let bear = lk::obj(&e, P1, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Red, 2)]);
    lk::use_ability(&mut e, P0, duct_crawler(), 0, &[giant], &[]);
    lk::attackers_offered(&mut e);
    let offer = lk::declare(&mut e, &[crawler, friend]);
    assert!(!lk::may_block(&offer, giant, crawler));
    assert!(lk::may_block(&offer, giant, friend));
    assert!(lk::may_block(&offer, bear, crawler));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    lk::attackers_offered(&mut e);
    let offer = lk::declare(&mut e, &[crawler]);
    assert!(lk::may_block(&offer, giant, crawler));
}
