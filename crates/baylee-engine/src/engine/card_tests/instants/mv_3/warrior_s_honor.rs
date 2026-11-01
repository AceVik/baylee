//! `cards/instants/mv_3/warrior_s_honor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn warrior_s_honor() -> CardIndex {
    card_index("c3c9da49-ef4f-48c7-99c8-0f226b485285")
}

/// `Warrior's Honor` ({2}{W}, Instant, `Coverage::Implemented`) gives creatures
/// you control +1/+1 until end of turn.
#[test]
fn warrior_s_honor_pumps_creatures_you_control_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                GRIZZLY_BEARS,
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[warrior_s_honor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let opp = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, warrior_s_honor());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (3, 3));
    assert_eq!(pt(&engine, elf), (2, 2));
    assert_eq!(pt(&engine, opp), (3, 3));
}

/// "Until end of turn": the bonus is gone when the next turn begins, and the
/// opposing creature never had it.
#[test]
fn warrior_s_honor_wears_off_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[warrior_s_honor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, warrior_s_honor());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (3, 3));
    assert_eq!(
        pt(&engine, giant),
        (3, 3),
        "the opponent's creature is not ours"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bear), (2, 2), "the +1/+1 ended with the turn");
}
