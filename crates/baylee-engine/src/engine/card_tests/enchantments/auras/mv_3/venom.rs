//! `cards/enchantments/auras/mv_3/venom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

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

fn venom() -> CardIndex {
    card_index("ee55ca31-73f3-4e9d-8373-0a44009a25bd")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

/// Venom on P0's Hill Giant, which then attacks P1 and is blocked by
/// `blocker`. Stops once the blockers are declared, before damage.
fn venomed_giant_attacks_into(blocker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(&[venom()], &[hill_giant()], &[blocker]);
    let giant = on_battlefield(&e, P0, hill_giant()).expect("Hill Giant is out");
    let wall = on_battlefield(&e, P1, blocker).expect("the blocker is out");

    cast(&mut e, P0, venom());
    targets(&mut e, vec![giant]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, venom()).is_some(), "the Aura is out");

    let blocks = attack_and_collect_blocks(&mut e, giant, P1);
    assert!(
        blocks.iter().any(|b| b.blocker == wall),
        "the blocker may block: {blocks:?}"
    );
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, giant)],
        },
    )
    .expect("the block is legal");
    (e, wall)
}

/// Venom (ee55ca31-73f3-4e9d-8373-0a44009a25bd) — "Enchant creature. Whenever
/// enchanted creature blocks or becomes blocked by a non-Wall creature, destroy
/// the other creature at end of combat." (becomes blocked)
#[test]
fn venom_destroys_a_non_wall_that_blocks_the_enchanted_creature_at_end_of_combat() {
    // Serra Angel (4/4) survives the Giant's three damage, so only Venom can
    // be what destroys it.
    let (mut e, _angel) = venomed_giant_attacks_into(serra_angel());
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        in_graveyard(&e, P1, serra_angel()).is_some(),
        "destroyed at end of combat"
    );
}

/// Venom — (a Wall is excepted)
#[test]
fn venom_leaves_a_wall_that_blocks_alone() {
    let (mut e, wall) = venomed_giant_attacks_into(wall_of_stone());
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().object(wall).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Wall of Stone is a Wall: Venom does not touch it"
    );
}

/// Venom — (the enchanted creature blocks)
#[test]
fn venom_destroys_a_non_wall_that_the_enchanted_creature_blocks() {
    let mut e = prepared(&[venom()], &[grizzly_bears()], &[hill_giant()]);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");
    cast(&mut e, P0, venom());
    targets(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    // P1's turn: the Giant (3/3) attacks and the enchanted Bears block it, so
    // the Giant takes only two damage and survives; only Venom can kill it.
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    let blocks = attack_and_collect_blocks(&mut e, giant, P0);
    assert!(
        blocks.iter().any(|b| b.blocker == bears),
        "the Bears may block: {blocks:?}"
    );
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bears, giant)],
        },
    )
    .expect("the Bears block");
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        in_graveyard(&e, P1, hill_giant()).is_some(),
        "destroyed at end of combat"
    );
}
