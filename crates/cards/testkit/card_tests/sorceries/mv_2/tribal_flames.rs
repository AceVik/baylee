//! `cards/sorceries/mv_2/tribal_flames.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tribal_flames() -> CardIndex {
    card_index("c82eda61-d195-49c9-8ff0-a78dfa650689")
}

/// Tribal Flames (c82eda61-d195-49c9-8ff0-a78dfa650689) — {1}{R} Sorcery:
/// "Domain — Tribal Flames deals X damage to any target, where X is the number
/// of basic land types among lands you control."
#[test]
fn tribal_flames_deals_damage_equal_to_your_distinct_basic_land_types() {
    // Badlands (Swamp Mountain) + Mountain + Island + Abraded Bluffs (no basic land type)
    // = 3 distinct basic land types (Swamp, Mountain, Island); P1's Plains + Forest do not count.
    let mut engine = lk::game(
        SEED,
        &[badlands(), mountain(), island(), abraded_bluffs()],
        &[plains(), forest(), fangren_hunter()],
        &[tribal_flames()],
        &[],
    );

    tap_all_mana(&mut engine, P0);
    lk::cast_at(&mut engine, P0, tribal_flames(), &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P1),
        17,
        "3 distinct basic land types -> 3"
    );

    let mut vs_creature = lk::game(
        SEED,
        &[mountain(), swamp(), island(), forest(), plains()],
        &[fangren_hunter()],
        &[tribal_flames()],
        &[],
    );
    let hunter = lk::obj(&vs_creature, P1, fangren_hunter());
    tap_all_mana(&mut vs_creature, P0);
    lk::cast_at(&mut vs_creature, P0, tribal_flames(), &[hunter], &[]);
    assert_eq!(
        lk::zone_of(&vs_creature, hunter),
        Zone::Graveyard,
        "5 basic land types kills a 4/4"
    );
}
