//! `cards/enchantments/mv_2/chains_of_mephistopheles.rs`, played.
//!
//! Chains of Mephistopheles, {1}{B} Enchantment: "If a player would draw a
//! card except the first one they draw in each of their draw steps, that
//! player discards a card instead. If the player discards a card this way,
//! they draw a card. If the player doesn't discard a card this way, they
//! mill a card."
//!
//! Written from the printed text, the card's rulings and the Comprehensive
//! Rules (121.2 and 121.6 a draw at a time and replaced draws, 614.5 one
//! opportunity per event, 616.1-616.2 the drawing player's order, 701.9
//! discard, 701.17 mill, 504.1 and 103.8a the draw step), not from the card.
//!
//! The cards that draw are real ones: Jayemdae Tome (one draw), Ancestral
//! Recall (three), Howling Mine (a second card in the draw step) and
//! Phyrexian Arena (a card in the upkeep). The library is Forests, so a
//! drawn card is told apart by its place in the library when the test began.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::draw_kit as kit;

/// Chains plus the board for a single draw.
fn chains_and_tome() -> Vec<CardIndex> {
    let mut board = kit::tome_board();
    board.push(kit::chains());
    board
}

/// "except the first one they draw in each of their draw steps": the draw
/// step's own card is drawn, and the second card the Howling Mine adds in the
/// same step is not -- for the opponent in their turn (one card in hand: the
/// discard is not a choice) and for the Chains' controller in theirs (three
/// cards in hand: the discard is asked, and the card the draw step drew is
/// among the options). Without the exemption the first card would have been
/// replaced too, with an empty hand a mill and no card at all.
#[test]
fn the_first_draw_of_a_draw_step_is_left_alone_and_the_next_one_is_replaced() {
    let mut engine = kit::duel(
        4501,
        &[kit::chains(), kit::mine()],
        &[swamp(), plains()],
        &[],
        &[],
    );
    let theirs = kit::top(&engine, kit::P1, 2);
    let mine = kit::top(&engine, kit::P0, 2);

    // Turn 2, the opponent's draw step: the turn's card, then the Mine's.
    assert!(
        !kit::play_until(&mut engine, kit::first_main_of(kit::P1, 2)),
        "one card in hand: the Chains' discard has no choice in it"
    );
    assert_eq!(
        kit::graveyard(&engine, kit::P1),
        vec![theirs[0]],
        "the first card, drawn untouched, is the one the second draw discarded"
    );
    assert_eq!(
        kit::hand(&engine, kit::P1),
        vec![theirs[1]],
        "and then the replaced draw was made"
    );

    // Turn 3, the controller's own draw step.
    assert!(
        kit::play_until(&mut engine, |e| e.state().turn.number > 3),
        "three cards in hand: the discard is the player's to choose"
    );
    assert_eq!(engine.state().turn.number, 3);
    assert!(kit::in_draw_step_of(&engine, kit::P0));
    assert_eq!(
        kit::hand(&engine, kit::P0).len(),
        3,
        "the draw step's first card was drawn without a question"
    );
    let asked = kit::question(&engine);
    assert_eq!(
        (asked.player, asked.min, asked.max, asked.prompt),
        (kit::P0, 1, 1, ChoicePrompt::Discard)
    );
    assert_eq!(
        kit::sorted(&asked.options),
        kit::sorted(&kit::hand(&engine, kit::P0)),
        "any card in the hand, the one just drawn included"
    );
    assert!(asked.options.contains(&mine[0]));
    let plains = in_hand(&engine, kit::P0, plains()).expect("the Plains is in hand");
    kit::reply(&mut engine, &[plains]);
    assert!(
        !kit::play_until(&mut engine, kit::first_main_of(kit::P0, 3)),
        "and that was the turn's only question"
    );
    assert_eq!(kit::graveyard(&engine, kit::P0), vec![plains]);
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        kit::sorted(&[
            in_hand(&engine, kit::P0, swamp()).expect("the Swamp stayed"),
            mine[0],
            mine[1],
        ]),
        "the Swamp, the card of the draw step and the card the discard drew"
    );
}

/// "discards a card instead. If the player discards a card this way, they
/// draw a card." In the main phase every card of Ancestral Recall's three is
/// a draw of its own (CR 121.2) and is replaced: the discard is asked first,
/// with the hand as it is (the card to be drawn is not in it yet), then the
/// draw is made. Three draws, three discards of the player's choice, three
/// cards drawn.
#[test]
fn a_draw_outside_the_draw_step_is_a_discard_of_your_choice_and_then_the_draw() {
    let mut engine = kit::duel(
        4502,
        &[kit::chains(), island()],
        &[kit::recall(), swamp(), plains()],
        &[],
        &[],
    );
    let [a, b, c]: [ObjectId; 3] = kit::top(&engine, kit::P0, 3).try_into().unwrap();
    let library_before = kit::library(&engine, kit::P0).len();
    kit::recall_at(&mut engine, kit::P0);

    let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
    let plains = in_hand(&engine, kit::P0, plains()).expect("the Plains");
    // What is thrown away each time, and what the hand is when it is asked.
    for (throw, hand) in [
        (swamp, vec![swamp, plains]),
        (plains, vec![plains, a]),
        (a, vec![a, b]),
    ] {
        let asked = kit::question(&engine);
        assert_eq!(
            (asked.player, asked.min, asked.max, asked.prompt),
            (kit::P0, 1, 1, ChoicePrompt::Discard)
        );
        assert_eq!(
            kit::sorted(&asked.options),
            kit::sorted(&hand),
            "asked before the card is drawn, over the hand as it stands"
        );
        kit::reply(&mut engine, &[throw]);
        kit::run_stack(&mut engine);
    }
    assert!(!kit::is_asking(&engine), "three cards, three questions");
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        kit::sorted(&[b, c]),
        "each discard was followed by the draw: two cards are left of five"
    );
    let grave = kit::graveyard(&engine, kit::P0);
    for card in [swamp, plains, a] {
        assert!(grave.contains(&card), "{card:?} was discarded");
    }
    assert_eq!(
        kit::library(&engine, kit::P0).len(),
        library_before - 3,
        "three cards left the library"
    );
}

/// "If the player doesn't discard a card this way, they mill a card" and the
/// card's ruling: with an empty hand they "put the top card of their library
/// into their graveyard. The player doesn't draw a card at all." Ancestral
/// Recall with nothing else in hand: three draws, three mills, and no card in
/// hand.
#[test]
fn with_an_empty_hand_each_replaced_draw_mills_a_card_and_draws_nothing() {
    let mut engine = kit::duel(4503, &[kit::chains(), island()], &[kit::recall()], &[], &[]);
    let library = kit::top(&engine, kit::P0, 60);
    kit::recall_at(&mut engine, kit::P0);
    assert!(
        !kit::is_asking(&engine),
        "an empty hand has nothing to choose"
    );
    assert!(kit::hand(&engine, kit::P0).is_empty(), "nothing was drawn");
    assert_eq!(kit::drawn_this_turn(&engine, kit::P0), 0);
    let grave = kit::graveyard(&engine, kit::P0);
    for milled in &library[..3] {
        assert!(grave.contains(milled), "the top three cards were milled");
    }
    assert_eq!(kit::library(&engine, kit::P0).len(), library.len() - 3);
    assert!(
        in_graveyard(&engine, kit::P0, kit::recall()).is_some(),
        "the spell itself is in the graveyard too"
    );
}

/// The same three draws with a single card in hand. The discard is the only
/// move there is, so nothing is asked, and the drawn card is the next draw's
/// discard: the Swamp, then the first card, then the second go, and the third
/// is kept. Beside the empty hand above it is the other half of "if the
/// player discards a card this way, they draw a card".
#[test]
fn with_one_card_in_hand_it_is_discarded_and_a_card_is_drawn_each_time() {
    let mut engine = kit::duel(
        4504,
        &[kit::chains(), island()],
        &[kit::recall(), swamp()],
        &[],
        &[],
    );
    let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
    let [a, b, c]: [ObjectId; 3] = kit::top(&engine, kit::P0, 3).try_into().unwrap();
    kit::recall_at(&mut engine, kit::P0);
    assert!(
        !kit::is_asking(&engine),
        "a hand of one has no choice in it"
    );
    assert_eq!(
        kit::hand(&engine, kit::P0),
        vec![c],
        "the last draw is kept"
    );
    let grave = kit::graveyard(&engine, kit::P0);
    for card in [swamp, a, b] {
        assert!(grave.contains(&card), "{card:?} was discarded in turn");
    }
}

/// "If a *player* would draw a card": the Chains' controller's opponent is
/// replaced too, and it is the opponent -- the player drawing -- who is asked
/// which card to discard, not the controller. Ancestral Recall at the
/// opponent, who holds two cards: three questions, all put to them.
#[test]
fn the_opponents_draws_are_replaced_too_and_they_choose_the_discard() {
    let mut engine = kit::duel(
        4505,
        &[kit::chains(), island()],
        &[kit::recall(), forest()],
        &[],
        &[swamp(), plains()],
    );
    let kept = in_hand(&engine, kit::P0, forest()).expect("the Forest");
    let library_before = kit::library(&engine, kit::P1).len();
    kit::recall_at(&mut engine, kit::P1);
    for round in 1..=3 {
        let asked = kit::question(&engine);
        assert_eq!(
            (asked.player, asked.min, asked.max, asked.prompt),
            (kit::P1, 1, 1, ChoicePrompt::Discard),
            "discard {round}: the drawing player is asked, not the Chains' controller"
        );
        assert_eq!(
            kit::sorted(&asked.options),
            kit::sorted(&kit::hand(&engine, kit::P1)),
            "over the opponent's own hand"
        );
        kit::reply(&mut engine, &[asked.options[0]]);
        kit::run_stack(&mut engine);
    }
    assert!(!kit::is_asking(&engine));
    assert_eq!(
        kit::hand(&engine, kit::P1).len(),
        2,
        "each discard was followed by a draw"
    );
    assert_eq!(kit::graveyard(&engine, kit::P1).len(), 3, "three discarded");
    assert_eq!(
        kit::library(&engine, kit::P1).len(),
        library_before - 3,
        "and three cards drawn"
    );
    assert_eq!(
        kit::hand(&engine, kit::P0),
        vec![kept],
        "the controller's hand is untouched: nothing of theirs was discarded"
    );
}

/// "except the first one they draw in each of their *draw steps*": a card
/// drawn in the upkeep is not a draw-step draw and is replaced, and it does
/// not use up the draw step's exemption. The opponent's Phyrexian Arena draws
/// in their upkeep with an empty hand: the Chains mill that card and no card
/// is drawn (the life is lost all the same), and then their draw step's
/// first card is drawn untouched.
#[test]
fn a_draw_in_the_upkeep_is_replaced_and_the_draw_steps_first_card_is_still_exempt() {
    let mut engine = kit::duel(4506, &[kit::chains()], &[], &[kit::arena()], &[]);
    let theirs = kit::top(&engine, kit::P1, 2);
    assert!(
        !kit::play_until(&mut engine, kit::first_main_of(kit::P1, 2)),
        "an empty hand and then an exempt draw: no question anywhere"
    );
    assert_eq!(
        kit::graveyard(&engine, kit::P1),
        vec![theirs[0]],
        "the upkeep's draw was a mill: the top card is in the graveyard"
    );
    assert_eq!(
        kit::hand(&engine, kit::P1),
        vec![theirs[1]],
        "the draw step's card is the one after it, drawn"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the Arena's life loss is not part of the replaced draw"
    );
}

/// The card's ruling: "If there are two of these on the battlefield, each of
/// them will modify each draw, and will cause the player to discard or to
/// 'mill' a card ... Once the player fails to discard and instead 'mills' a
/// card, all further effects of additional Chains of Mephistopheles will not
/// do anything." Two Chains, one card in hand, Ancestral Recall: the first
/// draw is discarded for by the first Chains and milled for by the second;
/// the other two draws are milled for by the first and no further. Three
/// cards leave the library, not six.
#[test]
fn two_chains_discard_if_they_can_and_after_a_mill_the_second_does_nothing() {
    let mut engine = kit::duel(
        4507,
        &[kit::chains(), kit::chains(), island()],
        &[kit::recall(), swamp()],
        &[],
        &[],
    );
    let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
    let library = kit::top(&engine, kit::P0, 60);
    kit::recall_at(&mut engine, kit::P0);
    assert!(!kit::is_asking(&engine));
    assert!(kit::hand(&engine, kit::P0).is_empty(), "no card was drawn");
    let grave = kit::graveyard(&engine, kit::P0);
    assert!(grave.contains(&swamp), "the Swamp was discarded");
    for milled in &library[..3] {
        assert!(grave.contains(milled), "the top three cards were milled");
    }
    assert_eq!(
        kit::library(&engine, kit::P0).len(),
        library.len() - 3,
        "one mill per draw, not one per Chains"
    );
}

/// The draw a Chains leave behind is a draw (CR 616.2: a replacement can make
/// another applicable), so a dredge card the Chains just made the player
/// discard may replace it: the Thug, discarded, is offered; taken, the player
/// ends with no card drawn and four milled. Thrown away a Swamp instead, the
/// Thug stays in hand, nothing is offered and the draw is made.
#[test]
fn a_thug_discarded_to_the_chains_may_be_dredged_for_the_draw_they_leave() {
    for discard_thug in [true, false] {
        let mut engine = kit::duel(4508, &chains_and_tome(), &[kit::thug(), swamp()], &[], &[]);
        let library = kit::top(&engine, kit::P0, 60);
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);

        let thug = in_hand(&engine, kit::P0, kit::thug()).expect("the Thug");
        let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
        let asked = kit::question(&engine);
        assert_eq!(
            (asked.player, asked.min, asked.max, asked.prompt),
            (kit::P0, 1, 1, ChoicePrompt::Discard)
        );
        assert_eq!(kit::sorted(&asked.options), kit::sorted(&[thug, swamp]));
        kit::reply(&mut engine, &[if discard_thug { thug } else { swamp }]);
        kit::run_stack(&mut engine);

        if discard_thug {
            let asked = kit::question(&engine);
            let buried = in_graveyard(&engine, kit::P0, kit::thug()).expect("discarded");
            assert_eq!(
                (
                    asked.player,
                    asked.options,
                    asked.min,
                    asked.max,
                    asked.prompt
                ),
                (kit::P0, vec![buried], 0, 1, ChoicePrompt::Dredge),
                "the draw the Chains left is offered the card they discarded"
            );
            kit::reply(&mut engine, &[buried]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            let mut expected = vec![
                in_hand(&engine, kit::P0, kit::thug()).expect("the Thug is back"),
                swamp,
            ];
            expected.sort_unstable();
            assert_eq!(
                kit::sorted(&kit::hand(&engine, kit::P0)),
                expected,
                "the Thug is back in hand, in the place of the draw"
            );
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..4]),
                "four cards were milled, and the Swamp was never discarded"
            );
        } else {
            assert!(
                !kit::is_asking(&engine),
                "a Thug in hand dredges nothing: the draw is made"
            );
            assert_eq!(
                kit::sorted(&kit::hand(&engine, kit::P0)),
                kit::sorted(&[thug, library[0]])
            );
            assert_eq!(kit::graveyard(&engine, kit::P0), vec![swamp]);
        }
    }
}

/// CR 616.1: with a dredge card in the graveyard and Chains in play, the
/// drawing player chooses which replacement applies first, and the question
/// offers the Thug and the Chains. Dredge first leaves no draw for the
/// Chains: nothing is discarded. Chains first discards a card, and the draw
/// they leave is offered the dredge again, which is declined here and drawn.
#[test]
fn the_drawing_player_chooses_between_dredge_and_the_chains() {
    for dredge_first in [true, false] {
        let mut engine = kit::duel(
            4509,
            &chains_and_tome(),
            &[kit::thug(), swamp(), plains()],
            &[],
            &[],
        );
        hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
        let chains = kit::permanents(&engine, kit::P0, kit::chains())[0];
        let library = kit::top(&engine, kit::P0, 60);
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);

        let asked = kit::question(&engine);
        assert_eq!(
            (asked.player, asked.min, asked.max, asked.prompt),
            (kit::P0, 1, 1, ChoicePrompt::DrawReplacement)
        );
        assert_eq!(
            kit::sorted(&asked.options),
            kit::sorted(&[thug, chains]),
            "the Thug and the Chains are the two to order"
        );
        let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
        let plains = in_hand(&engine, kit::P0, plains()).expect("the Plains");
        if dredge_first {
            kit::reply(&mut engine, &[thug]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            assert_eq!(
                kit::sorted(&kit::hand(&engine, kit::P0)),
                kit::sorted(&[
                    swamp,
                    plains,
                    in_hand(&engine, kit::P0, kit::thug()).expect("the Thug is back"),
                ]),
                "no discard: the dredge left no draw for the Chains"
            );
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..4])
            );
        } else {
            kit::reply(&mut engine, &[chains]);
            let discard = kit::question(&engine);
            assert_eq!(discard.prompt, ChoicePrompt::Discard);
            assert_eq!(kit::sorted(&discard.options), kit::sorted(&[swamp, plains]));
            kit::reply(&mut engine, &[swamp]);
            let again = kit::question(&engine);
            assert_eq!(
                (again.options, again.prompt),
                (vec![thug], ChoicePrompt::Dredge),
                "the draw the Chains left is offered the dredge again"
            );
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            assert_eq!(
                kit::sorted(&kit::hand(&engine, kit::P0)),
                kit::sorted(&[plains, library[0]]),
                "declined: the Swamp was discarded and the top card drawn"
            );
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&[thug, swamp])
            );
        }
    }
}
