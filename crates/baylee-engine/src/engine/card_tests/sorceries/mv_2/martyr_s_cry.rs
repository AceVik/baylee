//! `cards/sorceries/mv_2/martyr_s_cry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn martyr_s_cry() -> CardIndex {
    card_index("b636f17d-b54a-4a72-9a5e-ce13e7b4f00a")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

/// Seat 0 holds Martyr's Cry with two Plains; the boards are as given.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut lands = vec![plains(), plains()];
    lands.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .battlefield(1, theirs)
        .hand(0, &[martyr_s_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Martyr's Cry (b636f17d-b54a-4a72-9a5e-ce13e7b4f00a) — "Exile all white
/// creatures. For each creature exiled this way, its controller draws a
/// card." Both sides lose their white creatures to exile (not the graveyard),
/// keep the others, and each draws one card per creature they lost.
#[test]
fn martyr_s_cry_exiles_all_white_creatures_and_each_controller_draws_per_creature() {
    let mut engine = board(
        &[savannah_lions(), grizzly_bears()],
        &[serra_angel(), pearled_unicorn(), ornithopter()],
    );
    let lions = on_battlefield(&engine, P0, savannah_lions()).expect("Lions");
    let angel = on_battlefield(&engine, P1, serra_angel()).expect("Angel");
    let unicorn = on_battlefield(&engine, P1, pearled_unicorn()).expect("Unicorn");
    let (mine, theirs) = (library_size(&engine, P0), library_size(&engine, P1));

    cast_from_hand(&mut engine, P0, martyr_s_cry());
    lk::settle(&mut engine);

    for white in [lions, angel, unicorn] {
        assert_eq!(
            lk::zone_of(&engine, white),
            Zone::Exile,
            "exiled, not destroyed"
        );
    }
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "green stays"
    );
    assert!(
        on_battlefield(&engine, P1, ornithopter()).is_some(),
        "colorless stays"
    );
    assert_eq!(
        library_size(&engine, P0),
        mine - 1,
        "one white creature, one card"
    );
    assert_eq!(
        library_size(&engine, P1),
        theirs - 2,
        "two white creatures, two cards"
    );
}

/// Martyr's Cry — with no white creature in play it exiles nothing and
/// nobody draws.
#[test]
fn martyr_s_cry_with_no_white_creature_does_nothing() {
    let mut engine = board(&[grizzly_bears()], &[ornithopter()]);
    let (mine, theirs) = (library_size(&engine, P0), library_size(&engine, P1));

    cast_from_hand(&mut engine, P0, martyr_s_cry());
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P1, ornithopter()).is_some());
    assert_eq!(library_size(&engine, P0), mine);
    assert_eq!(library_size(&engine, P1), theirs);
    assert!(in_graveyard(&engine, P0, martyr_s_cry()).is_some());
}

/// Martyr's Cry — "its controller draws": a white creature seat 0 owns but
/// seat 1 controls goes to its owner's exile, and seat 1 is the one who
/// draws.
#[test]
fn martyr_s_cry_the_controller_draws_and_the_owner_gets_the_exiled_card() {
    let mut engine = board(&[savannah_lions()], &[grizzly_bears()]);
    let lions = on_battlefield(&engine, P0, savannah_lions()).expect("Lions");
    engine
        .dev_state_mut(P0)
        .expect("the harness may set boards up")
        .object_mut(lions)
        .expect("the Lions exist")
        .set_controller(P1);
    engine.refresh_offer();
    let (mine, theirs) = (library_size(&engine, P0), library_size(&engine, P1));

    cast_from_hand(&mut engine, P0, martyr_s_cry());
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, lions), Zone::Exile);
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 1);
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P1)).len(), 0);
    assert_eq!(library_size(&engine, P0), mine, "the owner does not draw");
    assert_eq!(library_size(&engine, P1), theirs - 1, "the controller does");
}
