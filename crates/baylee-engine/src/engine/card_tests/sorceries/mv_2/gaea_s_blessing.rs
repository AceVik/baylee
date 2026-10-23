//! `cards/sorceries/mv_2/gaea_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gaea_s_blessing() -> CardIndex {
    card_index("bd5478df-3854-404b-8634-9debcef69b99")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn library_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Library(seat)).len()
}

fn hand_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// How many copies of `card` sit in `seat`'s library or hand: after a
/// shuffle and a draw, a card put into the library may be in either.
fn in_library_or_hand(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    [ZoneLocation::Library(seat), ZoneLocation::Hand(seat)]
        .into_iter()
        .flat_map(|zone| e.state().zones.list(zone).iter().copied())
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// Answers the order question the controller gets on resolution for two or
/// more cards (they are shuffled right after, so any order will do).
#[track_caller]
fn order_as_offered(e: &mut Engine<RegistryLookup>) {
    let Pending::Arrange { player, cards, .. } = e.pending().clone() else {
        panic!("the cards go to the top in an order, got {:?}", e.pending());
    };
    e.apply(player, PlayerAction::Arrange { piles: vec![cards] })
        .unwrap();
}

/// Gaea's Blessing (bd5478df-3854-404b-8634-9debcef69b99) — "Target player
/// shuffles up to three target cards from their graveyard into their library.
/// Draw a card." Four cards lie in the graveyard: the menu offers all four,
/// a fourth target is refused, three go into the library (not exile), the
/// fourth stays, and the caster draws exactly one.
#[test]
fn gaea_s_blessing_shuffles_up_to_three_cards_into_the_library_and_draws() {
    let mut e = lk::game(
        SEED,
        &[forest(), forest()],
        &[],
        &[
            gaea_s_blessing(),
            grizzly_bears(),
            hill_giant(),
            savannah_lions(),
            wall_of_ice(),
        ],
        &[],
    );
    let bears = hand_to_graveyard(&mut e, P0, grizzly_bears());
    let giant = hand_to_graveyard(&mut e, P0, hill_giant());
    let lions = hand_to_graveyard(&mut e, P0, savannah_lions());
    let wall = hand_to_graveyard(&mut e, P0, wall_of_ice());
    let library_before = library_size(&e, P0);
    assert_eq!(hand_size(&e, P0), 1, "only the Blessing is left in hand");

    cast_from_hand(&mut e, P0, gaea_s_blessing());
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("the cards are named next, got {:?}", e.pending());
    };
    assert_eq!((min, max), (0, 3), "\"up to three\"");
    for card in [bears, giant, lions, wall] {
        assert!(options.contains(&card), "{card:?} is in the graveyard");
    }
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant, lions, wall],
                players: vec![],
            },
        )
        .is_err(),
        "a fourth card is one too many"
    );
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bears, giant, lions],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    order_as_offered(&mut e);
    pass_until(&mut e, stack_is_empty);

    for card in [grizzly_bears(), hill_giant(), savannah_lions()] {
        assert!(
            in_graveyard(&e, P0, card).is_none(),
            "{card:?} left the graveyard"
        );
        assert_eq!(
            in_library_or_hand(&e, P0, card),
            1,
            "{card:?} is in the library (or was drawn from it), not exiled"
        );
    }
    assert!(
        in_graveyard(&e, P0, wall_of_ice()).is_some(),
        "the one card not named stays"
    );
    assert!(in_graveyard(&e, P0, gaea_s_blessing()).is_some());
    assert_eq!(
        hand_size(&e, P0),
        1,
        "the Blessing left, one card was drawn"
    );
    assert_eq!(
        library_size(&e, P0),
        library_before + 3 - 1,
        "three went in, one came out"
    );
}

/// Gaea's Blessing — "Target player … their graveyard … their library": aimed
/// at the opponent, the menu lists the opponent's graveyard and not the
/// caster's; their card goes into their library; they draw nothing, the
/// caster draws.
#[test]
fn gaea_s_blessing_aimed_at_the_opponent_uses_their_graveyard_and_library() {
    let mut e = lk::game(
        SEED,
        &[forest(), forest()],
        &[],
        &[gaea_s_blessing(), hill_giant()],
        &[grizzly_bears()],
    );
    let mine = hand_to_graveyard(&mut e, P0, hill_giant());
    let theirs = hand_to_graveyard(&mut e, P1, grizzly_bears());
    let mine_before = library_size(&e, P0);
    let theirs_before = library_size(&e, P1);
    let their_hand = hand_size(&e, P1);

    cast_from_hand(&mut e, P0, gaea_s_blessing());
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the cards are named next, got {:?}", e.pending());
    };
    assert!(options.contains(&theirs), "their graveyard: {options:?}");
    assert!(!options.contains(&mine), "not the caster's: {options:?}");
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![theirs],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P1, grizzly_bears()).is_none());
    assert_eq!(in_library_or_hand(&e, P1, grizzly_bears()), 1);
    assert_eq!(
        library_size(&e, P1),
        theirs_before + 1,
        "into their library"
    );
    assert_eq!(hand_size(&e, P1), their_hand, "and they drew nothing");
    assert!(
        in_graveyard(&e, P0, hill_giant()).is_some(),
        "the caster's graveyard is untouched"
    );
    assert_eq!(library_size(&e, P0), mine_before - 1, "the caster drew");
    assert_eq!(hand_size(&e, P0), 1);
}

/// Gaea's Blessing — "up to three": naming none is legal, nothing moves and
/// the draw still happens.
#[test]
fn gaea_s_blessing_may_name_no_cards_and_still_draws() {
    let mut e = lk::game(
        SEED,
        &[forest(), forest()],
        &[],
        &[gaea_s_blessing(), grizzly_bears()],
        &[],
    );
    hand_to_graveyard(&mut e, P0, grizzly_bears());
    let library_before = library_size(&e, P0);

    cast_from_hand(&mut e, P0, gaea_s_blessing());
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "nothing was named"
    );
    assert_eq!(
        library_size(&e, P0),
        library_before - 1,
        "the draw happened"
    );
}
