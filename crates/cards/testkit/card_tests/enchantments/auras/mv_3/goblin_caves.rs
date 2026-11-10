//! `cards/enchantments/auras/mv_3/goblin_caves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).expect("the spell is in hand");
    e.apply(p, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn goblin_caves() -> CardIndex {
    card_index("3ab5c3a7-12e7-4394-a495-0bc7c310bf9c")
}

fn goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Goblin Caves cast on `land` (one of P0's), with Goblins on both sides and a
/// Bear. Returns the engine, P0's Goblin, P1's Goblin and the Bear.
fn caves_on(land: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut e = prepared(
        &[goblin_caves()],
        &[land, goblin_raiders(), grizzly_bears()],
        &[goblin_raiders()],
    );
    let mine = on_battlefield(&e, P0, goblin_raiders()).expect("P0's Goblin is out");
    let theirs = on_battlefield(&e, P1, goblin_raiders()).expect("P1's Goblin is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let target = on_battlefield(&e, P0, land).expect("the land is out");
    assert_eq!(pt(&e, mine), (1, 1));

    cast(&mut e, P0, goblin_caves());
    targets(&mut e, vec![target]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, goblin_caves()).is_some(),
        "the Aura is out"
    );
    (e, mine, theirs, bears)
}

/// Goblin Caves (3ab5c3a7-12e7-4394-a495-0bc7c310bf9c) — "Enchant land. As long
/// as enchanted land is a basic Mountain, Goblin creatures get +0/+2."
#[test]
fn goblin_caves_on_a_basic_mountain_gives_every_goblin_plus_zero_plus_two() {
    let (e, mine, theirs, bears) = caves_on(mountain());
    assert_eq!(pt(&e, mine), (1, 3), "P0's Goblin");
    assert_eq!(pt(&e, theirs), (1, 3), "and P1's: all Goblin creatures");
    assert_eq!(pt(&e, bears), (2, 2), "a Bear is no Goblin");
}

/// Goblin Caves — (not a Mountain)
#[test]
fn goblin_caves_on_a_forest_does_nothing() {
    let (e, mine, theirs, bears) = caves_on(forest());
    assert_eq!(pt(&e, mine), (1, 1));
    assert_eq!(pt(&e, theirs), (1, 1));
    assert_eq!(pt(&e, bears), (2, 2));
}

/// Goblin Caves — (a Mountain that is not basic)
#[test]
fn goblin_caves_on_a_nonbasic_mountain_does_nothing() {
    let (e, mine, theirs, _bears) = caves_on(badlands());
    assert_eq!(
        pt(&e, mine),
        (1, 1),
        "Badlands is a Mountain, but not a basic one"
    );
    assert_eq!(pt(&e, theirs), (1, 1));
}
