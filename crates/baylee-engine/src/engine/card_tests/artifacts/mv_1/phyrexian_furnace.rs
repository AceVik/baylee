//! `cards/artifacts/mv_1/phyrexian_furnace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::zone::Zone;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phyrexian_furnace() -> CardIndex {
    card_index("4aafb7e7-b9fc-44ae-9c54-da11fd9dcc17")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// The Furnace and a Plains on player 0's side, one creature card in each
/// graveyard (Bears in player 0's, the Giant in player 1's).
fn table() -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[phyrexian_furnace(), plains()],
        &[],
        &[grizzly_bears()],
        &[hill_giant()],
    );
    hand_to_graveyard(&mut e, P0, grizzly_bears());
    hand_to_graveyard(&mut e, P1, hill_giant());
    e
}

/// Phyrexian Furnace (4aafb7e7-b9fc-44ae-9c54-da11fd9dcc17) — "{1}, Sacrifice
/// this artifact: Exile target card from a graveyard. Draw a card." The
/// target may be in either graveyard: the opponent's Giant is exiled, the
/// caster's own Bears stays, the Furnace is sacrificed and one card is drawn.
#[test]
fn phyrexian_furnace_sacrifices_to_exile_a_card_from_a_graveyard_and_draw() {
    let mut e = table();
    let bears = in_graveyard(&e, P0, grizzly_bears()).expect("Bears in the graveyard");
    let giant = in_graveyard(&e, P1, hill_giant()).expect("Giant in the graveyard");
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, phyrexian_furnace(), 0);
    let menu = aim_at(&mut e, P0, giant);
    assert!(
        menu.contains(&bears) && menu.contains(&giant),
        "a card in either graveyard is a legal target: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::zone_of(&e, giant), Zone::Exile, "the target is exiled");
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "only the target goes"
    );
    assert!(
        in_graveyard(&e, P0, phyrexian_furnace()).is_some(),
        "the Furnace was sacrificed as the cost"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        1,
        "and a card is drawn"
    );
}

/// Phyrexian Furnace — the same ability aimed at the caster's own graveyard
/// ("a graveyard", not "target opponent's"): the Bears is exiled and the
/// opponent's Giant stays.
#[test]
fn phyrexian_furnace_may_exile_a_card_from_its_controllers_graveyard() {
    let mut e = table();
    let bears = in_graveyard(&e, P0, grizzly_bears()).expect("Bears in the graveyard");

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, phyrexian_furnace(), 0);
    aim_at(&mut e, P0, bears);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::zone_of(&e, bears), Zone::Exile);
    assert!(
        in_graveyard(&e, P1, hill_giant()).is_some(),
        "the other graveyard is untouched"
    );
}

/// Phyrexian Furnace — "{1}" is a cost: with mana in the pool the ability is
/// offered, with no mana it is not; and the printed "{T}: Exile the bottom
/// card of target player's graveyard" is not on the card (nothing at ability
/// index 1, with or without mana).
#[test]
fn phyrexian_furnace_needs_its_mana_and_has_no_tap_ability() {
    let mut e = table();
    tap_all_mana(&mut e, P0);
    assert!(
        lk::offered(&e, phyrexian_furnace(), P0, 0),
        "{{1}} is in the pool"
    );
    assert!(
        !lk::offered(&e, phyrexian_furnace(), P0, 1),
        "no second ability: the tap ability is not supported"
    );

    let mut poor = lk::game(SEED, &[phyrexian_furnace()], &[], &[grizzly_bears()], &[]);
    hand_to_graveyard(&mut poor, P0, grizzly_bears());
    assert!(
        !lk::offered(&poor, phyrexian_furnace(), P0, 0),
        "without {{1}} the sacrifice ability cannot be activated"
    );
    assert!(
        !lk::offered(&poor, phyrexian_furnace(), P0, 1),
        "tapping alone does nothing"
    );
}
