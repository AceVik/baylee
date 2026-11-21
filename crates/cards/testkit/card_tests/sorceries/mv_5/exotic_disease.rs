//! `cards/sorceries/mv_5/exotic_disease.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn exotic_disease() -> CardIndex {
    card_index("9480e142-289a-439d-b6a5-2666fb0e5be3")
}

/// Exotic Disease (9480e142-289a-439d-b6a5-2666fb0e5be3) — {4}{B} Sorcery:
/// "Domain — Target player loses X life and you gain X life, where X is the
/// number of basic land types among lands you control."
#[test]
fn exotic_disease_drains_target_player_for_your_distinct_basic_land_types() {
    // Swamp + Swamp + Badlands (Swamp Mountain) + Island + Forest + Abraded Bluffs
    // = 4 distinct basic land types (Swamp, Mountain, Island, Forest); P1's Plains ignored.
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            badlands(),
            island(),
            forest(),
            abraded_bluffs(),
        ],
        &[plains()],
        &[exotic_disease()],
        &[],
    );

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, exotic_disease(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 16, "target lost 4 life");
    assert_eq!(lk::life(&engine, P0), 24, "caster gained 4 life");
}
