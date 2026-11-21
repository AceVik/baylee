//! `cards/instants/mv_2/call_to_glory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b01d24d5-79d0-401f-99f5-da036628dd39"
fn call_to_glory() -> CardIndex {
    card_index("b01d24d5-79d0-401f-99f5-da036628dd39")
}

/// Call to Glory ({1}{W}): untaps ALL creatures you control (Samurai and
/// non-Samurai alike, leaving opponent's creatures tapped) and gives ONLY
/// Samurai creatures you control +1/+1 until end of turn.
#[test]
fn call_to_glory_untaps_all_your_creatures_and_pumps_only_your_samurai() {
    let mut engine = lk::game(
        SEED,
        &[index::DEVOTED_RETAINER, index::GRIZZLY_BEARS],
        &[index::DEVOTED_RETAINER],
        &[call_to_glory()],
        &[],
    );
    let my_samurai = lk::obj(&engine, P0, index::DEVOTED_RETAINER);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_samurai = lk::obj(&engine, P1, index::DEVOTED_RETAINER);

    // Attack with both of P0's creatures so both become tapped.
    let _ = lk::attack(&mut engine, &[my_samurai, my_bears]);
    lk::block(&mut engine, &[]);
    assert!(is_tapped(&engine, my_samurai) && is_tapped(&engine, my_bears));

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, call_to_glory());
    lk::settle(&mut engine);

    assert!(
        !is_tapped(&engine, my_samurai) && !is_tapped(&engine, my_bears),
        "all creatures you control untap"
    );
    assert_eq!(pt(&engine, my_samurai), (2, 2), "own Samurai gets +1/+1");
    assert_eq!(pt(&engine, my_bears), (2, 2), "non-Samurai gets no pump");
    assert_eq!(
        pt(&engine, their_samurai),
        (1, 1),
        "opponent Samurai untouched"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_samurai), (1, 1), "+1/+1 wears off at EOT");
}
