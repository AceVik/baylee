//! `cards/creatures/mv_2/sylvan_hierophant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sylvan_hierophant() -> CardIndex {
    card_index("e98361d9-1e3a-464e-bac6-75314fc49f32")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn gray_ogre() -> CardIndex {
    card_index("83c8a3a6-2e1a-4e26-8847-6d066f42d906")
}

/// Player 0's Hierophant and a Bolt in hand; `p0_dead` are creature cards
/// already in player 0's graveyard, `p1_dead` in player 1's.
fn table(p0_dead: &[CardIndex], p1_dead: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = vec![lightning_bolt()];
    hand.extend_from_slice(p0_dead);
    let mut e = lk::game(SEED, &[sylvan_hierophant()], &[], &hand, p1_dead);
    for card in p0_dead {
        hand_to_graveyard(&mut e, P0, *card);
    }
    for card in p1_dead {
        hand_to_graveyard(&mut e, P1, *card);
    }
    e
}

/// Bolts the Hierophant (3 damage to a 1/2) and returns the target menu its
/// death trigger asks for.
fn bolt_the_hierophant(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let hierophant = lk::obj(e, P0, sylvan_hierophant());
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, lightning_bolt());
    aim_at(e, P0, hierophant);
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    options
}

/// Sylvan Hierophant (e98361d9-1e3a-464e-bac6-75314fc49f32) — "When this
/// creature dies, exile it, then return another target creature card from your
/// graveyard to your hand." The menu is **your** graveyard's **other creature**
/// cards: the Bears and the Giant, not the Hierophant itself, not the Bolt that
/// killed it, not the opponent's Ogre. The chosen Bears returns, the Giant stays,
/// the Hierophant is exiled.
#[test]
fn sylvan_hierophant_exiles_itself_and_returns_one_other_creature_card_of_yours() {
    let mut e = table(&[grizzly_bears(), hill_giant()], &[gray_ogre()]);
    let bears = in_graveyard(&e, P0, grizzly_bears()).expect("Bears in the graveyard");
    let giant = in_graveyard(&e, P0, hill_giant()).expect("Giant in the graveyard");
    let ogre = in_graveyard(&e, P1, gray_ogre()).expect("Ogre in their graveyard");
    let hierophant = lk::obj(&e, P0, sylvan_hierophant());

    let menu = bolt_the_hierophant(&mut e);
    let bolt = in_graveyard(&e, P0, lightning_bolt()).expect("the Bolt resolved");
    assert!(
        menu.contains(&bears) && menu.contains(&giant),
        "your creature cards: {menu:?}"
    );
    assert!(
        !menu.contains(&hierophant),
        "'another': not itself: {menu:?}"
    );
    assert!(
        !menu.contains(&bolt),
        "a Bolt is no creature card: {menu:?}"
    );
    assert!(
        !menu.contains(&ogre),
        "'your graveyard': not theirs: {menu:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        lk::zone_of(&e, hierophant),
        crate::zone::Zone::Exile,
        "it exiles itself"
    );
    assert!(
        in_hand(&e, P0, grizzly_bears()).is_some(),
        "the target returns to hand"
    );
    assert!(
        in_graveyard(&e, P0, hill_giant()).is_some(),
        "the other creature card stays"
    );
    assert!(
        in_graveyard(&e, P1, gray_ogre()).is_some(),
        "theirs is untouched"
    );
}

/// Sylvan Hierophant — with no other creature card of yours in the graveyard
/// the trigger has no legal target and is removed from the stack (CR 608.2b):
/// nothing is asked and the Hierophant is not exiled, it stays in the
/// graveyard.
#[test]
fn sylvan_hierophant_with_no_other_creature_card_stays_in_the_graveyard() {
    let mut e = table(&[], &[gray_ogre()]);
    let hierophant = lk::obj(&e, P0, sylvan_hierophant());

    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, hierophant);
    pass_until(&mut e, stack_is_empty);

    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "no target was asked for: {:?}",
        e.pending()
    );
    assert!(
        in_graveyard(&e, P0, sylvan_hierophant()).is_some(),
        "no target, so the trigger does nothing: it was not exiled"
    );
}
