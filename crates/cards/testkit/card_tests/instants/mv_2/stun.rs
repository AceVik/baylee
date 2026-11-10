//! `cards/instants/mv_2/stun.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn stun() -> CardIndex {
    card_index("d3491972-44c5-4962-a680-42b79357a189")
}

/// `Stun` ({1}{R}, Instant, `Coverage::Implemented`):
/// `Target creature can't block this turn.`
/// `Draw a card.`
#[test]
fn stun_makes_target_creature_unable_to_block_this_turn_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[stun()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, stun());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::CANT_BLOCK)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::CANT_BLOCK)
    );
}
