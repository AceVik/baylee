//! `cards/instants/mv_1/firestorm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn firestorm() -> CardIndex {
    card_index("157813c8-af83-41b5-9ffb-4abd167c1bfc")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Whether the Firestorm may be cast now, mana already made or still to be made.
fn may_cast(e: &Engine<RegistryLookup>) -> bool {
    let id = in_hand(e, P0, firestorm()).expect("the Firestorm is in hand");
    let Pending::Priority { legal, .. } = e.pending() else {
        return false;
    };
    legal.castable.contains(&id) || legal.payable.contains(&id)
}

/// Firestorm — "As an additional cost to cast this spell, discard X cards."
/// The card is the X = 1 case: it cannot be cast with no other card in hand,
/// and with one it can.
#[test]
fn firestorm_cannot_be_cast_without_a_card_to_discard() {
    let mut alone = lk::game(SEED, &[mountain()], &[], &[firestorm()], &[]);
    tap_all_mana(&mut alone, P0);
    assert!(
        !may_cast(&alone),
        "nothing to discard: the cost cannot be paid"
    );

    let mut with_card = lk::game(SEED, &[mountain()], &[], &[firestorm(), forest()], &[]);
    tap_all_mana(&mut with_card, P0);
    assert!(may_cast(&with_card), "one card beside it pays the cost");
}

/// Firestorm — "discard X cards" and "X damage to each of X targets", X = 1:
/// the caster picks which card goes, the Firestorm itself is not on the menu,
/// the other card stays, and the one player target takes 1 (not 2, and the
/// caster takes none).
#[test]
fn firestorm_discards_the_chosen_card_and_deals_one_damage_to_a_player() {
    let mut e = lk::game(
        SEED,
        &[mountain()],
        &[],
        &[firestorm(), grizzly_bears(), forest()],
        &[],
    );
    let bears = in_hand(&e, P0, grizzly_bears()).expect("Bears in hand");
    let spell = in_hand(&e, P0, firestorm()).expect("Firestorm in hand");
    let land = in_hand(&e, P0, forest()).expect("Forest in hand");

    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, firestorm());
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
        panic!("the cost asks which card to discard, got {:?}", e.pending())
    };
    assert_eq!(player, P0, "the caster picks the discard");
    assert!(
        options.contains(&bears) && options.contains(&land),
        "both other cards: {options:?}"
    );
    assert!(
        !options.contains(&spell),
        "the spell itself is on the stack, not in hand: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, grizzly_bears()).is_some(), "discarded");
    assert!(in_hand(&e, P0, forest()).is_some(), "exactly one card goes");
    assert_eq!(lk::life(&e, P1), 19, "one damage to the target player");
    assert_eq!(lk::life(&e, P0), 20, "the caster is untouched");
}

/// Firestorm — "any target": a creature takes the 1 damage too, and the
/// player at 20 beside it is not hit.
#[test]
fn firestorm_deals_one_damage_to_a_creature_and_not_to_the_player() {
    let mut e = lk::game(
        SEED,
        &[mountain()],
        &[grizzly_bears()],
        &[firestorm(), forest()],
        &[],
    );
    let theirs = lk::obj(&e, P1, grizzly_bears());
    let land = in_hand(&e, P0, forest()).expect("Forest in hand");

    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, firestorm());
    let menu = aim_at(&mut e, P0, theirs);
    assert!(
        menu.contains(&theirs),
        "a creature is a legal target: {menu:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::damage_on(&e, theirs), 1, "one damage, not two");
    assert!(
        on_battlefield(&e, P1, grizzly_bears()).is_some(),
        "a 2/2 survives it"
    );
    assert_eq!(lk::life(&e, P1), 20, "the player was not the target");
    assert!(
        in_graveyard(&e, P0, forest()).is_some(),
        "the discard was paid"
    );
}
