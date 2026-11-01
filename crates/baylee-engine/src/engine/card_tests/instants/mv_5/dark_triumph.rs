//! `cards/instants/mv_5/dark_triumph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Dark Triumph ({4}{B} Instant):
/// "Creatures you control get +2/+0 until end of turn."
#[test]
fn dark_triumph_gives_creatures_you_control_plus_two_plus_zero_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), grizzly_bears()],
        )
        .hand(0, &[card_index("885630a8-c43b-4804-acd6-14a7c7321d73")])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    cast_from_hand(
        &mut engine,
        p0,
        card_index("885630a8-c43b-4804-acd6-14a7c7321d73"),
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 2));
    assert_eq!(pt(&engine, giant), (3, 3));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2));
}
