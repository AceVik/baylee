//! `cards/sorceries/mv_3/dust_to_dust.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dust_to_dust() -> CardIndex {
    card_index("b17eb586-0272-4d02-be75-de06ea044b6f")
}

fn bone_flute() -> CardIndex {
    card_index("8664a004-d786-4357-8856-2f8f5a9ae24d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Seat 0 holds Dust to Dust with {1}{W}{W} floating; the boards are as
/// given.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .hand(0, &[dust_to_dust()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    engine
}

/// Dust to Dust (b17eb586-0272-4d02-be75-de06ea044b6f) — "Exile two target
/// artifacts." Two of the opponent's three artifacts are exiled (the card
/// goes to exile, not the graveyard); the third and a non-artifact stay.
#[test]
fn dust_to_dust_exiles_two_target_artifacts() {
    let mut engine = board(&[grizzly_bears()], &[sol_ring(), bone_flute(), sol_ring()]);
    let rings: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P1 && o.card.is_some_and(|c| c.index == sol_ring())
            })
        })
        .collect();
    assert_eq!(rings.len(), 2);
    let flute = on_battlefield(&engine, P1, bone_flute()).expect("the Flute");

    lk::announce(
        &mut engine,
        P0,
        dust_to_dust(),
        0,
        0,
        &[rings[0], flute],
        &[],
    );
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, rings[0]), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, flute), Zone::Exile);
    assert_eq!(
        lk::zone_of(&engine, rings[1]),
        Zone::Battlefield,
        "the unnamed artifact stays"
    );
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, P0, dust_to_dust()).is_some());
}

/// Dust to Dust — it targets any artifacts, mine included, and offers no
/// creature that is not one.
#[test]
fn dust_to_dust_offers_artifacts_on_both_sides_and_nothing_else() {
    let mut engine = board(&[sol_ring(), grizzly_bears()], &[bone_flute()]);
    let mine = on_battlefield(&engine, P0, sol_ring()).expect("my Sol Ring");
    let theirs = on_battlefield(&engine, P1, bone_flute()).expect("their Flute");
    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("my Bears");

    cast_with_floating(&mut engine, P0, dust_to_dust());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2), "exactly two targets");
    assert!(options.contains(&mine) && options.contains(&theirs));
    assert!(!options.contains(&bears), "a creature that is no artifact");

    lk::aim(&mut engine, &[mine, theirs], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, mine), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, theirs), Zone::Exile);
}

/// Dust to Dust — "two target artifacts": one artifact in play is not enough
/// to cast it.
#[test]
fn dust_to_dust_needs_two_artifacts_to_be_cast() {
    let engine = board(&[grizzly_bears()], &[bone_flute()]);
    assert!(!lk::castable(&engine, P0, dust_to_dust()));
}
