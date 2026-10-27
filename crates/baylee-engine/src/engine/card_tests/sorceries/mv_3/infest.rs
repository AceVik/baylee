//! `cards/sorceries/mv_3/infest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Infest (`d6850616-7db5-4141-9ab0-ae8d1f08114f`) — {1}{B}{B} Sorcery:
/// "All creatures get -2/-2 until end of turn."
#[test]
fn infest_gives_all_creatures_minus_two_minus_two_until_end_of_turn() {
    let infest_card = card_index("d6850616-7db5-4141-9ab0-ae8d1f08114f");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[infest_card])
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[index::EAGER_CADET])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, infest_card);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, index::EAGER_CADET).is_some());
    assert_eq!(pt(&engine, giant), (1, 1));
}
