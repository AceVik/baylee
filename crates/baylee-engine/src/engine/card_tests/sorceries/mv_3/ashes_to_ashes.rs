//! `cards/sorceries/mv_3/ashes_to_ashes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ashes_to_ashes() -> CardIndex {
    card_index("944a52d9-bb14-43c5-8d05-2afaf023dc9f")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Seat 0 holds Ashes to Ashes with {1}{B}{B} floating; the boards are as
/// given.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .hand(0, &[ashes_to_ashes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 3)]);
    engine
}

/// Ashes to Ashes (944a52d9-bb14-43c5-8d05-2afaf023dc9f) — "Exile two target
/// nonartifact creatures. Ashes to Ashes deals 5 damage to you." Two of the
/// opponent's creatures are exiled (not put in the graveyard), the third
/// stays, and the caster takes 5.
#[test]
fn ashes_to_ashes_exiles_two_creatures_and_hurts_its_caster() {
    let mut engine = board(&[], &[hill_giant(), pearled_unicorn(), grizzly_bears()]);
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("Giant");
    let unicorn = on_battlefield(&engine, P1, pearled_unicorn()).expect("Unicorn");

    lk::announce(
        &mut engine,
        P0,
        ashes_to_ashes(),
        0,
        0,
        &[giant, unicorn],
        &[],
    );
    assert_eq!(lk::life(&engine, P0), 20, "nothing before it resolves");
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, giant), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, unicorn), Zone::Exile);
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "an unnamed creature stays"
    );
    assert_eq!(lk::life(&engine, P0), 15, "5 damage to the caster");
    assert_eq!(lk::life(&engine, P1), 20, "none to the opponent");
}

/// Ashes to Ashes — "nonartifact": an artifact creature is not a legal
/// target, a creature of mine is.
#[test]
fn ashes_to_ashes_does_not_offer_artifact_creatures() {
    let mut engine = board(
        &[grizzly_bears(), ornithopter()],
        &[hill_giant(), ornithopter()],
    );
    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).expect("my Bears");
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("Giant");
    let my_thopter = on_battlefield(&engine, P0, ornithopter()).expect("my Ornithopter");
    let their_thopter = on_battlefield(&engine, P1, ornithopter()).expect("their Ornithopter");

    cast_with_floating(&mut engine, P0, ashes_to_ashes());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2), "exactly two targets");
    assert!(options.contains(&my_bears) && options.contains(&giant));
    assert!(!options.contains(&my_thopter), "an artifact creature");
    assert!(!options.contains(&their_thopter), "an artifact creature");

    lk::aim(&mut engine, &[my_bears, giant], &[]);
    lk::settle(&mut engine);
    assert_eq!(
        lk::zone_of(&engine, my_bears),
        Zone::Exile,
        "mine are fair game"
    );
    assert_eq!(lk::zone_of(&engine, giant), Zone::Exile);
    assert_eq!(lk::life(&engine, P0), 15);
}

/// Ashes to Ashes — "two target": with a single nonartifact creature in play
/// there are not two targets, so it cannot be cast.
#[test]
fn ashes_to_ashes_needs_two_targets_to_be_cast() {
    let engine = board(&[ornithopter()], &[hill_giant()]);
    assert!(
        !lk::castable(&engine, P0, ashes_to_ashes()),
        "one nonartifact creature is not two targets"
    );
}
