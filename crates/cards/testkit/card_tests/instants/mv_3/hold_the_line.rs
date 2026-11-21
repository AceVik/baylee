//! `cards/instants/mv_3/hold_the_line.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1ff55bbd-cde2-4fe9-a129-16790831d665"
fn hold_the_line() -> CardIndex {
    card_index("1ff55bbd-cde2-4fe9-a129-16790831d665")
}

/// Hold the Line ({1}{W}{W}): gives ONLY blocking creatures +7/+7 until end of
/// turn (leaving attackers and non-blocking creatures unchanged), and wears
/// off next turn.
#[test]
fn hold_the_line_pumps_only_blocking_creatures_by_seven_until_eot() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                index::LLANOWAR_ELVES,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[hold_the_line()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(elf, giant)]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, hold_the_line());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (8, 8), "blocking Elf gets +7/+7");
    assert_eq!(pt(&engine, bears), (2, 2), "non-blocking Bears untouched");
    assert_eq!(pt(&engine, giant), (3, 3), "attacking Giant untouched");

    lk::through_combat(&mut engine);
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert_eq!(
        on_battlefield(&engine, P0, index::LLANOWAR_ELVES),
        Some(elf)
    );

    reach_their_main_phase(&mut engine, P0);
    assert_eq!(pt(&engine, elf), (1, 1), "+7/+7 wears off at end of turn");
}
