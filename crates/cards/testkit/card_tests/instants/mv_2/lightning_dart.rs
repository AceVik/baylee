//! `cards/instants/mv_2/lightning_dart.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lightning_dart() -> CardIndex {
    card_index("7bc53740-86e7-4e9d-aab6-b5ae62a42133")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Lightning Dart ({1}{R} Instant):
/// "Lightning Dart deals 1 damage to target creature. If that creature is white
/// or blue, Lightning Dart deals 4 damage to it instead."
#[test]
fn lightning_dart_deals_four_to_white_or_blue_and_one_to_others() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[serra_angel(), grizzly_bears(), hill_giant()],
        &[lightning_dart(), lightning_dart()],
        &[],
    );
    let angel = lk::obj(&engine, P1, serra_angel());
    let bears = lk::obj(&engine, P1, grizzly_bears());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, lightning_dart());
    lk::aim(&mut engine, &[angel], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P1, serra_angel()).is_some(),
        "4 damage killed 4/4 white creature"
    );

    cast_with_floating(&mut engine, P0, lightning_dart());
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        lk::damage_on(&engine, bears),
        1,
        "1 damage to green creature"
    );
}
