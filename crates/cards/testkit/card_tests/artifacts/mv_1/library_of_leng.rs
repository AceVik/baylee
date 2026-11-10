//! `cards/artifacts/mv_1/library_of_leng.rs`, played.
//!
//! Two sentences, two kinds of test. "You have no maximum hand size" is an
//! absence (no cleanup question), so its test has a control: the same hand
//! walked into the same cleanup step without the card. "If an effect causes
//! you to discard a card, discard it, but you may put it on top of your
//! library instead of into your graveyard" is a question
//! (`Pending::Arrange` with `ArrangePrompt::DiscardToLibrary`: a graveyard
//! pile first, then a library pile, top card first), so every test here
//! answers it itself, and the ones that matter answer it with something other
//! than the default.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn leng() -> CardIndex {
    card_index("867def48-4be8-4056-bcf1-d6b00450b9a3")
}

fn mind_twist() -> CardIndex {
    card_index("78f9c223-9982-4282-a496-a6f892f0a5bf")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn tireless_tribe() -> CardIndex {
    card_index("5dfbcdb4-d2ad-477d-b37d-db4725410b27")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn library_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

fn graveyard_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

fn hand_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// The question on the table, asked of `seat`, and the cards it names.
/// Asserts the shape a client is told: the discard-to-library prompt, the
/// graveyard pile first and the library pile second, so that "no preference"
/// is the printed discard.
#[track_caller]
fn leng_asks(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "expected Library of Leng's question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "the question goes to the discarding player");
    assert_eq!(prompt, ArrangePrompt::DiscardToLibrary);
    assert_eq!(
        piles.iter().map(|p| p.place).collect::<Vec<_>>(),
        vec![ArrangePlace::Graveyard, ArrangePlace::LibraryTop],
        "the graveyard pile comes first"
    );
    cards
}

#[track_caller]
fn answer(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    graveyard: &[ObjectId],
    top: &[ObjectId],
) {
    engine
        .apply(
            seat,
            PlayerAction::Arrange {
                piles: vec![graveyard.to_vec(), top.to_vec()],
            },
        )
        .expect("the arrangement is a legal answer");
}

/// p0 casts Mind Twist for two at p1, whose hand is three distinct
/// creatures. `p0_board` and `p1_board` hold whatever else is out (the Leng,
/// or not). Returns the engine with both players having passed, so the
/// spell is resolving or its question is up.
fn twist_at_p1(
    seed: u64,
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut p0_all = vec![swamp(), swamp(), swamp()];
    p0_all.extend_from_slice(p0_board);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &p0_all)
        .hand(0, &[mind_twist()])
        .battlefield(1, p1_board)
        .hand(1, &[bears(), serra_angel(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, mind_twist());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine
}

/// p0 casts Wheel of Fortune with three cards left in hand; p1 holds one.
/// The Leng is on `leng_for`'s battlefield.
fn wheel_with_leng_for(seed: u64, leng_for: PlayerId) -> Engine<RegistryLookup> {
    let (board0, board1): (&[CardIndex], &[CardIndex]) = if leng_for == P0 {
        (&[mountain(), mountain(), mountain(), leng()], &[])
    } else {
        (&[mountain(), mountain(), mountain()], &[leng()])
    };
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, board0)
        .battlefield(1, board1)
        .hand(
            0,
            &[wheel_of_fortune(), bears(), serra_angel(), hill_giant()],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, wheel_of_fortune());
    engine
}

/// "You have no maximum hand size." p0 holds more than seven through its
/// own cleanup step and keeps every card. (The sentence is an absence, so the
/// control for it is `a_hand_over_seven_is_cut_without_the_library_or_with_the_opponents`.)
#[test]
fn library_of_leng_leaves_no_maximum_hand_size() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[leng()])
        .hand(0, &[forest(); 9])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let held = hand_of(&engine, P0).len();
    assert!(held > 7, "more than seven in hand: {held}");
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        hand_of(&engine, P0).len(),
        held,
        "nothing discarded at cleanup"
    );
    assert!(
        graveyard_of(&engine, P0).is_empty(),
        "and nothing in the graveyard either: {:?}",
        graveyard_of(&engine, P0)
    );
}

/// Walks p0's own turn out and returns what its cleanup step asked: the
/// count, and the hand size at that moment. `None` if the turn ends without
/// a question.
fn cleanup_question(engine: &mut Engine<RegistryLookup>) -> Option<(u8, usize)> {
    for _ in 0..400 {
        match engine.pending().clone() {
            Pending::DiscardChoice { player, count } if player == P0 => {
                return Some((count, hand_of(engine, P0).len()));
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while walking the turn: {other:?}"),
        }
        if engine.state().turn.active != P0 {
            return None;
        }
    }
    panic!("the turn never ended");
}

/// The control of the hand-size sentence. The same nine-card hand reaches
/// the same cleanup step three times: with no Library of Leng, and with the
/// opponent's, the engine asks for the overflow over seven; with p0's own, it
/// asks nothing. The sentence is the controller's ("you"), not the table's.
#[test]
fn a_hand_over_seven_is_cut_without_the_library_or_with_the_opponents() {
    for (board0, board1, asked) in [
        (&[][..], &[][..], true),
        (&[][..], &[leng()][..], true),
        (&[leng()][..], &[][..], false),
    ] {
        let mut engine = Duel::new(4_900 + u64::from(asked), forest())
            .battlefield(0, board0)
            .battlefield(1, board1)
            .hand(0, &[forest(); 9])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let question = cleanup_question(&mut engine);
        if asked {
            let (count, at_cleanup) = question.expect("a hand of nine is asked to discard");
            assert_eq!(usize::from(count), at_cleanup - 7, "down to seven");
        } else {
            assert_eq!(question, None, "p0's own Leng: no cleanup question");
            assert!(hand_of(&engine, P0).len() > 7);
        }
    }
}

/// Mind Twist at a Leng's controller: both random cards are asked about
/// before they move, and both are put on top in the order the player lists
/// (the first listed on top). The graveyard stays empty, and the card the
/// player listed first is the one the player draws next.
#[test]
fn mind_twist_at_the_controller_may_put_both_cards_on_top_in_a_chosen_order() {
    let mut engine = twist_at_p1(5201, &[], &[leng()]);
    let cards = leng_asks(&engine, P1);
    assert_eq!(cards.len(), 2, "X = 2");
    for card in &cards {
        assert!(
            hand_of(&engine, P1).contains(card),
            "asked before anything moved"
        );
    }
    let (first, second) = (cards[1], cards[0]);
    answer(&mut engine, P1, &[], &[first, second]);

    let lib = library_of(&engine, P1);
    assert_eq!(
        lib[lib.len() - 2..],
        [second, first],
        "the first listed is the top card, the second sits under it"
    );
    assert!(
        graveyard_of(&engine, P1).is_empty(),
        "nothing reached the graveyard"
    );
    assert_eq!(
        hand_of(&engine, P1).len(),
        1,
        "the third card was never asked about"
    );

    // The next draw is the card listed first: p1's own draw step.
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, P1);
    assert!(
        hand_of(&engine, P1).contains(&first),
        "p1's draw step drew the first listed card"
    );
    assert_eq!(
        library_of(&engine, P1).last(),
        Some(&second),
        "the second listed is the new top"
    );
}

/// The other order of the same two cards, so a library which ignored the
/// listed order and kept the discard order would fail one of the two.
#[test]
fn the_listed_order_decides_which_card_is_on_top_either_way_round() {
    for (seed, swap) in [(5202, false), (5202, true)] {
        let mut engine = twist_at_p1(seed, &[], &[leng()]);
        let cards = leng_asks(&engine, P1);
        let listed = if swap {
            vec![cards[1], cards[0]]
        } else {
            vec![cards[0], cards[1]]
        };
        answer(&mut engine, P1, &[], &listed);
        let lib = library_of(&engine, P1);
        assert_eq!(lib.last(), Some(&listed[0]), "swap = {swap}");
        assert_eq!(lib[lib.len() - 2], listed[1], "swap = {swap}");
    }
}

/// A split answer: one of the two cards to the graveyard, the other on top.
#[test]
fn a_split_answer_sends_one_card_each_way() {
    let mut engine = twist_at_p1(5203, &[], &[leng()]);
    let cards = leng_asks(&engine, P1);
    let size = library_of(&engine, P1).len();
    answer(&mut engine, P1, &[cards[0]], &[cards[1]]);

    assert_eq!(graveyard_of(&engine, P1), vec![cards[0]], "only the first");
    let lib = library_of(&engine, P1);
    assert_eq!(lib.len(), size + 1, "one card went on top");
    assert_eq!(lib.last(), Some(&cards[1]));
    assert!(!hand_of(&engine, P1).contains(&cards[0]));
    assert!(!hand_of(&engine, P1).contains(&cards[1]));
}

/// The default answer: every card in the graveyard pile (the pile listed
/// first, so the answer that prefers nothing) is the discard as printed, and
/// the library is not touched. Both cards are still discards.
#[test]
fn the_default_answer_is_the_printed_discard() {
    let mut engine = twist_at_p1(5204, &[], &[leng()]);
    let cards = leng_asks(&engine, P1);
    let size = library_of(&engine, P1).len();
    answer(&mut engine, P1, &cards, &[]);

    let mut in_graveyard = graveyard_of(&engine, P1);
    let mut asked = cards.clone();
    in_graveyard.sort();
    asked.sort();
    assert_eq!(in_graveyard, asked, "both in the graveyard");
    assert_eq!(
        library_of(&engine, P1).len(),
        size,
        "the library is untouched"
    );
    assert_eq!(hand_of(&engine, P1).len(), 1);
}

/// Wheel of Fortune cast by the Leng's controller: "Each player discards
/// their hand, then draws seven cards." p0 is asked about the three cards of
/// its hand, puts two on top (and one in the graveyard), and the draw that
/// follows takes them back. The opponent, with no Leng, is never asked.
#[test]
fn wheel_of_fortune_asks_its_caster_and_the_draws_take_the_cards_back() {
    let mut engine = wheel_with_leng_for(5205, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let cards = leng_asks(&engine, P0);
    assert_eq!(cards.len(), 3, "p0's whole hand, and not the Wheel");
    let elves = in_hand(&engine, P1, llanowar_elves()).expect("p1's card is untouched yet");
    let before = library_of(&engine, P0).len();

    answer(&mut engine, P0, &[cards[2]], &[cards[0], cards[1]]);
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "p1 is not asked: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        graveyard_of(&engine, P0).len(),
        2,
        "the Wheel and the one card"
    );
    assert!(graveyard_of(&engine, P0).contains(&cards[2]));
    assert!(
        in_graveyard(&engine, P1, llanowar_elves()).is_some(),
        "p1 discarded as printed"
    );
    assert!(!hand_of(&engine, P1).contains(&elves));
    let hand = hand_of(&engine, P0);
    assert_eq!(hand.len(), 7, "seven drawn");
    assert!(
        hand.contains(&cards[0]) && hand.contains(&cards[1]),
        "the cards put on top are among the cards drawn: {hand:?}"
    );
    assert!(
        !hand.contains(&cards[2]),
        "the one sent to the graveyard is not"
    );
    assert_eq!(
        library_of(&engine, P0).len(),
        before + 2 - 7,
        "two on top, seven drawn"
    );
}

/// The Wheel cast by the opponent of a Leng's controller: the question goes
/// to the player who discards the cards and has the Leng, p1, and not to the
/// caster. p0's own hand goes to the graveyard unasked.
#[test]
fn the_question_goes_to_the_leng_controller_who_is_discarding_not_the_caster() {
    let mut engine = wheel_with_leng_for(5206, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let cards = leng_asks(&engine, P1);
    assert_eq!(cards.len(), 1, "p1's one card, and none of p0's three");
    assert!(
        in_hand(&engine, P1, llanowar_elves()).is_some_and(|c| c == cards[0]),
        "the card asked about is p1's"
    );
    assert!(
        in_hand(&engine, P0, bears()).is_some(),
        "p0's discard waits behind p1's question (nothing has moved)"
    );

    answer(&mut engine, P1, &[], &[cards[0]]);
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "p0 is never asked: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    for card in [bears(), serra_angel(), hill_giant()] {
        assert!(
            in_graveyard(&engine, P0, card).is_some(),
            "p0's cards were discarded as printed"
        );
    }
    assert!(in_graveyard(&engine, P1, llanowar_elves()).is_none());
    assert!(
        hand_of(&engine, P1).contains(&cards[0]),
        "p1 drew its own card back"
    );
}

/// Tireless Tribe: "Discard a card: This creature gets +0/+4 until end of
/// turn." A cost is not an effect's discard, so a Library of Leng out asks
/// nothing: the cost's own choice is the only question, and the card lands
/// in the graveyard.
#[test]
fn a_cost_discard_asks_nothing() {
    let mut engine = Duel::new(5207, forest())
        .battlefield(0, &[leng(), plains()])
        .hand(0, &[tireless_tribe(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, tireless_tribe());
    pass_until(&mut engine, stack_is_empty);
    let tribe = on_battlefield(&engine, P0, tireless_tribe()).expect("the Tribe resolved");
    let fodder = in_hand(&engine, P0, island()).expect("something to discard");
    let size = library_of(&engine, P0).len();

    activate(&mut engine, P0, tireless_tribe(), 0);
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostDiscard,
                ..
            }
        ),
        "the cost's own choice: {:?}",
        engine.pending()
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "no Library of Leng question for a cost: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, island()).is_some());
    assert_eq!(library_of(&engine, P0).len(), size, "nothing went on top");
    assert_eq!(pt(&engine, tribe), (1, 5), "and the ability still resolved");
}

/// With no Library of Leng on the battlefield, or only the opponent's,
/// Mind Twist asks nothing and both cards are in the graveyard. The card in
/// p1's own hand is not on the battlefield and does nothing either.
#[test]
fn without_a_library_on_the_discarders_battlefield_nobody_is_asked() {
    for (name, p0_board, p1_board) in [
        ("no Leng", &[][..], &[][..]),
        ("the caster's Leng", &[leng()][..], &[][..]),
    ] {
        let mut engine = twist_at_p1(5208, p0_board, p1_board);
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "{name}: {:?}",
            engine.pending()
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(graveyard_of(&engine, P1).len(), 2, "{name}");
        assert_eq!(hand_of(&engine, P1).len(), 1, "{name}");
    }

    // A Library of Leng in hand is a card, not a permanent.
    let mut engine = Duel::new(5209, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[mind_twist()])
        .hand(1, &[leng(), bears(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, mind_twist());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "Leng in hand: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(graveyard_of(&engine, P1).len(), 2);
}
