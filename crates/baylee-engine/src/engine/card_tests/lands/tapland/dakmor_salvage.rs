//! `cards/lands/tapland/dakmor_salvage.rs`, played.
//!
//! Dakmor Salvage: "This land enters tapped. {T}: Add {B}. Dredge 2". The
//! first test is the land (enters tapped, makes {B}); the others are the
//! dredge, written from the printed text, the cards' rulings and CR 702.52
//! (dredge), 121.2 (a draw at a time) and 305.2 (the land drop), and each
//! stands a case that is offered beside one that is not.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::draw_kit as kit;

/// Three shelf lands with one printed sentence between them — "this land
/// enters tapped" — and all of them a mana ability to go with it. They are
/// played together because the assertion that matters is the pair: a land
/// that entered untapped would still make its mana, and a colour is only a
/// claim beside a total.
///
/// Cryptic Spires also enters tapped and produces exactly one of the two
/// colors recorded for this physical copy while constructing its deck.
#[test]
fn three_shelf_taplands_enter_tapped_and_make_the_colour_they_print() {
    let p0 = PlayerId::new(0);

    for (card, colour) in [
        (dakmor_salvage(), ManaColor::Black),
        (spinerock_knoll(), ManaColor::Red),
    ] {
        let mut engine = Duel::new(SEED, forest()).hand(0, &[card]).start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "\"This land enters tapped\" is unconditional on this card"
        );

        cross_into_the_next_own_main(&mut engine, p0);
        activate(&mut engine, p0, card, 0);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            1,
            "the one colour this land's ability prints"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            1,
            "and nothing beside it"
        );
    }

    let mut spires = Duel::new(SEED, forest())
        .hand(0, &[cryptic_spires()])
        .start();
    keep_mulligans(&mut spires);
    assert!(walk_to_own_main(&mut spires, p0), "p0 reaches its own main");
    let hand = in_hand(&spires, p0, cryptic_spires()).unwrap();
    spires.state.object_mut(hand).unwrap().deck_choices = baylee_core::card_metadata::DeckChoices {
        circled_colors: Some(baylee_core::color::ColorPair::new(
            baylee_core::color::Color::White,
            baylee_core::color::Color::Black,
        )),
    };
    let land = play_land(&mut spires, p0, cryptic_spires());
    assert!(entered_tapped(&spires, land), "it enters tapped too");

    cross_into_the_next_own_main(&mut spires, p0);
    assert!(
        !is_tapped(&spires, land),
        "and untaps like any other permanent"
    );
    let Pending::Priority { legal, .. } = spires.pending().clone() else {
        panic!("expected priority, got {:?}", spires.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the circled-color mana action is offered"
    );
    activate(&mut spires, p0, cryptic_spires(), 0);
    let Pending::ChooseColor { options, .. } = spires.pending() else {
        panic!("circled colors are asked")
    };
    assert_eq!(options, &[ManaColor::White, ManaColor::Black]);
    spires
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        spires.state.players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert_eq!(spires.state.players[0].mana_pool.total(), 1);
    assert!(is_tapped(&spires, land));
}

/// Dredge 2 (CR 702.52a): "if you would draw a card, you may instead mill two
/// cards and return this card from your graveyard to your hand." Asked of
/// the drawing player with the Salvage on the menu and naming none allowed.
/// Taken, it mills the top two, returns the land and draws nothing -- and the
/// land can then be played as the turn's land drop, entering tapped as
/// printed. Declined, the draw is made and the Salvage stays in the
/// graveyard.
#[test]
fn dredge_2_mills_two_and_returns_the_land_instead_of_a_draw_and_it_can_be_played() {
    for dredge in [true, false] {
        let mut engine = kit::duel(4401, &kit::tome_board(), &[kit::salvage()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, kit::salvage());
        let salvage = in_graveyard(&engine, kit::P0, kit::salvage()).expect("the land is buried");
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
            (kit::P0, vec![salvage], 0, 1, ChoicePrompt::Dredge),
            "a draw with the Salvage in the graveyard is put to its player"
        );
        if dredge {
            kit::reply(&mut engine, &[salvage]);
            kit::run_stack(&mut engine);
            assert!(!kit::is_asking(&engine));
            assert_eq!(
                kit::hand(&engine, kit::P0),
                vec![salvage],
                "the land is back"
            );
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..2]),
                "two cards were milled, in place of the draw"
            );
            assert_eq!(kit::library(&engine, kit::P0).len(), library.len() - 2);
            assert_eq!(
                kit::drawn_this_turn(&engine, kit::P0),
                0,
                "nothing was drawn"
            );

            let land = play_land(&mut engine, kit::P0, kit::salvage());
            assert_eq!(
                engine.state().object(land).map(|o| o.zone),
                Some(Zone::Battlefield),
                "the dredged Salvage is played as the land drop"
            );
            assert!(
                entered_tapped(&engine, land),
                "and enters tapped as printed"
            );
        } else {
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
            assert_eq!(
                kit::hand(&engine, kit::P0),
                vec![library[0]],
                "declined: the top card was drawn"
            );
            assert_eq!(
                kit::graveyard(&engine, kit::P0),
                vec![salvage],
                "the Salvage stays, and nothing was milled"
            );
        }
    }
}

/// CR 702.52a-b: dredge 2 needs at least two cards in the library. With three
/// cards left, the first two cards of Ancestral Recall's three are each put
/// to the Salvage's owner (declined: three, then two left); the third draw
/// finds one card in the library and is made without a question.
#[test]
fn dredge_2_is_offered_while_the_library_holds_two_cards_and_not_with_one() {
    let mut engine = kit::duel(
        4402,
        &[island()],
        &[kit::recall(), kit::salvage()],
        &[],
        &[],
    );
    hand_to_graveyard(&mut engine, kit::P0, kit::salvage());
    let salvage = in_graveyard(&engine, kit::P0, kit::salvage()).expect("the land is buried");
    let all = kit::library(&engine, kit::P0).len();
    seed_graveyard(&mut engine, kit::P0, all - 3);
    let library = kit::top(&engine, kit::P0, 3);
    assert_eq!(library.len(), 3);

    kit::recall_at(&mut engine, kit::P0);
    let mut asked = 0;
    let mut sizes = Vec::new();
    while kit::is_asking(&engine) {
        let q = kit::question(&engine);
        assert_eq!((q.options, q.prompt), (vec![salvage], ChoicePrompt::Dredge));
        sizes.push(kit::library(&engine, kit::P0).len());
        asked += 1;
        kit::reply(&mut engine, &[]);
        kit::run_stack(&mut engine);
    }
    assert_eq!(
        sizes,
        vec![3, 2],
        "asked with three cards and with two in the library, not with one"
    );
    assert_eq!(asked, 2);
    assert_eq!(
        kit::sorted(&kit::hand(&engine, kit::P0)),
        kit::sorted(&library),
        "all three draws were made"
    );
}

/// CR 702.52a: dredge "functions only while the card with dredge is in a
/// player's graveyard". The Salvage on the battlefield or in hand is not
/// asked about; in the graveyard, the same draw is.
#[test]
fn a_salvage_on_the_battlefield_or_in_hand_dredges_nothing() {
    for place in ["battlefield", "hand", "graveyard"] {
        let mut board = kit::tome_board();
        let mut hand = Vec::new();
        match place {
            "battlefield" => board.push(kit::salvage()),
            _ => hand.push(kit::salvage()),
        }
        let mut engine = kit::duel(4403, &board, &hand, &[], &[]);
        if place == "graveyard" {
            hand_to_graveyard(&mut engine, kit::P0, kit::salvage());
        }
        let top = *kit::library(&engine, kit::P0).last().expect("a library");
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);
        assert_eq!(
            kit::is_asking(&engine),
            place == "graveyard",
            "the Salvage in its {place}: dredge is offered only from the graveyard"
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
