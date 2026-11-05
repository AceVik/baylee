//! Naming a card (Cabal Therapy, Pithing Needle): what the model is told it
//! may name, and what a name this game cannot take is answered with.

use super::*;

fn therapy() -> Request {
    let (view, log) = board();
    request(
        view,
        Pending::ChooseCardName {
            player: ME,
            types: baylee_core::types::TypeSet::EMPTY,
            excluded_types: baylee_core::types::TypeSet::LAND,
        },
        log,
    )
}

fn naming(request: &Request, name: &str) -> Result<Resolved, String> {
    let wake = Narrator::new(&request.context).wake(request, &[], None);
    wake.menu.resolve(&Decision {
        ask: Some("q12".into()),
        name: Some(name.into()),
        ..Default::default()
    })
}

/// "Any card's English name" was not so: a card with no script is no name the
/// table can take, and a model told otherwise named Return to Dust.
#[test]
fn the_question_asks_for_a_card_this_game_has_and_says_what_is_left_out() {
    let request = therapy();
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert!(
        wake.text
            .contains("any card this game has, as in its deckbuilder"),
        "{}",
        wake.text
    );
    assert!(
        wake.text
            .contains("Naming one nobody can hold simply misses"),
        "{}",
        wake.text
    );
    assert!(
        !wake.text.contains("any card's English name"),
        "{}",
        wake.text
    );
    assert!(
        wake.text
            .contains("A card with these printed types is not eligible: Land."),
        "{}",
        wake.text
    );
}

#[test]
fn a_card_this_game_has_is_named() {
    let answer = naming(&therapy(), "Lightning Bolt").expect("a scripted card");
    assert!(matches!(
        answer.act,
        Act::Now(PlayerAction::ChooseCardName { card, .. })
            if card == identity("Lightning Bolt").index
    ));
}

/// Return to Dust is one of the cards with no script: it exists, so "not a
/// card" would be false, and the model is told what it can do instead.
#[test]
fn a_real_card_without_a_script_is_told_apart_from_a_name_that_is_no_card() {
    assert!(
        baylee_cards::decks::by_name("Return to Dust").is_none(),
        "pick another unscripted card for this test"
    );
    let refused = naming(&therapy(), "Return to Dust").expect_err("no script, no name");
    assert_eq!(
        refused,
        "«Return to Dust» exists, but it isn't in this game's pool: name any card this game \
         has, as in its deckbuilder (naming one nobody can hold simply misses)"
    );
    let refused = naming(&therapy(), "Nowhere Card").expect_err("no such card");
    assert_eq!(
        refused,
        "«Nowhere Card» is not the name of a card this game has"
    );
}
