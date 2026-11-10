//! `cards/sorceries/mv_1/recall.rs`, played.
//!
//! "Discard X cards, then return a card from your graveyard to your hand for
//! each card discarded this way. Exile Recall." Three things have to be read
//! right: the discard is the player's own choice and takes what the hand has
//! when X is more than that; the return is counted off what was *actually*
//! discarded, not off X; and the graveyard it chooses from already holds the
//! discards, so a discarded card may come straight back (rulings,
//! 2009-10-01).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn recall() -> CardIndex {
    card_index("4158f397-4ee4-4f53-90f2-3b64bfe4f9b8")
}

/// One `ChooseCards` question Recall asked, as the player saw it.
struct Asked {
    /// Whether the menu was the hand (a discard) rather than the graveyard.
    from_hand: bool,
    min: u8,
    max: u8,
    options: Vec<ObjectId>,
}

/// Lets Recall resolve and answers each card question as a player would:
/// `discards` are the cards given up from the hand, `returns` the cards
/// taken back from the graveyard, each filtered down to what the menu
/// actually offers. Returns every question in the order it was asked.
#[track_caller]
fn play_out(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    discards: &[ObjectId],
    returns: &[ObjectId],
) -> Vec<Asked> {
    let mut asked = Vec::new();
    for _ in 0..20 {
        pass_until(engine, |e| {
            stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
        });
        if stack_is_empty(engine) {
            return asked;
        }
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            unreachable!("pass_until stopped on a card choice")
        };
        assert_eq!(player, p0, "the caster makes both choices");
        let from_hand = options.iter().all(|o| {
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(o)
        });
        let wanted = if from_hand { discards } else { returns };
        let picks: Vec<ObjectId> = wanted
            .iter()
            .copied()
            .filter(|o| options.contains(o))
            .take(usize::from(max))
            .collect();
        assert!(
            picks.len() >= usize::from(min),
            "the answer this test has ({picks:?}) is short of what the menu \
             requires: min {min}, options {options:?}"
        );
        asked.push(Asked {
            from_hand,
            min,
            max,
            options,
        });
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: picks })
            .unwrap();
    }
    panic!("Recall never finished asking");
}

/// Whether Recall is in `seat`'s exile zone, and nowhere else it should not be.
#[track_caller]
fn assert_recall_exiled(engine: &Engine<RegistryLookup>, seat: PlayerId) {
    assert!(
        in_graveyard(engine, seat, recall()).is_none(),
        "\"Exile Recall\": it is not in the graveyard"
    );
    assert!(
        in_hand(engine, seat, recall()).is_none(),
        "and not back in hand"
    );
    let exiled = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == recall()))
        });
    assert!(exiled, "it is in exile");
}

/// "Discard X cards, then return a card from your graveyard to your hand for
/// each card discarded this way. Exile Recall." X = 2: the player chooses two
/// cards to discard, then two cards to return -- from a graveyard that holds
/// the two it already had *and* the two just discarded, so one of the
/// discarded cards may be taken straight back. Recall itself is exiled, not
/// put into the graveyard to be fetched later.
#[test]
fn recall_discards_x_then_returns_that_many_cards_including_discarded_ones() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(
            0,
            &[
                recall(),
                mountain(),
                swamp(),
                forest(),
                plains(),
                lightning_bolt(),
                giant_growth(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bolt = hand_to_graveyard(&mut engine, p0, lightning_bolt());
    hand_to_graveyard(&mut engine, p0, giant_growth());
    let mountain_card = in_hand(&engine, p0, mountain()).expect("a Mountain in hand");
    let swamp_card = in_hand(&engine, p0, swamp()).expect("a Swamp in hand");
    let forest_card = in_hand(&engine, p0, forest()).expect("a Forest in hand");
    let plains_card = in_hand(&engine, p0, plains()).expect("a Plains in hand");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, recall());
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("{{X}}{{X}}{{U}} asks for X, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "five mana pays {{X}}{{X}}{{U}} up to X = 2"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let asked = play_out(
        &mut engine,
        p0,
        &[mountain_card, swamp_card],
        &[bolt, mountain_card],
    );

    let discard = asked
        .iter()
        .find(|q| q.from_hand)
        .expect("the discard was asked");
    assert_eq!(
        (discard.min, discard.max),
        (2, 2),
        "exactly X cards are discarded"
    );
    let mut menu = discard.options.clone();
    menu.sort();
    let mut hand_cards = vec![mountain_card, swamp_card, forest_card, plains_card];
    hand_cards.sort();
    assert_eq!(
        menu, hand_cards,
        "the whole hand is the menu: Recall is on the stack and the graveyard is not discarded from"
    );
    assert!(
        asked.iter().any(|q| !q.from_hand),
        "and cards were chosen back from the graveyard"
    );
    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_some(),
        "the Bolt that was already in the graveyard came back"
    );
    assert!(
        in_hand(&engine, p0, mountain()).is_some(),
        "so did the Mountain just discarded: \"you may choose to return some \
         or all of the cards you just discarded\""
    );
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "the other discarded card stayed there"
    );
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "and so did the card that was not chosen"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some() && in_hand(&engine, p0, plains()).is_some(),
        "the cards that were not discarded are still in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        4,
        "Forest, Plains, Bolt and Mountain: two discarded, two returned"
    );
    assert_recall_exiled(&engine, p0);
}

/// "You don't discard cards until Recall resolves. If you don't have X cards
/// in your hand at that time, you discard all the cards in your hand." And the
/// return is "for each card discarded this way": a one-card hand under X = 2
/// discards one card and returns one, not two.
#[test]
fn recall_with_a_short_hand_discards_what_it_has_and_returns_only_that_many() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[recall(), mountain(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bolt = hand_to_graveyard(&mut engine, p0, lightning_bolt());
    let mountain_card = in_hand(&engine, p0, mountain()).expect("a Mountain in hand");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, recall());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    // The player would happily take both cards back; only one was discarded.
    play_out(&mut engine, p0, &[mountain_card], &[bolt, mountain_card]);

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(
        hand.len(),
        1,
        "one card was discarded, so one card is returned: {hand:?}"
    );
    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_some(),
        "the one the player chose, the Bolt, is the card in hand"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "and the discarded Mountain is in the graveyard"
    );
    assert_recall_exiled(&engine, p0);
}

/// X = 0: nothing is discarded, so nothing is returned, and Recall is still
/// exiled -- "Exile Recall" does not depend on the discard.
#[test]
fn recall_for_zero_does_nothing_but_exile_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[recall(), mountain(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, lightning_bolt());

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, recall());
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    // `pass_until` refuses a card question: X = 0 asks none.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, mountain()).is_some(),
        "no card was discarded"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "and none was returned"
    );
    assert_recall_exiled(&engine, p0);
}
