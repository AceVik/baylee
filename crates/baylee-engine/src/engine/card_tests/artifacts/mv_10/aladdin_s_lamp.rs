//! `cards/artifacts/mv_10/aladdin_s_lamp.rs`, played.
//!
//! Aladdin's Lamp, {10} Artifact: "{X}, {T}: The next time you would draw a
//! card this turn, instead look at the top X cards of your library, put all
//! but one of them on the bottom of your library in a random order, then draw
//! a card. X can't be 0."
//!
//! Written from the printed text, the card's rulings and the Comprehensive
//! Rules (107.3 X, 121.2 and 121.6 a draw at a time and replaced draws,
//! 611.2a "this turn", 514.2 effects of a turn end, 616.1 the order of two
//! replacements), not from the card.
//!
//! The cards that draw are real ones: Jayemdae Tome (one draw), Ancestral
//! Recall (three). The library is Forests, so a card is told apart by where
//! it lay in the library when the test began.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::draw_kit as kit;

/// The Lamp, the Tome, and `forests` Forests to pay for both.
fn lamp_and_tome(forests: usize) -> Vec<CardIndex> {
    let mut board = vec![kit::lamp(), kit::tome()];
    board.extend(std::iter::repeat_n(forest(), forests));
    board
}

/// "{X}, {T}" and "X can't be 0" (CR 107.3k): the X is announced with the
/// activation, from one up to what the floating mana pays, and neither
/// nought nor more than that is accepted. Without mana the Lamp cannot be
/// activated at all: the cheapest X is one.
#[test]
fn x_is_announced_from_one_up_to_what_the_mana_pays_and_never_nought() {
    let mut engine = kit::duel(4601, &[kit::lamp(), forest(), forest()], &[], &[], &[]);
    let lamp = on_battlefield(&engine, kit::P0, kit::lamp()).expect("the Lamp is out");
    assert!(
        engine
            .apply(
                kit::P0,
                PlayerAction::ActivateAbility {
                    source: lamp,
                    ability_index: 0
                }
            )
            .is_err(),
        "no mana, no X of one: the Lamp cannot be activated"
    );

    tap_all_mana(&mut engine, kit::P0);
    activate(&mut engine, kit::P0, kit::lamp(), 0);
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("X is announced: {:?}", engine.pending())
    };
    assert_eq!(
        (player, min, max),
        (kit::P0, 1, 2),
        "X from 1 to the 2 mana"
    );
    assert!(
        engine
            .apply(kit::P0, PlayerAction::ChooseNumber(0))
            .is_err(),
        "X can't be 0"
    );
    assert!(
        engine
            .apply(kit::P0, PlayerAction::ChooseNumber(3))
            .is_err(),
        "nor more than the pool pays"
    );
    engine
        .apply(kit::P0, PlayerAction::ChooseNumber(2))
        .expect("X = 2");
}

/// The whole sentence: with the Lamp lit for three, the first card of
/// Ancestral Recall's three is replaced -- the top three cards are looked at,
/// the player names the two that go to the bottom (all but one, the
/// question's bounds say exactly two), and the one left on top is drawn. It is
/// the *next* draw only: the second and third cards come off the library
/// plainly, from what follows the three.
#[test]
fn the_next_draw_keeps_one_of_the_top_three_and_only_that_draw_is_changed() {
    let mut engine = kit::duel(
        4602,
        &[kit::lamp(), island(), forest(), forest(), forest()],
        &[kit::recall()],
        &[],
        &[],
    );
    let forests = kit::permanents(&engine, kit::P0, forest());
    kit::tap_lands_of(&mut engine, &forests);
    kit::light_the_lamp(&mut engine, 3);
    let [t1, t2, t3, t4, t5]: [ObjectId; 5] = kit::top(&engine, kit::P0, 5).try_into().unwrap();

    kit::recall_at(&mut engine, kit::P0);
    let asked = kit::question(&engine);
    assert_eq!(
        (asked.player, asked.min, asked.max, asked.prompt),
        (kit::P0, 2, 2, ChoicePrompt::PutOnBottom),
        "all but one of three: exactly two go to the bottom"
    );
    assert_eq!(
        kit::sorted(&asked.options),
        kit::sorted(&[t1, t2, t3]),
        "the top three cards of the library are the ones looked at"
    );
    kit::reply(&mut engine, &[t1, t2]);
    kit::run_stack(&mut engine);

    assert!(
        !kit::is_asking(&engine),
        "the Lamp is spent by the first card: the other two are drawn plainly"
    );
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        kit::sorted(&[t3, t4, t5]),
        "the card kept, then the two after the three that were looked at"
    );
    assert_eq!(
        kit::sorted(&kit::library(&engine, kit::P0)[..2]),
        kit::sorted(&[t1, t2]),
        "the two named are the bottom two cards of the library"
    );
}

/// X is the number of cards looked at: X of one has nothing to put on the
/// bottom, so nothing is asked and the top card is drawn; two, three and four
/// ask for X - 1 of the top X. Beside each other so that a Lamp that always
/// looked at three, or never asked, fails one of them.
#[test]
fn x_is_the_number_of_cards_looked_at() {
    for x in [1_u32, 2, 4] {
        let mut engine = kit::duel(4603, &lamp_and_tome(x as usize + 4), &[], &[], &[]);
        tap_all_mana(&mut engine, kit::P0);
        kit::light_the_lamp(&mut engine, x);
        let looked = kit::top(&engine, kit::P0, x as usize);
        kit::tome_draw(&mut engine);

        if x == 1 {
            assert!(
                !kit::is_asking(&engine),
                "one card looked at, none to put away: no question"
            );
            assert_eq!(kit::hand(&engine, kit::P0), looked, "the top card is drawn");
            continue;
        }
        let asked = kit::question(&engine);
        let bottom = u8::try_from(x - 1).unwrap();
        assert_eq!(
            (asked.player, asked.min, asked.max, asked.prompt),
            (kit::P0, bottom, bottom, ChoicePrompt::PutOnBottom),
            "X = {x}: all but one go to the bottom"
        );
        assert_eq!(
            kit::sorted(&asked.options),
            kit::sorted(&looked),
            "X = {x}: the top {x} cards are looked at"
        );
        let kept = *looked.last().unwrap();
        kit::reply(&mut engine, &looked[..looked.len() - 1]);
        kit::run_stack(&mut engine);
        assert!(!kit::is_asking(&engine));
        assert_eq!(
            kit::hand(&engine, kit::P0),
            vec![kept],
            "X = {x}: the kept card"
        );
        assert_eq!(
            kit::sorted(&kit::library(&engine, kit::P0)[..looked.len() - 1]),
            kit::sorted(&looked[..looked.len() - 1]),
            "X = {x}: the rest are at the bottom"
        );
    }
}

/// "the next time you would draw a card *this turn*": a Lamp lit in a turn
/// in which its controller draws nothing does nothing in the next turn
/// (611.2a, 514.2). The same Lamp, lit the same way, replaces a draw made
/// before the turn ends; and at the owner's next draw step the draw is made
/// without a question.
#[test]
fn the_effect_ends_with_the_turn_if_no_card_was_drawn() {
    for draw_this_turn in [true, false] {
        let mut engine = kit::duel(4604, &lamp_and_tome(6), &[], &[], &[]);
        tap_all_mana(&mut engine, kit::P0);
        kit::light_the_lamp(&mut engine, 2);
        let top = kit::top(&engine, kit::P0, 2);
        if draw_this_turn {
            kit::tome_draw(&mut engine);
            let asked = kit::question(&engine);
            assert_eq!(asked.prompt, ChoicePrompt::PutOnBottom, "this turn's draw");
            assert_eq!(kit::sorted(&asked.options), kit::sorted(&top));
        } else {
            assert!(
                !kit::play_until(&mut engine, kit::first_main_of(kit::P0, 3)),
                "turn 3's draw step is a plain draw: the Lamp was only for turn 1"
            );
            assert_eq!(
                kit::hand(&engine, kit::P0),
                vec![top[0]],
                "the top card, drawn as it lay"
            );
        }
    }
}

/// "the next time *you* would draw": the opponent's draws are not the Lamp's
/// business and do not use it up. Lit for two, with the opponent drawing
/// three from Ancestral Recall first: they are asked nothing and draw all
/// three; the Lamp's controller's next draw, a Tome's, then looks at two.
#[test]
fn only_its_controllers_draw_is_replaced_and_the_opponents_do_not_spend_it() {
    let mut engine = kit::duel(
        4605,
        &[
            kit::lamp(),
            kit::tome(),
            island(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
        &[kit::recall()],
        &[],
        &[],
    );
    let forests = kit::permanents(&engine, kit::P0, forest());
    kit::tap_lands_of(&mut engine, &forests);
    kit::light_the_lamp(&mut engine, 2);

    kit::recall_at(&mut engine, kit::P1);
    assert!(!kit::is_asking(&engine), "the opponent's draws are plain");
    assert_eq!(kit::hand(&engine, kit::P1).len(), 3, "they drew all three");

    let top = kit::top(&engine, kit::P0, 2);
    kit::tome_draw(&mut engine);
    let asked = kit::question(&engine);
    assert_eq!(
        (asked.player, asked.min, asked.max, asked.prompt),
        (kit::P0, 1, 1, ChoicePrompt::PutOnBottom),
        "the Lamp's controller's own draw is still the next one"
    );
    assert_eq!(kit::sorted(&asked.options), kit::sorted(&top));
}

/// The card's ruling: "If you use more than one Lamp effect, the second one
/// will end up replacing the card being drawn with the first one." Two Lamps
/// lit for two: the draw looks at the top two (A, B), one goes to the bottom
/// (A); the card the first would draw (B) is replaced by the second, which
/// looks at the top two again (B, C) -- the card the first one kept is among
/// them -- puts one on the bottom (C) and draws the other (B).
#[test]
fn a_second_lamp_replaces_the_card_the_first_one_would_draw() {
    let mut engine = kit::duel(
        4606,
        &[
            kit::lamp(),
            kit::lamp(),
            kit::tome(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
        &[],
        &[],
        &[],
    );
    tap_all_mana(&mut engine, kit::P0);
    kit::light_the_lamp(&mut engine, 2);
    kit::light_the_lamp(&mut engine, 2);
    let [a, b, c]: [ObjectId; 3] = kit::top(&engine, kit::P0, 3).try_into().unwrap();

    kit::tome_draw(&mut engine);
    let first = kit::question(&engine);
    assert_eq!(
        (first.min, first.max, first.prompt),
        (1, 1, ChoicePrompt::PutOnBottom)
    );
    assert_eq!(kit::sorted(&first.options), kit::sorted(&[a, b]));
    kit::reply(&mut engine, &[a]);

    let second = kit::question(&engine);
    assert_eq!(
        (second.player, second.min, second.max, second.prompt),
        (kit::P0, 1, 1, ChoicePrompt::PutOnBottom),
        "the card the first Lamp would draw is looked at again by the second"
    );
    assert_eq!(
        kit::sorted(&second.options),
        kit::sorted(&[b, c]),
        "the top two cards again: the card kept, and the one under it"
    );
    kit::reply(&mut engine, &[c]);
    kit::run_stack(&mut engine);
    assert!(!kit::is_asking(&engine));
    assert_eq!(kit::hand(&engine, kit::P0), vec![b], "the card both kept");
    assert_eq!(
        kit::sorted(&kit::library(&engine, kit::P0)[..2]),
        kit::sorted(&[a, c]),
        "one card from each look went to the bottom"
    );
}

/// CR 616.1: two replacements of one draw that do different things -- the
/// Lamp's look and Chains of Mephistopheles' discard -- are ordered by the
/// drawing player, and the question offers the two sources. Either order ends
/// the same, with exactly one discard (a replacement gets one opportunity at
/// an event, 614.5): the card named stays, the other is discarded and the card
/// kept from the Lamp's three is drawn. What differs is the order of the
/// questions: look first and then the discard, or discard first and then the
/// look.
#[test]
fn the_drawing_player_orders_the_lamp_and_the_chains() {
    for lamp_first in [true, false] {
        let mut board = lamp_and_tome(7);
        board.push(kit::chains());
        let mut engine = kit::duel(4607, &board, &[swamp(), plains()], &[], &[]);
        tap_all_mana(&mut engine, kit::P0);
        kit::light_the_lamp(&mut engine, 3);
        let lamp = kit::permanents(&engine, kit::P0, kit::lamp())[0];
        let chains = kit::permanents(&engine, kit::P0, kit::chains())[0];
        let swamp = in_hand(&engine, kit::P0, swamp()).expect("the Swamp");
        let plains = in_hand(&engine, kit::P0, plains()).expect("the Plains");
        let [t1, t2, t3]: [ObjectId; 3] = kit::top(&engine, kit::P0, 3).try_into().unwrap();
        kit::tome_draw(&mut engine);

        let order = kit::question(&engine);
        assert_eq!(
            (order.player, order.min, order.max, order.prompt),
            (kit::P0, 1, 1, ChoicePrompt::DrawReplacement)
        );
        assert_eq!(
            kit::sorted(&order.options),
            kit::sorted(&[lamp, chains]),
            "the two sources are the menu"
        );
        let look = |engine: &mut Engine<RegistryLookup>| {
            let q = kit::question(engine);
            assert_eq!((q.min, q.max, q.prompt), (2, 2, ChoicePrompt::PutOnBottom));
            assert_eq!(kit::sorted(&q.options), kit::sorted(&[t1, t2, t3]));
            kit::reply(engine, &[t1, t2]);
        };
        let discard = |engine: &mut Engine<RegistryLookup>| {
            let q = kit::question(engine);
            assert_eq!((q.min, q.max, q.prompt), (1, 1, ChoicePrompt::Discard));
            assert_eq!(
                kit::sorted(&q.options),
                kit::sorted(&[swamp, plains]),
                "the hand as it stands: the card to be drawn is not in it yet"
            );
            kit::reply(engine, &[swamp]);
        };
        if lamp_first {
            kit::reply(&mut engine, &[lamp]);
            look(&mut engine);
            discard(&mut engine);
        } else {
            kit::reply(&mut engine, &[chains]);
            discard(&mut engine);
            look(&mut engine);
        }
        kit::run_stack(&mut engine);
        assert!(!kit::is_asking(&engine), "lamp first: {lamp_first}");
        assert_eq!(
            kit::sorted(&kit::hand(&engine, kit::P0)),
            kit::sorted(&[plains, t3]),
            "lamp first: {lamp_first}: the Plains stayed and the kept card was drawn"
        );
        assert_eq!(kit::graveyard(&engine, kit::P0), vec![swamp]);
        assert_eq!(
            kit::sorted(&kit::library(&engine, kit::P0)[..2]),
            kit::sorted(&[t1, t2])
        );
    }
}
