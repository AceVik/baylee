#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn iron_maiden() -> CardIndex {
    card_index("e0351638-9224-4779-8f7c-f2b11d134a5f")
}

#[test]
fn iron_maiden_deals_hand_size_minus_four_on_opponent_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[iron_maiden()])
        .hand(
            1,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, iron_maiden());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn iron_maiden_only_damages_opponents_and_floors_at_zero() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for cards in [0, 4, 5] {
        let hand = vec![plains(); cards];
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[iron_maiden()])
            .hand(
                0,
                &[plains(), plains(), plains(), plains(), plains(), plains()],
            )
            .hand(1, &hand)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        assert_eq!(lk::life(&engine, p0), 20, "no damage on controller upkeep");
        reach_their_main_phase(&mut engine, p1);
        assert_eq!(lk::life(&engine, p1), if cards == 5 { 19 } else { 20 });
    }
}
