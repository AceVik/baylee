//! `cards/sorceries/mv_3/trip_wire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn trip_wire() -> CardIndex {
    card_index("5751df7a-aa9a-4a27-b7ab-445ec4bcbdee")
}

fn barbarian_general() -> CardIndex {
    card_index("d4f2e305-bec0-48c8-8040-f85618c71cc9")
}

/// Trip Wire (5751df7a-aa9a-4a27-b7ab-445ec4bcbdee) — {2}{G} Sorcery:
/// "Destroy target creature with horsemanship."
#[test]
fn trip_wire_destroys_target_horsemanship_creature_and_excludes_others() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), aurochs()],
        &[barbarian_general(), fangren_hunter()],
        &[trip_wire()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let rider = lk::obj(&engine, P1, barbarian_general());
    let hunter = lk::obj(&engine, P1, fangren_hunter());

    cast_from_hand(&mut engine, P0, trip_wire());
    let menu = aim_at(&mut engine, P0, rider);
    assert_eq!(menu, vec![rider], "only horsemanship creatures offered");
    assert!(!menu.contains(&my_beast) && !menu.contains(&hunter));
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, rider), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Battlefield);

    let mut no_horse = lk::game(
        SEED,
        &[forest(), forest(), forest()],
        &[fangren_hunter()],
        &[trip_wire()],
        &[],
    );
    tap_all_mana(&mut no_horse, P0);
    assert!(!lk::castable(&no_horse, P0, trip_wire()));
}
