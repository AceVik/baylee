//! `cards/sorceries/mv_1/mind_bomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_bomb() -> CardIndex {
    card_index("fad1562d-0ab9-4a3f-b80a-476d8132f66d")
}

/// Mind Bomb is cast with seat 0's hand and seat 1's hand as given (seat 0's
/// has the spell besides), and the engine stops at the first discard choice.
/// Returns the engine and the two hands as they stand with the spell on the
/// stack.
fn bombed(
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, Vec<ObjectId>, Vec<ObjectId>) {
    let mut hand = vec![mind_bomb()];
    hand.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &hand)
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, mind_bomb());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).clone();
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).clone();
    (engine, my_hand, their_hand)
}

/// Answers the discard question: it must be `seat`'s, offering exactly
/// `hand`, and it takes at most three; `count` of them are discarded.
#[track_caller]
fn discard(engine: &mut Engine<RegistryLookup>, seat: PlayerId, hand: &[ObjectId], count: usize) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("a discard choice was expected, got {:?}", engine.pending());
    };
    assert_eq!(player, seat, "this seat is asked");
    assert_eq!(options.len(), hand.len(), "every card in hand is offered");
    assert_eq!(min, 0, "up to: none is allowed");
    assert_eq!(usize::from(max), hand.len().min(3), "up to three");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: hand[..count].to_vec(),
            },
        )
        .expect("discarding up to three is legal");
}

/// Mind Bomb (fad1562d-0ab9-4a3f-b80a-476d8132f66d) — "Each player may
/// discard up to three cards. Mind Bomb deals damage to each player equal to
/// 3 minus the number of cards they discarded this way." Seat 0 discards two
/// and takes 1; seat 1 holds one card, discards it and takes 2.
#[test]
fn mind_bomb_damage_is_three_minus_the_cards_each_player_discarded() {
    let (mut engine, mine, theirs) = bombed(&[forest(), plains(), swamp()], &[mountain()]);
    discard(&mut engine, P0, &mine, 2);
    discard(&mut engine, P1, &theirs, 1);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 19, "3 - 2");
    assert_eq!(lk::life(&engine, P1), 18, "3 - 1");
    for card in &mine[..2] {
        assert_eq!(lk::zone_of(&engine, *card), Zone::Graveyard, "discarded");
    }
    assert_eq!(lk::zone_of(&engine, theirs[0]), Zone::Graveyard);
    for card in &mine[2..] {
        assert_eq!(lk::zone_of(&engine, *card), Zone::Hand, "kept");
    }
}

/// Mind Bomb — discarding three each (three is the most) takes the damage
/// down to nothing, and a fourth card in hand stays there.
#[test]
fn mind_bomb_discarding_three_each_prevents_all_the_damage() {
    let (mut engine, mine, theirs) = bombed(
        &[forest(), plains(), swamp(), island()],
        &[mountain(), mountain(), mountain(), forest()],
    );
    discard(&mut engine, P0, &mine, 3);
    discard(&mut engine, P1, &theirs, 3);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20, "3 - 3");
    assert_eq!(lk::life(&engine, P1), 20, "3 - 3");
    assert_eq!(
        lk::zone_of(&engine, theirs[3]),
        Zone::Hand,
        "the fourth is kept"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        3
    );
}

/// Mind Bomb — "may": discarding nothing is allowed, and costs 3 life each.
#[test]
fn mind_bomb_discarding_nothing_deals_three_to_each_player() {
    let (mut engine, mine, theirs) = bombed(&[forest(), plains()], &[mountain()]);
    discard(&mut engine, P0, &mine, 0);
    discard(&mut engine, P1, &theirs, 0);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 17);
    assert_eq!(lk::life(&engine, P1), 17);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        theirs.len(),
        "nothing left the hand"
    );
}

/// Mind Bomb — "up to three": a fourth card is refused.
#[test]
fn mind_bomb_refuses_a_fourth_discard() {
    let (mut engine, mine, _) = bombed(&[forest(), plains(), swamp(), island()], &[]);
    assert!(mine.len() >= 4);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: mine[..4].to_vec()
                }
            )
            .is_err(),
        "four is more than up to three"
    );
}
