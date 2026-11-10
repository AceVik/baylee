//! `cards/enchantments/auras/mv_3/goblin_shrine.rs`, played.

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

fn goblin_shrine() -> CardIndex {
    card_index("5e35df8a-2404-4b3b-888a-25d663bd4383")
}

fn goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn goblin_chirurgeon() -> CardIndex {
    card_index("ea55db87-a5c3-49f7-b968-77510cdeb469")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Goblin Shrine cast on `land` (one of P0's), with a Goblin Chirurgeon (0/2)
/// and a Bear for P0 and a Goblin Raiders (1/1) for P1.
fn shrine_on(
    land: CardIndex,
) -> (
    Engine<RegistryLookup>,
    ObjectId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let mut e = prepared(
        &[goblin_shrine()],
        &[land, goblin_chirurgeon(), grizzly_bears()],
        &[goblin_raiders()],
    );
    let chirurgeon = on_battlefield(&e, P0, goblin_chirurgeon()).expect("Chirurgeon is out");
    let raiders = on_battlefield(&e, P1, goblin_raiders()).expect("Raiders are out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let target = on_battlefield(&e, P0, land).expect("the land is out");
    assert_eq!(pt(&e, chirurgeon), (0, 2));

    cast(&mut e, P0, goblin_shrine());
    targets(&mut e, vec![target]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, goblin_shrine()).is_some(),
        "the Aura is out"
    );
    (e, chirurgeon, raiders, bears, target)
}

/// Goblin Shrine (5e35df8a-2404-4b3b-888a-25d663bd4383) — "Enchant land. As long
/// as enchanted land is a basic Mountain, Goblin creatures get +1/+0."
#[test]
fn goblin_shrine_on_a_basic_mountain_gives_every_goblin_plus_one_plus_zero() {
    let (e, chirurgeon, raiders, bears, _land) = shrine_on(mountain());
    assert_eq!(pt(&e, chirurgeon), (1, 2), "P0's Goblin");
    assert_eq!(pt(&e, raiders), (2, 1), "and P1's");
    assert_eq!(pt(&e, bears), (2, 2), "a Bear is no Goblin");
}

/// Goblin Shrine — (not a basic Mountain)
#[test]
fn goblin_shrine_on_a_forest_or_a_nonbasic_mountain_does_nothing() {
    for land in [forest(), badlands()] {
        let (e, chirurgeon, raiders, _bears, _land) = shrine_on(land);
        assert_eq!(pt(&e, chirurgeon), (0, 2));
        assert_eq!(pt(&e, raiders), (1, 1));
    }
}

/// Goblin Shrine — "When this Aura leaves the battlefield, it deals 1 damage
/// to each Goblin creature."
#[test]
fn goblin_shrine_leaving_deals_one_damage_to_each_goblin() {
    let (mut e, chirurgeon, _raiders, bears, land) = shrine_on(mountain());

    // The enchanted land dies; the Aura goes with it.
    kill(&mut e, land);
    assert!(
        on_battlefield(&e, P0, goblin_shrine()).is_none(),
        "the Aura left"
    );
    assert!(in_graveyard(&e, P0, goblin_shrine()).is_some());

    assert_eq!(
        e.state().object(chirurgeon).unwrap().damage,
        1,
        "one damage to P0's Goblin (0/2 now that the bonus is gone)"
    );
    assert!(
        in_graveyard(&e, P1, goblin_raiders()).is_some(),
        "one damage kills P1's 1/1 Goblin"
    );
    assert_eq!(
        e.state().object(bears).unwrap().damage,
        0,
        "the Bear is no Goblin"
    );
    assert_eq!(pt(&e, chirurgeon), (0, 2), "the bonus ended with the Aura");
}
