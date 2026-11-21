//! `cards/instants/mv_2/kjeldoran_war_cry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e45a1626-d659-4eda-bc6b-bd8119ce7386"
fn kjeldoran_war_cry() -> CardIndex {
    card_index("e45a1626-d659-4eda-bc6b-bd8119ce7386")
}

/// Kjeldoran War Cry ({1}{W}): creatures YOU control get +X/+X until EOT where
/// X is 1 plus the number of Kjeldoran War Cry cards in ALL graveyards (not
/// counting the resolving spell itself).
#[test]
fn kjeldoran_war_cry_counts_copies_in_all_graveyards_and_pumps_yours_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[kjeldoran_war_cry(), kjeldoran_war_cry()],
        &[kjeldoran_war_cry()],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // 1. 0 copies in graveyards -> X = 1 (+1/+1 -> 3/3); opponent's stays 2/2.
    lk::float(&mut engine, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut engine, P0, kjeldoran_war_cry());
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // 2. Now 1 in P0's GY + put 1 in P1's GY -> 2 in all GYs -> X = 3 (+3/+3 -> 6/6).
    hand_to_graveyard(&mut engine, P1, kjeldoran_war_cry());
    cast_with_floating(&mut engine, P0, kjeldoran_war_cry());
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, my_bears), (6, 6), "3/3 + 3/3 = 6/6");
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // Wears off at end of turn.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_bears), (2, 2));
}
