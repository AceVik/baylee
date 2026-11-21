//! `cards/sorceries/mv_1/rolling_earthquake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rolling_earthquake() -> CardIndex {
    card_index("46b96552-830e-4732-9560-9872aeec858b")
}

fn barbarian_general() -> CardIndex {
    card_index("d4f2e305-bec0-48c8-8040-f85618c71cc9")
}

/// Rolling Earthquake (46b96552-830e-4732-9560-9872aeec858b) — {X}{R} Sorcery:
/// "Rolling Earthquake deals X damage to each creature without horsemanship and
/// each player."
#[test]
fn rolling_earthquake_damages_non_horsemanship_creatures_and_each_player() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            barbarian_general(),
            aurochs(),
        ],
        &[fangren_hunter(), barbarian_general()],
        &[rolling_earthquake()],
        &[],
    );
    let my_rider = lk::obj(&engine, P0, barbarian_general());
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_hunter = lk::obj(&engine, P1, fangren_hunter());
    let their_rider = lk::obj(&engine, P1, barbarian_general());

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, rolling_earthquake(), 0, 3, &[], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, my_rider), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, my_rider), 0, "horsemanship spared");
    assert_eq!(lk::zone_of(&engine, their_rider), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, their_rider), 0);
    assert_eq!(lk::zone_of(&engine, my_beast), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_hunter), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, their_hunter), 3);
    assert_eq!(lk::life(&engine, P0), 17);
    assert_eq!(lk::life(&engine, P1), 17);
}
