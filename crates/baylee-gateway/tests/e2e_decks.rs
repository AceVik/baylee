//! The deck-builder contract, against a real gateway process.
//!
//! The builder in `baylee-client-core` decides everything locally — what the
//! pool holds, whether a deck is legal, which route a save takes. All of that
//! is only worth anything if the gateway agrees, and none of it is exercised
//! by the duel tests, which start from a deck that is already stored.
//!
//! No agent is attached: nothing here starts a game.

mod common;

use common::{http, json_field, json_number, login, spawn_gateway};

/// The pool the deck builder searches.
#[test]
fn the_card_pool_is_public_and_says_what_the_engine_does_with_each_card() {
    let gateway = spawn_gateway("pool");

    // No token: the pool is reference data about this build, the same for
    // everybody, and the sign-in screen is allowed to have shown it already.
    let (status, body) = http(gateway.port, "GET", "/pool", None, "");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"cards\":["), "{body}");
    assert!(body.contains("\"pool_hash\""), "{body}");
    assert!(
        body.contains("\"english_name\""),
        "a deck row is written with the English name: {body}"
    );
    // Every row says how far the engine gets with it, or a builder offering a
    // stub would be lying about it.
    assert!(
        body.contains("\"coverage\":\"implemented\""),
        "at least one card is fully implemented: {body}"
    );
    // The pool is the registry, so a card the client can name is in it.
    assert!(body.contains("Baleful Strix"), "{body}");

    let (status, translated) = http(gateway.port, "GET", "/pool?lang=de", None, "");
    assert_eq!(status, 200, "{translated}");
    assert!(
        translated.contains("\"lang\":\"de\""),
        "the answer says which language it is in: {translated}"
    );
    // Without a catalog there is nothing to translate *to*, and the honest
    // answer is the English row rather than a blank one.
    assert!(translated.contains("Baleful Strix"), "{translated}");
}

/// A deck's whole life: saved, listed, read back, edited, deleted.
#[test]
fn a_deck_survives_a_round_trip_with_its_sideboard() {
    let gateway = spawn_gateway("decks");
    let token = login(gateway.port, "builder@example.test", "Builder");

    let body = r#"{"name":"Strixes","cards":["4 Baleful Strix","20 Forest"],
                   "sideboard":["2 Counterspell"],"commander":null}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    // The list carries counts, which is what the lobby's deck rows show.
    let (status, list) = http(gateway.port, "GET", "/decks", Some(&token), "");
    assert_eq!(status, 200, "{list}");
    assert!(list.contains("\"cards\":2"), "two lines, not 24: {list}");
    assert!(list.contains("\"sideboard\":1"), "{list}");

    // The deck itself comes back row for row — this is what the builder
    // re-opens, and a lost row would be silently dropped on the next save.
    let (status, one) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{one}");
    assert!(one.contains("4 Baleful Strix"), "{one}");
    assert!(one.contains("20 Forest"), "{one}");
    assert!(one.contains("2 Counterspell"), "{one}");

    // Editing overwrites in place: same id, new contents.
    let edited = r#"{"name":"Strixes, again","cards":["4 Baleful Strix"],
                     "sideboard":[],"commander":null}"#;
    let (status, answer) = http(
        gateway.port,
        "PUT",
        &format!("/decks/{id}"),
        Some(&token),
        edited,
    );
    assert_eq!(status, 204, "{answer}");
    let (_, one) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert!(one.contains("Strixes, again"), "{one}");
    assert!(
        !one.contains("Counterspell"),
        "the sideboard went with it: {one}"
    );

    let (status, answer) = http(
        gateway.port,
        "DELETE",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 204, "{answer}");
    let (status, _) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 404, "and it is really gone");
}

/// What the builder greys the save button for, the gateway refuses. The two
/// lists must agree or a live button would still fail.
#[test]
fn the_gateway_refuses_exactly_what_the_builder_calls_blocking() {
    let gateway = spawn_gateway("legality");
    let token = login(gateway.port, "picky@example.test", "Picky");

    for (why, body) in [
        (
            "an unknown card",
            r#"{"name":"D","cards":["1 Not A Real Card"],"sideboard":[],"commander":null}"#,
        ),
        (
            "an empty deck",
            r#"{"name":"D","cards":[],"sideboard":[],"commander":null}"#,
        ),
        (
            "a fifth copy",
            r#"{"name":"D","cards":["5 Baleful Strix"],"sideboard":[],"commander":null}"#,
        ),
        (
            "a nameless deck",
            r#"{"name":"","cards":["1 Forest"],"sideboard":[],"commander":null}"#,
        ),
        (
            "an unknown card in the sideboard",
            r#"{"name":"D","cards":["1 Forest"],"sideboard":["1 Not A Real Card"],"commander":null}"#,
        ),
        // The limit is on the card. Once a printing became part of a row's
        // identity, each of these was a pair of rows that passed a per-row
        // check and stored more copies than the route's own error message
        // allows — through the side that is supposed to be doing the
        // enforcing, which is what made it a way to cheat.
        (
            "eight copies split across two printings",
            r#"{"name":"D","cards":["4 Baleful Strix","4 Baleful Strix *F*"],"sideboard":[],"commander":null}"#,
        ),
        (
            "five copies split across two rows of the same printing",
            r#"{"name":"D","cards":["2 Baleful Strix","3 Baleful Strix"],"sideboard":[],"commander":null}"#,
        ),
        (
            "the same split, in the sideboard",
            r#"{"name":"D","cards":["1 Forest"],"sideboard":["4 Baleful Strix","4 Baleful Strix *F*"],"commander":null}"#,
        ),
    ] {
        let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
        assert_eq!(status, 400, "{why}: {answer}");
    }

    // And a basic land is the one card there may be any number of.
    let body = r#"{"name":"Mono-forest","cards":["40 Forest"],"sideboard":[],"commander":null}"#;
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{answer}");

    // The other half of counting per card: four copies *are* four copies
    // however many pieces of cardboard they are. Owning two foils and two
    // plain is an ordinary deck list, and the fix above must not refuse it.
    let body = r#"{"name":"Split","cards":["2 Baleful Strix","2 Baleful Strix *F*"],"sideboard":[],"commander":null}"#;
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{answer}");

    // Each list is counted on its own, which is what `DeckBuilder::add_print`
    // does — see the note in `docs/design.md` about whether it should.
    let body = r#"{"name":"Both","cards":["4 Baleful Strix"],"sideboard":["4 Baleful Strix"],"commander":null}"#;
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{answer}");
}

/// Decks belong to accounts, and the builder addresses them by id alone.
#[test]
fn another_account_cannot_read_edit_or_delete_a_deck() {
    let gateway = spawn_gateway("deck-owners");
    let mine = login(gateway.port, "mine@example.test", "Mine");
    let theirs = login(gateway.port, "theirs@example.test", "Theirs");

    let body = r#"{"name":"Mine","cards":["1 Forest"],"sideboard":[],"commander":null}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&mine), body);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    for (method, payload) in [("GET", ""), ("PUT", body), ("DELETE", "")] {
        let (status, answer) = http(
            gateway.port,
            method,
            &format!("/decks/{id}"),
            Some(&theirs),
            payload,
        );
        assert_eq!(status, 403, "{method}: {answer}");
    }
    // And an unsigned request never gets that far.
    let (status, _) = http(gateway.port, "GET", &format!("/decks/{id}"), None, "");
    assert_eq!(status, 401);
}

/// A row may name the printing, the language and the finish, and the gateway
/// stores it verbatim — which is what makes the stored form the exported form.
#[test]
fn a_row_keeps_the_printing_its_owner_chose() {
    let gateway = spawn_gateway("printings");
    let token = login(gateway.port, "collector@example.test", "Collector");

    let rows = r#"["4 Baleful Strix (2X2) 155 [de] *F*",
                   "1 Counterspell scryfall=11111111-2222-3333-4444-555555555555",
                   "20 Forest *E*"]"#;
    let body = format!(r#"{{"name":"Shiny","cards":{rows},"sideboard":[],"commander":null}}"#);
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&token), &body);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    let (status, one) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{one}");
    for kept in [
        "4 Baleful Strix (2X2) 155 [de] *F*",
        "1 Counterspell scryfall=11111111-2222-3333-4444-555555555555",
        "20 Forest *E*",
    ] {
        assert!(one.contains(kept), "{kept} was not kept: {one}");
    }
}

/// The rules still apply through the extra groups: a copy count is a copy
/// count whether or not the row names a foil.
#[test]
fn a_printing_does_not_buy_a_fifth_copy() {
    let gateway = spawn_gateway("printing-rules");
    let token = login(gateway.port, "sneaky@example.test", "Sneaky");

    for (why, row) in [
        ("five copies", "5 Baleful Strix (2X2) 155 *F*"),
        ("an unknown card", "1 Not A Real Card [de]"),
        ("a finish this build has never heard of", "1 Forest *Q*"),
        ("a language that is a word", "1 Forest [deutsch]"),
    ] {
        let body = format!(r#"{{"name":"D","cards":["{row}"],"sideboard":[],"commander":null}}"#);
        let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), &body);
        assert_eq!(status, 400, "{why}: {answer}");
    }
}

/// The printing picker's one request. Without a catalog the gateway still
/// answers with a printing to pick, because a picker that had to handle a
/// 503 would need a second code path for every gateway without a database.
#[test]
fn a_card_always_has_at_least_one_printing_to_choose() {
    let gateway = spawn_gateway("printing-list");

    // The pool says which card, and carries the identity the picker asks on.
    let (status, pool) = http(gateway.port, "GET", "/pool", None, "");
    assert_eq!(status, 200, "{pool}");
    assert!(
        pool.contains("\"oracle_id\""),
        "a row names the card behind its printing: {pool}"
    );

    // Asked about a card the pool actually holds. It used to ask about index
    // 1 — fine while the ledger numbered this pool and wrong the moment it
    // numbered the whole corpus, where index 1 is a card nobody implemented.
    let card = pool
        .split("\"index\":")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .expect("the pool names an index");
    let (status, one) = http(
        gateway.port,
        "GET",
        &format!("/printings?card={card}"),
        None,
        "",
    );
    assert_eq!(status, 200, "{one}");
    assert!(one.contains("\"printings\":["), "{one}");
    assert!(
        one.contains("\"nonfoil\""),
        "every card can be had plain: {one}"
    );
    // No catalog is configured here, so the answer says the list is the
    // registry's own reference rather than everything ever printed.
    assert!(one.contains("\"from_catalog\":false"), "{one}");

    // A card outside the registry is a 404, not an empty list: the picker
    // asked about something this build cannot play.
    let (status, answer) = http(gateway.port, "GET", "/printings?card=999999", None, "");
    assert_eq!(status, 404, "{answer}");
}

/// Every change to a deck's cards is a version somebody can go back to.
///
/// The whole of the owner's ask, end to end: save, change, look at what the
/// deck used to be, put it back — and then put the putting-back back, which
/// is the part that says a revert is a change like any other rather than a
/// rewind that erases one.
#[test]
fn a_deck_remembers_every_state_it_has_been_in() {
    let gateway = spawn_gateway("history");
    let token = login(gateway.port, "historian@example.test", "Historian");

    let first = r#"{"name":"Strixes","cards":["4 Baleful Strix","20 Forest"],
                    "commander":null,"description":"erster Wurf"}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&token), first);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    // A new deck is version 1 with nothing behind it.
    let (status, history) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}/history"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{history}");
    assert!(history.contains("\"version\":1"), "{history}");
    assert!(
        history.contains("\"past\":[]"),
        "nothing to undo yet: {history}"
    );

    // Change the cards.
    let second = r#"{"name":"Strixes","cards":["4 Baleful Strix","4 Counterspell","20 Island"],
                     "commander":null,"summary":"Wald raus, Insel rein"}"#;
    let (status, body) = http(
        gateway.port,
        "PUT",
        &format!("/decks/{id}"),
        Some(&token),
        second,
    );
    assert_eq!(status, 204, "{body}");

    let (status, history) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}/history"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{history}");
    assert!(
        history.contains("\"version\":2"),
        "the deck moved on: {history}"
    );
    assert!(
        history.contains("Wald raus, Insel rein"),
        "the change says what it was: {history}"
    );

    // The old state is readable in full, and it is the one that was replaced.
    let (status, old) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}/versions/1"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{old}");
    assert!(old.contains("20 Forest"), "{old}");
    assert!(
        !old.contains("20 Island"),
        "version 1 never had islands: {old}"
    );
    assert!(old.contains("\"current\":false"), "{old}");

    // And so is the current one, from the deck rather than from the history.
    let (status, now) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}/versions/2"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{now}");
    assert!(now.contains("\"current\":true"), "{now}");
    assert!(now.contains("20 Island"), "{now}");

    // What going back does with all of this is the next test.
}

/// Putting an earlier state back is a change like any other.
///
/// Not a rewind: the deck's present is archived exactly as any save archives
/// it, and the old lists become the head one version higher — so a revert
/// can itself be reverted and nothing in the history is ever removed.
#[test]
fn going_back_is_a_change_and_not_an_erasure() {
    let gateway = spawn_gateway("revert");
    let token = login(gateway.port, "reverter@example.test", "Reverter");

    let first = r#"{"name":"Strixes","cards":["4 Baleful Strix","20 Forest"],
                    "commander":null,"description":"erster Wurf"}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&token), first);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    let second = r#"{"name":"Strixes","cards":["4 Baleful Strix","4 Counterspell","20 Island"],
                     "commander":null,"summary":"Wald raus, Insel rein"}"#;
    let (status, body) = http(
        gateway.port,
        "PUT",
        &format!("/decks/{id}"),
        Some(&token),
        second,
    );
    assert_eq!(status, 204, "{body}");

    // Go back.
    let (status, reverted) = http(
        gateway.port,
        "POST",
        &format!("/decks/{id}/versions/1/revert"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{reverted}");
    assert!(
        reverted.contains("\"version\":3"),
        "a revert is the next version, not a rewind to the old number: {reverted}"
    );

    let (status, deck) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{deck}");
    assert!(deck.contains("20 Forest"), "the forests are back: {deck}");
    assert!(deck.contains("\"version\":3"), "{deck}");
    assert!(
        deck.contains("erster Wurf"),
        "a revert of the cards left the description alone: {deck}"
    );

    // The state the revert replaced is itself in the history now, so the
    // islands can be brought back. Nothing was erased.
    let (status, again) = http(
        gateway.port,
        "POST",
        &format!("/decks/{id}/versions/2/revert"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{again}");
    let (status, deck) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{deck}");
    assert!(deck.contains("20 Island"), "{deck}");
    assert!(deck.contains("\"version\":4"), "{deck}");

    // The counter-half: saving without touching the cards writes no version.
    let renamed = r#"{"name":"Strixes, second edition",
                      "cards":["4 Baleful Strix","4 Counterspell","20 Island"],
                      "commander":null}"#;
    let (status, body) = http(
        gateway.port,
        "PUT",
        &format!("/decks/{id}"),
        Some(&token),
        renamed,
    );
    assert_eq!(status, 204, "{body}");
    let (status, deck) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{deck}");
    assert!(
        deck.contains("\"version\":4"),
        "a rename is not an edit of the cards: {deck}"
    );
    assert!(deck.contains("second edition"), "{deck}");
}

/// The four house decks are there to be played and to be copied.
#[test]
fn the_house_decks_belong_to_nobody_and_anybody_may_take_a_copy() {
    let gateway = spawn_gateway("house");
    let token = login(gateway.port, "copier@example.test", "Copier");

    let (status, shared) = http(gateway.port, "GET", "/decks/shared", Some(&token), "");
    assert_eq!(status, 200, "{shared}");
    assert!(shared.contains("\"kind\":\"house\""), "{shared}");
    assert!(shared.contains("Breya, Etherium Shaper"), "{shared}");
    assert!(shared.contains("Kenrith, the Returned King"), "{shared}");
    assert!(shared.contains("Kess, Dissident Mage"), "{shared}");
    assert!(shared.contains("Tayam, Luminous Enigma"), "{shared}");
    // And the four that came from a table rather than from the engine's
    // needs: two of the owner's, two of his friends'.
    assert!(shared.contains("Allytifact"), "{shared}");
    assert!(shared.contains("Victory"), "{shared}");
    assert!(shared.contains("Schwarzrand"), "{shared}");
    assert!(shared.contains("Weltenbaum"), "{shared}");
    // `cards` in this answer is the number of *rows*, not of cards: a deck
    // stores `"N Card Name"` lines, so a playset is one row. Every house deck
    // is a hundred cards and they run from 97 rows to 100, which is why the
    // assertion below is on a deck that is singleton throughout rather than
    // on a number every one of them shares.
    assert!(shared.contains("\"cards\":100"), "{shared}");
    // The commander is among the rows and named again, bare, beside them —
    // the shape `decks::by_name` can actually resolve.
    assert!(
        shared.contains("\"commanders\":[\"Kenrith, the Returned King\"]"),
        "{shared}"
    );

    // A player's own list is still their own: the house decks are not in it.
    let (status, mine) = http(gateway.port, "GET", "/decks", Some(&token), "");
    assert_eq!(status, 200, "{mine}");
    assert_eq!(mine.trim(), "[]", "a new account owns no decks: {mine}");

    let id = json_field(&shared, "id").to_string();
    let (status, copied) = http(
        gateway.port,
        "POST",
        &format!("/decks/{id}/copy"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{copied}");
    let copy = json_field(&copied, "deck_id").to_string();

    let (status, deck) = http(
        gateway.port,
        "GET",
        &format!("/decks/{copy}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{deck}");
    assert!(
        deck.contains("\"kind\":\"account\""),
        "a copy is the copier's own deck: {deck}"
    );
    assert!(
        deck.contains(&format!("\"copied_from\":\"{id}\"")),
        "the copy remembers what it came from: {deck}"
    );
    assert!(
        deck.contains("\"copied_version\":1"),
        "and which state of it: {deck}"
    );
    assert!(
        deck.contains("\"version\":1"),
        "its own history starts over: {deck}"
    );

    // And now it is in the player's list, where the original never was.
    let (status, mine) = http(gateway.port, "GET", "/decks", Some(&token), "");
    assert_eq!(status, 200, "{mine}");
    // Against the deck that was copied, not against a number: the listing is
    // ordered by the database and the first house deck is whichever one that
    // is, so a literal here asserts the ordering and calls it a card count.
    let source_cards = json_number(&shared, "cards");
    assert_eq!(
        json_number(&mine, "cards"),
        source_cards,
        "the copy holds what the original held: {mine}"
    );
}

/// Two facts stood behind one refusal, and now the player is told which.
///
/// A name that is no card and a real card this build compiles nothing for
/// were both `unknown card`. That is the one answer that is wrong for the
/// second, and the second is the common case: this build holds 2716 of the
/// ledger's 33 694 cards, so 92 % of the real cards a player might type are
/// real and unavailable. The message sent them hunting a typo they had not
/// made.
///
/// The card is **derived rather than named**. The pool grows, so a
/// hard-coded `Black Lotus` becomes a card this build plays and the test
/// then passes for the wrong reason — it would be asserting about a card in
/// the pool. The first ledger row the pool cannot resolve by *either*
/// spelling is the one asked about, and the failure names it.
///
/// Both halves are asserted. Without the counter-half a gateway that said
/// the new sentence to everybody would pass, which is the same defect with a
/// different word in it.
#[test]
fn a_real_card_this_build_cannot_play_is_not_called_unknown() {
    let gateway = spawn_gateway("unplayable");
    let token = login(gateway.port, "importer@example.test", "Importer");

    // A deck row carries a printing, a language and a finish in brackets and
    // parentheses, so a name holding one would be refused for parsing rather
    // than for the pool — a different code path and not the one under test.
    // Two-faced names are left out too: which spellings a row may use is
    // #106's question, and a test that settled it here by accident would be
    // answering it in the wrong ticket.
    let candidates: Vec<&baylee_cards_index::Row> = baylee_cards_index::ROWS
        .iter()
        .filter(|row| {
            !row.name.contains(" // ")
                && row
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '\'' || c == ',')
                && baylee_cards::decks::by_name(row.name).is_none()
        })
        .collect();
    assert!(
        candidates.len() > 10_000,
        "only {} plainly-spelled cards outside the pool — the filter is \
         doing the choosing rather than the ledger",
        candidates.len()
    );
    let outside = candidates[0];

    let body = format!(
        r#"{{"name":"D","cards":["1 {}"],"sideboard":[],"commander":null}}"#,
        outside.name
    );
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), &body);
    assert_eq!(
        status, 400,
        "{} is a real card this build cannot play, and the deck is still \
         refused: {answer}",
        outside.name
    );
    assert!(
        answer.contains("this server cannot play it"),
        "{} is a real card and was called unknown: {answer}",
        outside.name
    );

    let (status, answer) = http(
        gateway.port,
        "POST",
        "/decks",
        Some(&token),
        r#"{"name":"D","cards":["1 Not A Real Card"],"sideboard":[],"commander":null}"#,
    );
    assert_eq!(status, 400, "{answer}");
    assert!(
        answer.contains("unknown card"),
        "a name that is no card at all is still unknown: {answer}"
    );
}

/// A deck row may be written either way the card is printed.
///
/// `data/card-pool.txt` writes 106 of its own names as `A // B`, which is
/// what Scryfall prints and what a deck site exports — and that spelling was
/// refused by the route that stores decks. Asserted through `POST /decks`
/// rather than against `by_name`, because the import path runs the name
/// through `deckrow::parse` first and a row is what a player actually sends.
///
/// The card is derived: the first pool card whose whole name differs from
/// what the pool calls it. Naming one would make this test a statement about
/// that card rather than about the spelling.
#[test]
fn a_deck_row_may_name_a_card_by_either_of_its_printed_spellings() {
    let gateway = spawn_gateway("two-spellings");
    let token = login(gateway.port, "importer2@example.test", "Importer");

    let (whole, index) = baylee_cards::generated_names::WHOLE_NAMES
        .iter()
        .copied()
        .find(|(_, index)| baylee_cards::by_index(*index).is_some())
        .expect("the pool holds a card with two faces");
    let front = baylee_cards::by_index(index).expect("compiled").name();
    assert_ne!(
        front, whole,
        "the two spellings differ, or this proves nothing"
    );

    for (why, spelling) in [("the pool's own", front), ("Scryfall's whole", whole)] {
        let body = format!(
            r#"{{"name":"D","cards":["1 {spelling}","20 Forest"],"sideboard":[],"commander":null}}"#
        );
        let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), &body);
        assert_eq!(
            status, 200,
            "{why} spelling {spelling:?} was refused: {answer}"
        );
    }

    // A name nobody prints is still refused, or the two above would pass on
    // a route that had simply stopped checking.
    let body = format!(
        r#"{{"name":"D","cards":["1 {front} // Not A Real Face","20 Forest"],"sideboard":[],"commander":null}}"#
    );
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), &body);
    assert_eq!(
        status, 400,
        "an invented second face was accepted: {answer}"
    );
}

/// `/pool` carries the whole spelling, so the builder's search box can find
/// what `POST /decks` will accept.
///
/// The two halves were apart: the route took `Agadeem's Awakening // Agadeem,
/// the Undercrypt` and the search box could not find it, which only shows up
/// when somebody uses both. Asserted over the wire rather than against
/// `pool::row`, because the field is `skip_serializing_if` empty and the
/// question is what a client receives.
#[test]
fn the_pool_carries_the_spelling_a_deck_row_may_use() {
    let gateway = spawn_gateway("pool-spellings");
    let (status, body) = http(gateway.port, "GET", "/pool", None, "");
    assert_eq!(status, 200, "{body}");

    let (whole, index) = baylee_cards::generated_names::WHOLE_NAMES
        .iter()
        .copied()
        .find(|(_, index)| baylee_cards::by_index(*index).is_some())
        .expect("the pool holds a card with two faces");
    assert!(
        body.contains(&format!("{whole:?}")),
        "{whole} is not in the pool answer"
    );

    // Without a catalog there are no translations, so this spelling is the
    // only thing in the field — which is the point of seeding it off the
    // registry rather than joining it on from an ingest.
    let front = baylee_cards::by_index(index).expect("compiled").name();
    assert!(
        body.contains(&format!("\"alt_names\":[{whole:?}]")),
        "{front} should carry exactly its whole spelling with no catalog"
    );
}
