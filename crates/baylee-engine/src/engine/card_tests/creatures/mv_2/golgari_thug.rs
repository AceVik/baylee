//! `cards/creatures/mv_2/golgari_thug.rs`, played.
//!
//! Golgari Thug, {1}{B}, Creature -- Human Warrior, 1/1:
//! "When this creature dies, put target creature card from your graveyard on
//! top of your library." / "Dredge 4"
//!
//! Written from the printed text, the cards' rulings and the Comprehensive
//! Rules (700.4 "dies", 603.3d targets, 702.52 dredge, 121.2 and 121.6
//! multiple and replaced draws), not from the card.
//!
//! A dredge card is found by a draw: every test that has one in a graveyard
//! makes the draw with a real card (Jayemdae Tome for one, Ancestral Recall
//! for three) and answers the question the engine asks about it. Every test
//! that proves something is *not* offered stands beside the case that is, in
//! the same function, so a card that offered dredge nowhere, or everywhere,
//! fails one of the two.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::draw_kit as kit;

/// Destroys the Thug on the battlefield the way a state-based action does and
/// lets the engine take its step. Says what the dies trigger asked, if it
/// asked a target question: the options it put to its controller. Whatever
/// else is on the stack is left there.
#[track_caller]
fn the_thug_dies(engine: &mut Engine<RegistryLookup>) -> Option<Vec<ObjectId>> {
    let thug = on_battlefield(engine, kit::P0, kit::thug()).expect("the Thug is out");
    let state = engine
        .dev_state_mut(kit::P0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, thug);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
    next_question_or_quiet(engine)
}

/// Passes priority until the stack is empty, or stops on a target question
/// and hands back its options.
#[track_caller]
fn next_question_or_quiet(engine: &mut Engine<RegistryLookup>) -> Option<Vec<ObjectId>> {
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, kit::P0, "the trigger's controller chooses");
                return Some(options);
            }
            Pending::Priority { player, .. } => {
                if stack_is_empty(engine) {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the trigger resolves: {other:?}"),
        }
    }
    panic!("never settled: {:?}", engine.pending());
}

/// Names `target` for the trigger on the table and lets it resolve.
#[track_caller]
fn aim_the_trigger_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the trigger takes a legal target");
    assert!(
        next_question_or_quiet(engine).is_none(),
        "the trigger asks for its target once"
    );
}

/// "When this creature dies, put target creature card from your graveyard on
/// top of your library."
///
/// The graveyard holds a creature card, a land card and, in the opponent's
/// graveyard, a creature card. Only the first is "a creature card from your
/// graveyard" besides the Thug's own corpse: the menu is exactly those two,
/// and the card chosen is the top card of the library afterwards. The land
/// and the opponent's creature card, still where they were, are what keep
/// "creature card" and "your graveyard" from being decoration.
#[test]
fn when_the_thug_dies_it_puts_a_creature_card_from_your_graveyard_on_top_of_your_library() {
    let mut engine = kit::duel(
        4301,
        &[kit::thug()],
        &[kit::bears(), forest()],
        &[],
        &[kit::bears()],
    );
    hand_to_graveyard(&mut engine, kit::P0, kit::bears());
    hand_to_graveyard(&mut engine, kit::P0, forest());
    hand_to_graveyard(&mut engine, kit::P1, kit::bears());
    // Cards that reach a graveyard from a hand did not die (CR 700.4): the
    // Thug's trigger is the Thug's alone, and nothing has been put anywhere.
    assert!(stack_is_empty(&engine));
    let library_before = kit::library(&engine, kit::P0).len();

    let options = the_thug_dies(&mut engine)
        .expect("two creature cards are in the graveyard, so the target is a choice");
    let corpse = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug died");
    let bears = in_graveyard(&engine, kit::P0, kit::bears()).expect("the Bears wait");
    let land = in_graveyard(&engine, kit::P0, forest()).expect("the Forest waits");
    let theirs = in_graveyard(&engine, kit::P1, kit::bears()).expect("the opponent's Bears wait");
    assert_eq!(
        kit::sorted(&options),
        kit::sorted(&[corpse, bears]),
        "the creature cards of your own graveyard, the Thug's corpse among them: \
         not the land {land:?}, not the opponent's creature card {theirs:?}"
    );

    aim_the_trigger_at(&mut engine, bears);
    let top = *kit::library(&engine, kit::P0)
        .last()
        .expect("the library has a top");
    assert_eq!(kit::card_of(&engine, top), Some(kit::bears()), "on top");
    assert_eq!(
        kit::library(&engine, kit::P0).len(),
        library_before + 1,
        "one card was put there"
    );
    assert!(
        in_graveyard(&engine, kit::P0, kit::bears()).is_none(),
        "and it left the graveyard"
    );
    assert!(
        in_graveyard(&engine, kit::P0, kit::thug()).is_some()
            && in_graveyard(&engine, kit::P0, forest()).is_some()
            && in_graveyard(&engine, kit::P1, kit::bears()).is_some(),
        "nothing else moved"
    );
}

/// "target creature card" is not "another creature card": the Thug's own
/// corpse is in the graveyard when the trigger's target is chosen (CR 603.3d,
/// 400.7) and may be the target. Alone it is the only one there is; beside
/// another creature card it is one of two options, and naming it puts the
/// Thug, not the other card, on top.
#[test]
fn the_thug_may_put_itself_back_on_top_of_the_library() {
    for company in [false, true] {
        let hand: &[CardIndex] = if company { &[kit::bears()] } else { &[] };
        let mut engine = kit::duel(4302, &[kit::thug()], hand, &[], &[]);
        if company {
            hand_to_graveyard(&mut engine, kit::P0, kit::bears());
        }
        let asked = the_thug_dies(&mut engine);
        let corpse = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug died");
        let options = asked.expect("a target is chosen as the trigger goes on the stack");
        assert!(
            options.contains(&corpse),
            "the corpse is a legal target: {options:?}"
        );
        assert_eq!(
            options.len(),
            if company { 2 } else { 1 },
            "alone the Thug is the only creature card there is: {options:?}"
        );
        aim_the_trigger_at(&mut engine, corpse);
        let top = *kit::library(&engine, kit::P0)
            .last()
            .expect("the library has a top");
        assert_eq!(
            kit::card_of(&engine, top),
            Some(kit::thug()),
            "company: {company}: the Thug is the top card of its own library"
        );
        assert!(in_graveyard(&engine, kit::P0, kit::thug()).is_none());
        assert_eq!(
            in_graveyard(&engine, kit::P0, kit::bears()).is_some(),
            company,
            "the other creature card stayed where it was"
        );
    }
}

/// Dredge 4 (CR 702.52a): "if you would draw a card, you may instead mill
/// four cards and return this card from your graveyard to your hand." The
/// question is asked of the drawing player with the Thug on the menu and
/// naming none allowed. Taking it mills the top four, returns the Thug and
/// draws nothing; declining is drawing.
#[test]
fn dredge_4_mills_four_and_returns_the_thug_instead_of_a_draw_and_may_be_declined() {
    for dredge in [true, false] {
        let mut engine = kit::duel(4303, &kit::tome_board(), &[kit::thug()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
        let library = kit::top(&engine, kit::P0, 60);
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);

        let asked = kit::question(&engine);
        assert_eq!(
            (
                asked.player,
                asked.options,
                asked.min,
                asked.max,
                asked.prompt
            ),
            (kit::P0, vec![thug], 0, 1, ChoicePrompt::Dredge),
            "a draw with the Thug in the graveyard is put to its player"
        );
        if dredge {
            kit::reply(&mut engine, &[thug]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            assert_eq!(
                kit::hand(&engine, kit::P0),
                vec![thug],
                "the Thug is in hand and nothing else is"
            );
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..4]),
                "the top four cards of the library were milled, in their stead"
            );
            assert_eq!(kit::library(&engine, kit::P0).len(), library.len() - 4);
            assert_eq!(
                kit::drawn_this_turn(&engine, kit::P0),
                0,
                "no card was drawn: the draw was replaced"
            );
        } else {
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            assert_eq!(
                kit::hand(&engine, kit::P0),
                vec![library[0]],
                "declined: the top card was drawn"
            );
            assert_eq!(
                kit::graveyard(&engine, kit::P0),
                vec![thug],
                "and the Thug stays where it was, with nothing milled"
            );
            assert_eq!(kit::drawn_this_turn(&engine, kit::P0), 1);
        }
    }
}

/// CR 702.52a: "As long as you have at least N cards in your library" -- and
/// 702.52b: with fewer, no card can be milled this way. With four cards in the
/// library the first card of "draw three" is offered the dredge; once it has
/// been declined there are three, and the other two draws are made without
/// a question. With three to begin with nothing is asked at all.
#[test]
fn dredge_4_is_offered_only_while_the_library_holds_four_cards() {
    for (cards, questions) in [(4_usize, 1_usize), (3, 0)] {
        let mut engine = kit::duel(4304, &[island()], &[kit::recall(), kit::thug()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
        let all = kit::library(&engine, kit::P0).len();
        seed_graveyard(&mut engine, kit::P0, all - cards);
        let library = kit::top(&engine, kit::P0, cards);
        assert_eq!(library.len(), cards);

        kit::recall_at(&mut engine, kit::P0);
        let mut asked = 0;
        while kit::is_asking(&engine) {
            let q = kit::question(&engine);
            assert_eq!((q.options, q.prompt), (vec![thug], ChoicePrompt::Dredge));
            asked += 1;
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
        }
        assert_eq!(
            asked, questions,
            "{cards} cards in the library: the Thug is asked about {questions} time(s)"
        );
        assert_eq!(
            kit::sorted(&kit::hand(&engine, kit::P0)),
            kit::sorted(&library[..3]),
            "three cards were drawn from the top, every dredge declined or not offered"
        );
        assert_eq!(kit::library(&engine, kit::P0).len(), cards - 3);
    }
}

/// CR 121.2 and the cards' ruling: "If you're drawing multiple cards, each
/// draw is performed one at a time." Ancestral Recall's three cards are three
/// draws and the Thug is asked about each: declined twice, taken for the
/// third. The cards drawn are the first two and the Thug comes back for the
/// last, with the next four milled.
#[test]
fn each_card_of_a_multiple_draw_is_offered_the_dredge_on_its_own() {
    let mut engine = kit::duel(4305, &[island()], &[kit::recall(), kit::thug()], &[], &[]);
    hand_to_graveyard(&mut engine, kit::P0, kit::thug());
    let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
    let library = kit::top(&engine, kit::P0, 60);

    kit::recall_at(&mut engine, kit::P0);
    for (draw, take) in [(1, false), (2, false), (3, true)] {
        let asked = kit::question(&engine);
        assert_eq!(
            (asked.options, asked.min, asked.max, asked.prompt),
            (vec![thug], 0, 1, ChoicePrompt::Dredge),
            "the Thug is asked about draw {draw} of 3"
        );
        let answer = if take { vec![thug] } else { vec![] };
        kit::reply(&mut engine, &answer);
        kit::run_stack(&mut engine);
    }
    assert!(!kit::is_asking(&engine));
    let mut expected = vec![library[0], library[1], thug];
    expected.sort_unstable();
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        expected,
        "two cards drawn, then the Thug instead of the third"
    );
    for milled in &library[2..6] {
        assert!(
            kit::graveyard(&engine, kit::P0).contains(milled),
            "cards three to six from the top were milled"
        );
    }
}

/// The cards' ruling: "another card with a dredge ability (including one that
/// was milled by the first dredge ability) may be used to replace the second
/// draw." A Dakmor Salvage on top of the library is milled by the Thug's
/// dredge 4 and is then itself offered for the second card of Ancestral
/// Recall; its dredge 2 mills the next two cards, and the third card is drawn
/// from what follows.
#[test]
fn a_dredge_card_milled_by_the_first_dredge_may_replace_the_second_draw() {
    let mut engine = kit::duel(
        4306,
        &[island()],
        &[kit::recall(), kit::thug(), kit::salvage()],
        &[],
        &[],
    );
    hand_to_graveyard(&mut engine, kit::P0, kit::thug());
    hand_to_library_top(&mut engine, kit::P0, kit::salvage());
    let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
    let library = kit::top(&engine, kit::P0, 60);

    kit::recall_at(&mut engine, kit::P0);
    let first = kit::question(&engine);
    assert_eq!(
        (first.options, first.prompt),
        (vec![thug], ChoicePrompt::Dredge),
        "the Salvage is in the library, not the graveyard: only the Thug is offered"
    );
    kit::reply(&mut engine, &[thug]);
    kit::run_stack(&mut engine);

    let salvage = in_graveyard(&engine, kit::P0, kit::salvage())
        .expect("the Thug's dredge milled the Salvage from the top of the library");
    let second = kit::question(&engine);
    assert_eq!(
        (second.options, second.min, second.max, second.prompt),
        (vec![salvage], 0, 1, ChoicePrompt::Dredge),
        "the second draw may be replaced by the card the first dredge milled"
    );
    kit::reply(&mut engine, &[salvage]);
    kit::run_stack(&mut engine);

    assert!(!kit::is_asking(&engine), "the third card is drawn plainly");
    let thug_back = in_hand(&engine, kit::P0, kit::thug()).expect("the Thug is back");
    let salvage_back = in_hand(&engine, kit::P0, kit::salvage()).expect("the Salvage is back");
    let mut expected = vec![thug_back, salvage_back, library[6]];
    expected.sort_unstable();
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        expected,
        "the two dredge cards and the seventh card from the top: 4 + 2 milled, then one drawn"
    );
    for milled in &library[1..6] {
        assert!(
            kit::graveyard(&engine, kit::P0).contains(milled),
            "the cards under the Salvage, milled four and then two at a time"
        );
    }
}

/// CR 702.52a: dredge "functions only while the card with dredge is in a
/// player's graveyard". On the battlefield and in the hand the Thug is not
/// asked about, and the draw is made; in the graveyard, the same draw is put
/// to its player.
#[test]
fn a_thug_on_the_battlefield_or_in_hand_dredges_nothing() {
    for place in ["battlefield", "hand", "graveyard"] {
        let mut board = kit::tome_board();
        let mut hand = Vec::new();
        match place {
            "battlefield" => board.push(kit::thug()),
            _ => hand.push(kit::thug()),
        }
        let mut engine = kit::duel(4307, &board, &hand, &[], &[]);
        if place == "graveyard" {
            hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        }
        let top = *kit::library(&engine, kit::P0).last().expect("a library");
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);
        assert_eq!(
            kit::is_asking(&engine),
            place == "graveyard",
            "the Thug in its {place}: dredge is offered only from the graveyard"
        );
        if place == "graveyard" {
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
        }
        assert!(
            kit::hand(&engine, kit::P0).contains(&top),
            "{place}: the top card was drawn (declined, where it was offered)"
        );
    }
}

/// CR 702.52a: "if *you* would draw a card" -- the Thug in a graveyard serves
/// its owner's draws and no one else's. Ancestral Recall cast at the opponent
/// makes them draw three and nobody is asked; the same spell cast at the
/// Thug's owner asks.
#[test]
fn a_thug_in_my_graveyard_is_not_offered_to_the_opponent_drawing() {
    for target in [kit::P1, kit::P0] {
        let mut engine = kit::duel(4308, &[island()], &[kit::recall(), kit::thug()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        kit::recall_at(&mut engine, target);
        assert_eq!(
            kit::is_asking(&engine),
            target == kit::P0,
            "Ancestral Recall at {target:?}"
        );
        if target == kit::P1 {
            assert_eq!(
                kit::hand(&engine, kit::P1).len(),
                3,
                "the opponent drew three cards without a question"
            );
            assert!(in_graveyard(&engine, kit::P0, kit::thug()).is_some());
        }
    }
}

/// Dredge replaces the draw of the draw step as it replaces any other (the
/// cards' ruling: "Dredge can replace any card draw, not only the one during
/// your draw step"). Nothing is asked in turn 1 (the player on the play
/// skips the draw, CR 103.8a) or in the opponent's draw step; at the Thug's
/// owner's first draw step of the game the question comes, in that step.
#[test]
fn dredge_4_is_offered_in_the_draw_step_too() {
    for dredge in [true, false] {
        let mut engine = kit::duel(4309, &[], &[kit::thug()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, kit::thug());
        let thug = in_graveyard(&engine, kit::P0, kit::thug()).expect("the Thug is buried");
        let library = kit::top(&engine, kit::P0, 60);

        assert!(
            kit::play_until(&mut engine, |e| e.state().turn.number > 3),
            "a draw is put to its player, by the owner's first draw step"
        );
        assert_eq!(engine.state().turn.number, 3, "the owner's first draw step");
        assert!(kit::in_draw_step_of(&engine, kit::P0));
        assert!(
            kit::hand(&engine, kit::P0).is_empty(),
            "nothing has been drawn yet"
        );
        let asked = kit::question(&engine);
        assert_eq!(
            (
                asked.player,
                asked.options,
                asked.min,
                asked.max,
                asked.prompt
            ),
            (kit::P0, vec![thug], 0, 1, ChoicePrompt::Dredge)
        );
        let answer = if dredge { vec![thug] } else { vec![] };
        kit::reply(&mut engine, &answer);
        assert!(
            !kit::play_until(&mut engine, kit::first_main_of(kit::P0, 3)),
            "and that was the only question of the turn"
        );
        if dredge {
            assert_eq!(kit::hand(&engine, kit::P0), vec![thug]);
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..4])
            );
        } else {
            assert_eq!(kit::hand(&engine, kit::P0), vec![library[0]]);
            assert_eq!(kit::graveyard(&engine, kit::P0), vec![thug]);
        }
    }
}
