//! A question about another player's library says whose it is. Architects of Will aimed at an opponent puts the cards back on top of *their* library, and a shelf that named no library read as the player's own: a language-model seat was told "the top of your library" there (hc-smoke, game-0002, question 278). The question carries no owner; the view shows whose cards they are (`Interaction::prompt_in`).

use super::*;
use baylee_client_core::i18n::Lang;
use baylee_client_core::test_support::{ViewBuilder, statics, token};
use baylee_engine::choice::{ArrangePile, ArrangePlace, ArrangePrompt};

/// An ordering of two cards back on top of `owner`'s library, at a table
/// where the other seat is called Mira, fed through the real edges.
fn ordering(owner: u8) -> Duel {
    let looked = vec![
        token(101, owner, "Mountain", 0, 0),
        token(82, owner, "Island", 0, 0),
    ];
    let mut view = ViewBuilder::new(2).with_looking_at(looked).build();
    view.awaiting = Some(view.seat);
    let mut roster = statics(0);
    roster.seats.push(baylee_view::SeatIdentity {
        player: PlayerId::new(1),
        display_name: "Mira".to_string(),
        is_ai: false,
        away: false,
        team: None,
        guest: false,
    });
    let mut duel = Duel {
        statics: Some(roster),
        ..Duel::default()
    };
    duel.receive_view(view);
    duel.receive_choice(Pending::Arrange {
        player: PlayerId::new(0),
        cards: vec![ObjectId::new(101, 0), ObjectId::new(82, 0)],
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 2)],
        prompt: ArrangePrompt::Order,
    });
    duel
}

#[test]
fn the_shelf_names_whose_library_an_ordering_puts_the_cards_back_into() {
    let texts = crate::cardtext::CardTexts::default();
    let theirs = ordering(1);
    assert_eq!(
        theirs.headline(Lang::En, &texts).unwrap(),
        "Put these back on top of Mira's library: the first is the new top card"
    );
    assert_eq!(
        theirs.headline(Lang::De, &texts).unwrap(),
        "Lege diese oben auf die Bibliothek von Mira zurück: die erste ist die neue oberste Karte"
    );

    // The player's own library is said as it always was.
    let mine = ordering(0);
    assert_eq!(
        mine.headline(Lang::En, &texts).unwrap(),
        "Put these back on top: the first is the new top card"
    );
}
