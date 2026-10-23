//! `cards/sorceries/mv_3/haunting_misery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn haunting_misery() -> CardIndex {
    card_index("8db6651b-c368-46fd-b9c2-69a4d6562246")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Whether the spell may be cast now, mana already made or still to be made.
fn may_cast(e: &Engine<RegistryLookup>) -> bool {
    let id = in_hand(e, P0, haunting_misery()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = e.pending() else {
        return false;
    };
    legal.castable.contains(&id) || legal.payable.contains(&id)
}

/// A duel in player 0's first main phase with three Swamps, the spell in hand,
/// and the given cards already in player 0's graveyard.
fn table(graveyard: &[CardIndex], opp_board: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = vec![haunting_misery()];
    hand.extend_from_slice(graveyard);
    let mut e = lk::game(SEED, &[swamp(), swamp(), swamp()], opp_board, &hand, &[]);
    for card in graveyard {
        hand_to_graveyard(&mut e, P0, *card);
    }
    e
}

/// Haunting Misery (8db6651b-c368-46fd-b9c2-69a4d6562246) — "As an additional
/// cost to cast this spell, exile X creature cards from your graveyard": the
/// X = 1 case needs a creature card there. An empty graveyard and a graveyard
/// with only a land both leave it uncastable; a Bears beside the land
/// makes it castable.
#[test]
fn haunting_misery_needs_a_creature_card_in_the_graveyard_to_exile() {
    let empty = table(&[], &[]);
    assert!(!may_cast(&empty), "nothing in the graveyard to exile");

    let only_a_land = table(&[forest()], &[]);
    assert!(
        !may_cast(&only_a_land),
        "a land card is not a creature card"
    );

    let with_a_creature = table(&[forest(), grizzly_bears()], &[]);
    assert!(
        may_cast(&with_a_creature),
        "one creature card pays the cost"
    );
}

/// Haunting Misery — "deals X damage to target player or planeswalker" and
/// the exile cost, X = 1: the player target is named first, the menu of the
/// cost holds the creature cards only (not the Forest, not the other
/// seat's), the chosen card is exiled and the rest stays, the target takes 1
/// and the caster none.
#[test]
fn haunting_misery_exiles_the_chosen_creature_card_and_deals_one_damage() {
    let mut e = table(&[forest(), grizzly_bears(), hill_giant()], &[hill_giant()]);
    let bears = in_graveyard(&e, P0, grizzly_bears()).expect("Bears in the graveyard");
    let giant = in_graveyard(&e, P0, hill_giant()).expect("Giant in the graveyard");
    let land = in_graveyard(&e, P0, forest()).expect("Forest in the graveyard");
    let their_giant = lk::obj(&e, P1, hill_giant());

    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, haunting_misery());
    let Pending::ChooseTargets {
        player_options,
        options,
        ..
    } = e.pending().clone()
    else {
        panic!("the target comes first, got {:?}", e.pending())
    };
    assert!(player_options.contains(&P1), "a player is a legal target");
    assert!(
        !options.contains(&their_giant),
        "a creature is not a player or planeswalker: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("the cost asks which card to exile, got {:?}", e.pending())
    };
    assert_eq!(player, P0, "the caster picks the card");
    assert!(
        options.contains(&bears) && options.contains(&giant),
        "both creature cards: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "the Forest is no creature card: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        lk::zone_of(&e, bears),
        crate::zone::Zone::Exile,
        "the chosen card is exiled, not left in the graveyard"
    );
    assert!(
        in_graveyard(&e, P0, hill_giant()).is_some(),
        "the other creature card stays"
    );
    assert!(in_graveyard(&e, P0, forest()).is_some(), "the land stays");
    assert_eq!(lk::life(&e, P1), 19, "X = 1 damage to the target player");
    assert_eq!(lk::life(&e, P0), 20, "the caster is untouched");
    assert!(
        on_battlefield(&e, P1, hill_giant()).is_some(),
        "no creature is hurt"
    );
}
