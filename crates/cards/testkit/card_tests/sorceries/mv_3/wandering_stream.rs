//! `cards/sorceries/mv_3/wandering_stream.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wandering_stream() -> CardIndex {
    card_index("1f592b83-7838-4466-8bcb-093ac7346c55")
}

/// Wandering Stream (1f592b83-7838-4466-8bcb-093ac7346c55) — {2}{G} Sorcery:
/// "Domain — You gain 2 life for each basic land type among lands you control."
#[test]
fn wandering_stream_gains_two_life_per_distinct_basic_land_type_you_control() {
    // Forest + Forest + Badlands (Swamp Mountain) + Plains + Abraded Bluffs
    // = 4 distinct basic land types (Forest, Swamp, Mountain, Plains) -> +8 life.
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), badlands(), plains(), abraded_bluffs()],
        &[island()],
        &[wandering_stream()],
        &[],
    );

    cast_from_hand(&mut engine, P0, wandering_stream());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 28, "4 basic land types -> +8 life");
    assert_eq!(lk::life(&engine, P1), 20);

    // Zero basic land types (nonbasic Desert lands only) -> +0 life.
    let mut zero = lk::game(
        SEED,
        &[
            abraded_bluffs(),
            abraded_bluffs(),
            desert_of_the_indomitable(),
        ],
        &[plains(), island(), swamp(), mountain(), forest()],
        &[wandering_stream()],
        &[],
    );
    lk::float(&mut zero, P0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut zero, P0, wandering_stream());
    lk::settle(&mut zero);
    assert_eq!(lk::life(&zero, P0), 20);
}
