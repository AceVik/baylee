//! `cards/instants/mv_2/feral_instinct.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn feral_instinct() -> CardIndex {
    card_index("f8f3fc11-25b4-4a23-9d79-f380a14f0420")
}

#[test]
fn feral_instinct_pumps_creature_and_draws_a_card_at_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), grizzly_bears()])
        .hand(0, &[feral_instinct()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, feral_instinct());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (3, 3));
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let opponent_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 1,
        "the caster draws at the next turn's upkeep, the opponent's"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        opponent_hand + 1,
        "the opponent draws only its own draw step's card"
    );
    assert_eq!(pt(&engine, bear), (2, 2), "the +1/+1 ended with the turn");

    // Once: the caster's own next turn draws the draw step's card and no other.
    let hand_at_their_main = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_at_their_main + 1,
        "the delayed draw does not come back on a later upkeep"
    );
}
