//! `cards/sorceries/mv_2/shattered_crypt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shattered_crypt() -> CardIndex {
    card_index("77bef290-7229-420a-a5c6-8c1b6fea63df")
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

/// Player 0 has the Crypt in hand and creature cards (Bears, Giant) and a
/// Forest in their graveyard; player 1 has an Ogre card in theirs. Six Black
/// mana float, so the price paid is read off what is left.
fn table() -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[shattered_crypt(), grizzly_bears(), hill_giant(), forest()],
        &[gray_ogre()],
    );
    for card in [grizzly_bears(), hill_giant(), forest()] {
        hand_to_graveyard(&mut e, P0, card);
    }
    hand_to_graveyard(&mut e, P1, gray_ogre());
    lk::float(&mut e, P0, &[(ManaColor::Black, 6)]);
    e
}

/// Shattered Crypt (77bef290-7229-420a-a5c6-8c1b6fea63df) — "Return X target
/// creature cards from your graveyard to your hand. You lose X life." X = 2
/// costs {2}{B}{B} (four of the six), offers the Bears and the Giant only (not
/// the Forest, not the opponent's Ogre), returns both and costs 2 life.
#[test]
fn shattered_crypt_returns_x_creature_cards_of_yours_and_loses_x_life() {
    let mut e = table();
    let bears = in_graveyard(&e, P0, grizzly_bears()).expect("Bears in the graveyard");
    let giant = in_graveyard(&e, P0, hill_giant()).expect("Giant in the graveyard");
    let land = in_graveyard(&e, P0, forest()).expect("Forest in the graveyard");
    let ogre = in_graveyard(&e, P1, gray_ogre()).expect("Ogre in their graveyard");

    cast_with_floating(&mut e, P0, shattered_crypt());
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets {
        min, max, options, ..
    } = e.pending().clone()
    else {
        panic!("X targets are asked for, got {:?}", e.pending())
    };
    assert_eq!((min, max), (2, 2), "exactly X targets");
    assert!(
        options.contains(&bears) && options.contains(&giant),
        "{options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land card is no creature card: {options:?}"
    );
    assert!(
        !options.contains(&ogre),
        "only your own graveyard: {options:?}"
    );
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bears, giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_hand(&e, P0, grizzly_bears()).is_some() && in_hand(&e, P0, hill_giant()).is_some());
    assert!(
        in_graveyard(&e, P0, forest()).is_some(),
        "the land card stays"
    );
    assert!(in_graveyard(&e, P1, gray_ogre()).is_some(), "theirs stays");
    assert_eq!(lk::life(&e, P0), 18, "X = 2 life");
    assert_eq!(lk::life(&e, P1), 20);
    assert_eq!(
        lk::pool_total(&e, P0),
        2,
        "{{X}}{{B}}{{B}} with X = 2 is four mana"
    );
}

/// Shattered Crypt — X = 1 returns exactly the one chosen card (the Giant), the
/// Bears stays in the graveyard, 1 life is lost, and three mana are spent.
#[test]
fn shattered_crypt_with_x_one_returns_one_card_and_loses_one_life() {
    let mut e = table();
    let giant = in_graveyard(&e, P0, hill_giant()).expect("Giant in the graveyard");

    cast_with_floating(&mut e, P0, shattered_crypt());
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_hand(&e, P0, hill_giant()).is_some());
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "the other creature card stays"
    );
    assert_eq!(lk::life(&e, P0), 19);
    assert_eq!(lk::pool_total(&e, P0), 3, "X = 1 is three mana");
}
