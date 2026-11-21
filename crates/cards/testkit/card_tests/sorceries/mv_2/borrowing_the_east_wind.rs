//! `cards/sorceries/mv_2/borrowing_the_east_wind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn borrowing_the_east_wind() -> CardIndex {
    card_index("79334732-8db1-4393-9fed-3cde3c572c5c")
}

fn barbarian_general() -> CardIndex {
    card_index("d4f2e305-bec0-48c8-8040-f85618c71cc9")
}

/// Borrowing the East Wind (79334732-8db1-4393-9fed-3cde3c572c5c) — {X}{G}{G}:
/// "Borrowing the East Wind deals X damage to each creature with horsemanship
/// and each player."
#[test]
fn borrowing_the_east_wind_hits_only_horsemanship_creatures_and_each_player() {
    let mut engine = lk::game(
        SEED,
        &[
            forest(),
            forest(),
            forest(),
            forest(),
            barbarian_general(),
            llanowar_elves(),
        ],
        &[barbarian_general(), aurochs()],
        &[borrowing_the_east_wind()],
        &[],
    );
    let my_rider = lk::obj(&engine, P0, barbarian_general());
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let their_rider = lk::obj(&engine, P1, barbarian_general());
    let their_beast = lk::obj(&engine, P1, aurochs());

    tap_mana_except(&mut engine, P0, my_elf);
    lk::announce(&mut engine, P0, borrowing_the_east_wind(), 0, 2, &[], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, my_rider), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_rider), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, my_elf), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, my_elf), 0, "no horsemanship");
    assert_eq!(lk::damage_on(&engine, their_beast), 0);
    assert_eq!(lk::life(&engine, P0), 18);
    assert_eq!(lk::life(&engine, P1), 18);
}
