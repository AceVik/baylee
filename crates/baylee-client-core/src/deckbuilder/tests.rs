//! The builder's tests: every decision it makes, without a renderer.

#[allow(clippy::wildcard_imports)]
use super::*;

fn card(index: u32, name: &str, cost: &str, cmc: u32, kinds: &[&str]) -> PoolCard {
    let colors: String = cost.chars().filter(|c| "WUBRG".contains(*c)).collect();
    PoolCard {
        index,
        name: name.to_string(),
        english_name: name.to_string(),
        mana_cost: cost.to_string(),
        cmc,
        colors: colors.clone(),
        identity: colors,
        type_line: kinds.join(" "),
        kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
        stats: None,
        oracle_text: String::new(),
        coverage: Coverage::Implemented,
        note: None,
        commander: false,
        basic_land: kinds.contains(&"Land") && name == "Forest",
        ..PoolCard::default()
    }
}

fn pool() -> Vec<PoolCard> {
    vec![
        card(1, "Forest", "", 0, &["Land"]),
        card(2, "Grizzly Bears", "{1}{G}", 2, &["Creature"]),
        card(3, "Lightning Bolt", "{R}", 1, &["Instant"]),
        card(4, "Wrath of God", "{2}{W}{W}", 4, &["Sorcery"]),
        card(5, "Sol Ring", "{1}", 1, &["Artifact"]),
    ]
}

fn builder() -> DeckBuilder {
    let mut b = DeckBuilder::new();
    b.set_pool(pool(), true);
    b.set_name("Test");
    b
}

/// Everything the builder offers has to come from the pool, and the pool
/// is what the engine can play. A card that is only a stub is hidden by
/// default: offering it would be offering a card that does nothing.
#[test]
fn a_stub_is_hidden_until_it_is_asked_for() {
    let mut b = DeckBuilder::new();
    let mut cards = pool();
    cards[4].coverage = Coverage::Unimplemented;
    b.set_pool(cards, true);
    assert_eq!(b.results().len(), 4);
    b.toggle_playable_only();
    assert_eq!(b.results().len(), 5, "asking for them shows them");
}

/// Search reaches the name a player knows, whichever of the two it is,
/// and the rules text when the gateway had it.
#[test]
fn search_looks_where_a_player_would() {
    let mut b = builder();
    b.set_text("bears");
    assert_eq!(b.results().len(), 1);
    assert_eq!(
        b.card(b.results()[0]).unwrap().english_name,
        "Grizzly Bears"
    );
    b.set_text("instant");
    assert_eq!(b.results().len(), 1, "the type line is searched too");
    b.set_text("nothing at all");
    assert!(b.results().is_empty());
}

/// A color filter asks "what could a deck of these colors play", so a card
/// needing a colour that was not picked is out, and colorless is in.
#[test]
fn colors_filter_to_what_a_deck_could_play() {
    let mut b = builder();
    b.toggle_color('G');
    let names: Vec<&str> = b
        .results()
        .iter()
        .map(|s| b.card(*s).unwrap().english_name.as_str())
        .collect();
    assert!(names.contains(&"Grizzly Bears"));
    assert!(names.contains(&"Forest"), "a land is colorless");
    assert!(names.contains(&"Sol Ring"), "so is an artifact");
    assert!(!names.contains(&"Lightning Bolt"));
    b.toggle_color('R');
    assert!(
        b.results()
            .iter()
            .any(|s| b.card(*s).unwrap().english_name == "Lightning Bolt"),
        "adding red admits it"
    );
}

/// Clicking a bar of the curve filters to that mana value, and clicking it
/// again clears the filter — the same control both ways.
#[test]
fn a_curve_bar_filters_and_unfilters() {
    let mut b = builder();
    b.set_cmc(Some(1));
    let names: Vec<&str> = b
        .results()
        .iter()
        .map(|s| b.card(*s).unwrap().english_name.as_str())
        .collect();
    assert_eq!(names, vec!["Lightning Bolt", "Sol Ring"]);
    assert!(!names.contains(&"Forest"), "lands are not on the curve");
    b.set_cmc(Some(1));
    assert_eq!(b.results().len(), 5, "the same bar clears it");
}

/// Four of anything, any number of basics — the rule the gateway enforces,
/// enforced here too so the player is told before they try to save.
#[test]
fn copies_are_capped_except_for_basics() {
    let mut b = builder();
    let bears = b.slot_of("Grizzly Bears").unwrap();
    for _ in 0..4 {
        assert!(b.add(bears, Zone::Main));
    }
    assert!(!b.add(bears, Zone::Main), "the fifth is refused");
    assert_eq!(b.count_of(bears, Zone::Main), 4);

    let forest = b.slot_of("Forest").unwrap();
    for _ in 0..20 {
        assert!(b.add(forest, Zone::Main));
    }
    assert_eq!(b.count_of(forest, Zone::Main), 20);
}

/// The deck and the sideboard are separate lists holding the same cards.
#[test]
fn the_two_zones_count_separately() {
    let mut b = builder();
    let bolt = b.slot_of("Lightning Bolt").unwrap();
    b.add(bolt, Zone::Main);
    b.add(bolt, Zone::Side);
    b.add(bolt, Zone::Side);
    assert_eq!(b.count_of(bolt, Zone::Main), 1);
    assert_eq!(b.count_of(bolt, Zone::Side), 2);
    let counts = b.counts();
    assert_eq!(counts.main, 1);
    assert_eq!(counts.side, 2);
}

/// Removing the last copy takes the row away rather than leaving a zero.
#[test]
fn the_last_copy_takes_its_row_with_it() {
    let mut b = builder();
    let bolt = b.slot_of("Lightning Bolt").unwrap();
    b.add(bolt, Zone::Main);
    assert_eq!(b.entries(Zone::Main).len(), 1);
    b.remove(bolt, Zone::Main);
    assert!(b.entries(Zone::Main).is_empty());
    assert!(
        !b.remove(bolt, Zone::Main),
        "and removing again does nothing"
    );
}

/// A deck list prints creatures first and lands last, whatever order the
/// cards were added in.
#[test]
fn a_deck_list_is_in_deck_list_order() {
    let mut b = builder();
    for name in ["Forest", "Wrath of God", "Grizzly Bears", "Sol Ring"] {
        let slot = b.slot_of(name).unwrap();
        b.add(slot, Zone::Main);
    }
    let order: Vec<&str> = b
        .entries(Zone::Main)
        .iter()
        .map(|e| b.card(e.slot).unwrap().english_name.as_str())
        .collect();
    assert_eq!(
        order,
        vec!["Grizzly Bears", "Wrath of God", "Sol Ring", "Forest"]
    );
}

/// The curve counts spells by mana value and leaves lands out — they are
/// what pays for the curve, not part of it.
#[test]
fn the_curve_counts_spells_and_not_lands() {
    let mut b = builder();
    let forest = b.slot_of("Forest").unwrap();
    for _ in 0..10 {
        b.add(forest, Zone::Main);
    }
    let bolt = b.slot_of("Lightning Bolt").unwrap();
    b.add(bolt, Zone::Main);
    b.add(bolt, Zone::Main);
    let bears = b.slot_of("Grizzly Bears").unwrap();
    b.add(bears, Zone::Main);
    let curve = b.curve();
    assert_eq!(curve[0], 0, "no lands on the curve");
    assert_eq!(curve[1], 2);
    assert_eq!(curve[2], 1);
    assert_eq!(b.counts().lands, 10);
}

/// Pips are what a mana base is built against, so hybrid and generic
/// symbols must not be counted as coloured requirements.
#[test]
fn pips_count_coloured_symbols_only() {
    let mut b = builder();
    let wrath = b.slot_of("Wrath of God").unwrap();
    b.add(wrath, Zone::Main);
    // {2}{W}{W} — two white pips, and the {2} is not one of them.
    assert_eq!(b.pips(), [2, 0, 0, 0, 0]);
}

/// The save button may only be live when the save will succeed, so the
/// blocking problems have to be the gateway's refusals exactly.
#[test]
fn a_deck_that_cannot_save_says_why() {
    let mut b = DeckBuilder::new();
    b.set_pool(pool(), true);
    assert!(!b.saveable(), "no name, no cards");
    let problems = b.problems(Lang::En);
    let blocking: Vec<&str> = problems
        .iter()
        .filter(|p| p.blocking)
        .map(|p| p.message.as_str())
        .collect();
    assert_eq!(blocking.len(), 2, "{blocking:?}");
    b.set_name("Mono Green");
    let forest = b.slot_of("Forest").unwrap();
    b.add(forest, Zone::Main);
    assert!(b.saveable());
    assert!(b.save().is_some());
}

/// Advice is not a refusal. A half-built deck is a normal thing to keep,
/// and the builder has to let it be kept.
#[test]
fn advice_never_blocks_a_save() {
    let mut b = builder();
    let forest = b.slot_of("Forest").unwrap();
    b.add(forest, Zone::Main);
    assert!(b.saveable(), "one card is savable");
    let problems = b.problems(Lang::En);
    let advice: Vec<&Problem> = problems.iter().filter(|p| !p.blocking).collect();
    assert!(
        advice.iter().any(|p| p.message.contains("at least 60")),
        "the short deck is mentioned: {advice:?}"
    );
}

/// A deck is stored under English names whatever language it was built in,
/// or the gateway would not resolve it against the registry.
#[test]
fn rows_are_written_in_english() {
    let mut b = DeckBuilder::new();
    let mut cards = pool();
    cards[1].name = "Grislibären".to_string();
    b.set_pool(cards, true);
    b.set_name("Deutsch");
    let bears = b.slot_of("Grizzly Bears").expect("found by English name");
    b.add(bears, Zone::Main);
    b.add(bears, Zone::Main);
    assert_eq!(b.rows(Zone::Main), vec!["2 Grizzly Bears"]);
}

/// Loading a stored deck reproduces it exactly, and a save afterwards
/// updates that deck rather than creating a second one.
#[test]
fn a_loaded_deck_round_trips() {
    let mut b = builder();
    b.load(
        "deck-1",
        "Burn",
        &["4 Lightning Bolt".to_string(), "20 Forest".to_string()],
        &["2 Wrath of God".to_string()],
        None,
    );
    assert_eq!(b.name(), "Burn");
    assert_eq!(b.editing(), Some("deck-1"));
    assert!(!b.dirty(), "loading is not an edit");
    assert_eq!(b.counts().main, 24);
    assert_eq!(b.counts().side, 2);
    assert_eq!(
        b.rows(Zone::Main),
        vec!["4 Lightning Bolt".to_string(), "20 Forest".to_string()]
    );
    let Some(LobbyRequest::SaveDeck { deck_id, .. }) = b.save() else {
        panic!("a save request");
    };
    assert_eq!(deck_id.as_deref(), Some("deck-1"), "it updates the deck");
}

/// A card the pool no longer has is named, not dropped. Silently losing a
/// card on the next save is the one outcome a deck builder must not have.
#[test]
fn a_card_the_pool_lost_is_reported_not_dropped() {
    let mut b = builder();
    b.load("d", "Old", &["1 Black Lotus".to_string()], &[], None);
    assert_eq!(b.missing(), ["Black Lotus"]);
    assert!(!b.saveable(), "and it refuses to save over the loss");
    assert!(
        b.problems(Lang::En)
            .iter()
            .any(|p| p.blocking && p.message.contains("Black Lotus"))
    );
}

/// A deck cannot grow past what the gateway will store, and the builder
/// stops it at the click rather than at the save.
#[test]
fn each_lists_cap_holds_on_its_own() {
    let mut b = builder();
    let forest = b.slot_of("Forest").unwrap();
    for _ in 0..MAX_DECK_CARDS {
        b.add(forest, Zone::Main);
    }
    assert_eq!(b.counts().main, MAX_DECK_CARDS);
    assert!(!b.add(forest, Zone::Main), "and no further");
    assert!(b.saveable(), "at the cap it still saves");
    // The gateway caps each list separately, so a full main deck is not
    // what stops a sideboard being built.
    assert!(b.add(forest, Zone::Side), "the sideboard has its own room");
}

/// Starting a new deck forgets the one being edited, or the next save
/// would quietly overwrite it.
#[test]
fn a_new_deck_is_not_the_old_one() {
    let mut b = builder();
    b.load("deck-1", "Burn", &["1 Forest".to_string()], &[], None);
    b.start_new();
    assert_eq!(b.editing(), None);
    assert!(b.name().is_empty());
    assert!(b.entries(Zone::Main).is_empty());
}

#[test]
fn a_card_can_be_read_without_being_added() {
    let mut b = builder();
    b.inspect(0);
    assert_eq!(b.inspecting(), Some(0));
    assert!(
        b.entries(Zone::Main).is_empty(),
        "reading a card is not taking it"
    );
    b.stop_inspecting();
    assert_eq!(b.inspecting(), None);
    // A slot the pool does not have would draw a panel with nothing in it.
    b.inspect(9_999);
    assert_eq!(b.inspecting(), None);
    // And a new deck starts with nothing open.
    b.inspect(0);
    b.start_new();
    assert_eq!(b.inspecting(), None);
}
/// A player searches for the card they own, which is the card in their
/// hand, which is not necessarily printed in English.
#[test]
fn a_card_is_found_under_any_name_it_was_printed_with() {
    let mut builder = DeckBuilder::default();
    builder.set_pool(
        vec![
            PoolCard {
                index: 1,
                name: "Lightning Bolt".to_string(),
                english_name: "Lightning Bolt".to_string(),
                alt_names: vec!["Blitzschlag".to_string(), "稲妻".to_string()],
                kinds: vec!["Instant".to_string()],
                ..PoolCard::default()
            },
            PoolCard {
                index: 2,
                name: "Counterspell".to_string(),
                english_name: "Counterspell".to_string(),
                alt_names: vec!["Gegenzauber".to_string()],
                kinds: vec!["Instant".to_string()],
                ..PoolCard::default()
            },
        ],
        false,
    );

    for needle in ["blitz", "稲妻", "Lightning"] {
        builder.set_text(needle);
        let hits = builder.results();
        assert_eq!(hits.len(), 1, "{needle} matched {} cards", hits.len());
        assert_eq!(hits[0], 0, "{needle} found the wrong card");
    }

    // One row per card, never one per name. "l" is in this card's English
    // name *and* in its German one; a builder that searched printings
    // would list it twice.
    builder.set_text("l");
    let hits = builder.results();
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert_eq!(hits.iter().filter(|slot| **slot == 0).count(), 1);
}
/// A printing, as the picker's tests need one.
fn printing(set: &str, number: &str, lang: &str, finishes: &[&str]) -> Printing {
    Printing {
        scryfall_id: format!("{set}-{number}-{lang}"),
        oracle_id: "bolt".to_string(),
        lang: lang.to_string(),
        set: set.to_string(),
        set_name: format!("Set {set}"),
        collector_number: number.to_string(),
        finishes: finishes.iter().map(|f| (*f).to_string()).collect(),
        name: "Lightning Bolt".to_string(),
        ..Printing::default()
    }
}

/// A builder holding one card, with the picker open on it.
fn picking() -> DeckBuilder {
    let mut builder = DeckBuilder::new();
    builder.set_pool(
        vec![PoolCard {
            index: 7,
            name: "Lightning Bolt".to_string(),
            english_name: "Lightning Bolt".to_string(),
            oracle_id: "bolt".to_string(),
            scryfall_id: "reference".to_string(),
            kinds: vec!["Instant".to_string()],
            type_line: "Instant".to_string(),
            coverage: Coverage::Implemented,
            ..PoolCard::default()
        }],
        false,
    );
    let asked = builder.open_picker(0, Zone::Main);
    assert_eq!(asked, Some(LobbyRequest::LoadPrintings { card: 7 }));
    builder
}

/// The dialog opens before the answer arrives, or a tap would feel
/// dropped. What it shows meanwhile is the printing the row already
/// names.
#[test]
fn the_picker_has_something_to_show_while_it_waits() {
    let builder = picking();
    let picker = builder.picker().expect("the picker is open");
    assert!(picker.loading());
    assert!(!picker.from_catalog());
    assert_eq!(picker.len(), 1);
    assert_eq!(
        picker.current().map(|p| p.scryfall_id.as_str()),
        Some("reference")
    );
    assert_eq!(picker.finish(), Finish::Normal);
}

/// An answer for a card the player has already moved on from must not
/// replace the printings of the one they are looking at.
#[test]
fn a_late_answer_for_another_card_is_dropped() {
    let mut builder = picking();
    builder.set_printings(999, vec![printing("m11", "149", "de", &["foil"])], true);
    let picker = builder.picker().expect("still open");
    assert!(
        picker.loading(),
        "the answer it is waiting for has not come"
    );
    assert_eq!(picker.current().map(|p| p.set.as_str()), Some(""));
}

/// The carousel is a ring: twelve printings have no beginning, and a
/// player flicking through them should not have to notice which one the
/// list happened to start at.
#[test]
fn the_carousel_wraps_at_both_ends() {
    let mut builder = picking();
    builder.set_printings(
        7,
        vec![
            printing("m11", "149", "en", &["nonfoil", "foil"]),
            printing("a25", "141", "en", &["nonfoil"]),
            printing("sta", "42", "ja", &["nonfoil", "etched"]),
        ],
        true,
    );
    let at = |b: &DeckBuilder| b.picker().and_then(Picker::current).map(|p| p.set.clone());

    assert_eq!(at(&builder).as_deref(), Some("m11"));
    builder.picker_step(-1);
    assert_eq!(at(&builder).as_deref(), Some("sta"), "back from the first");
    builder.picker_step(1);
    assert_eq!(at(&builder).as_deref(), Some("m11"), "and forward again");
    builder.picker_step(2);
    assert_eq!(at(&builder).as_deref(), Some("sta"));
}

/// A finish that was never printed must not survive a move to a printing
/// that does not have it, or the row would name cardboard that does not
/// exist.
#[test]
fn a_finish_does_not_outlive_the_printing_that_offered_it() {
    let mut builder = picking();
    builder.set_printings(
        7,
        vec![
            printing("m11", "149", "en", &["nonfoil", "foil"]),
            printing("a25", "141", "en", &["nonfoil"]),
        ],
        true,
    );
    builder.picker_set_finish(Finish::Foil);
    assert_eq!(builder.picker().map(Picker::finish), Some(Finish::Foil));

    builder.picker_step(1);
    assert_eq!(
        builder.picker().map(Picker::finish),
        Some(Finish::Normal),
        "this one was only ever sold plain"
    );
    // And it cannot be chosen while that printing is showing.
    builder.picker_set_finish(Finish::Foil);
    assert_eq!(builder.picker().map(Picker::finish), Some(Finish::Normal));
}

/// Filtering by language narrows the carousel and never leaves it
/// pointing past the end.
#[test]
fn a_language_filter_narrows_the_carousel() {
    let mut builder = picking();
    builder.set_printings(
        7,
        vec![
            printing("m11", "149", "en", &["nonfoil"]),
            printing("a25", "141", "en", &["nonfoil"]),
            printing("sta", "42", "ja", &["nonfoil"]),
        ],
        true,
    );
    assert_eq!(
        builder.picker().map(Picker::langs),
        Some(&["en".to_string(), "ja".to_string()][..])
    );

    builder.picker_step(2);
    builder.picker_set_lang(Some("ja"));
    let picker = builder.picker().expect("open");
    assert_eq!(picker.len(), 1);
    assert_eq!(picker.at(), 0, "a shorter list starts over");
    assert_eq!(picker.current().map(|p| p.set.as_str()), Some("sta"));

    builder.picker_set_lang(None);
    assert_eq!(builder.picker().map(Picker::len), Some(3));
}

/// A choice that changes nothing writes nothing: picking the default
/// printing leaves the row exactly as a deck built before any of this
/// existed would have written it.
#[test]
fn picking_the_default_printing_writes_a_plain_row() {
    let mut builder = picking();
    builder.set_printings(7, Vec::new(), false);
    assert!(builder.picker_confirm());
    assert_eq!(builder.rows(Zone::Main), vec!["1 Lightning Bolt"]);
}

/// And a real pick writes every part of itself, in a form
/// `baylee_core::deckrow::parse` reads back.
#[test]
fn a_picked_printing_reaches_the_deck_row() {
    let mut builder = picking();
    builder.set_printings(
        7,
        vec![printing("m11", "149", "de", &["nonfoil", "foil"])],
        true,
    );
    builder.picker_set_finish(Finish::Foil);
    assert!(builder.picker_confirm());
    assert!(builder.picker().is_none(), "confirming closes it");

    let rows = builder.rows(Zone::Main);
    assert_eq!(rows, vec!["1 Lightning Bolt (M11) 149 [de] *F*"]);
    // The row the builder writes is the row the parser reads.
    let parsed = baylee_core::deckrow::parse(&rows[0]).expect("round-trips");
    assert_eq!(parsed.name, "Lightning Bolt");
    assert_eq!(parsed.print.finish, Some(Finish::Foil));
    assert_eq!(parsed.print.lang.as_deref(), Some("de"));
}

/// Two printings of one card are two rows and still four copies: the
/// limit is on the card, which is the rule the gateway enforces.
#[test]
fn a_second_printing_is_a_second_row_and_not_a_fifth_copy() {
    let mut builder = picking();
    for _ in 0..2 {
        builder.add(0, Zone::Main);
    }
    builder.set_printings(
        7,
        vec![printing("m11", "149", "en", &["nonfoil", "foil"])],
        true,
    );
    builder.picker_set_finish(Finish::Foil);
    assert!(builder.picker_confirm());

    assert_eq!(builder.count_of(0, Zone::Main), 3);
    assert_eq!(builder.entries(Zone::Main).len(), 2, "two rows");

    // Up to four, and no further.
    assert!(builder.add(0, Zone::Main));
    assert!(!builder.add(0, Zone::Main), "the fifth copy is refused");
    assert_eq!(builder.count_of(0, Zone::Main), 4);
}

/// A deck reopened and saved again has to come back out the way it went
/// in — editing one line must not strip every other line's printing.
#[test]
fn a_loaded_deck_keeps_the_printings_it_was_saved_with() {
    let mut builder = picking();
    builder.close_picker();
    builder.load(
        "id",
        "Shiny",
        &[
            "2 Lightning Bolt (M11) 149 [de] *F*".to_string(),
            "1 Lightning Bolt".to_string(),
        ],
        &[],
        None,
    );
    assert!(builder.missing().is_empty(), "{:?}", builder.missing());
    assert_eq!(
        builder.entries(Zone::Main).len(),
        2,
        "two printings, two rows"
    );
    assert_eq!(builder.count_of(0, Zone::Main), 3);
    assert_eq!(
        builder.rows(Zone::Main),
        vec!["1 Lightning Bolt", "2 Lightning Bolt (M11) 149 [de] *F*"],
        "plain before foil, and stable between saves"
    );
}

/// Undoing an add takes back what was just added, not one of the copies
/// that were already there.
#[test]
fn removing_takes_the_most_recent_printing_first() {
    let mut builder = picking();
    builder.close_picker();
    builder.load(
        "id",
        "Shiny",
        &[
            "1 Lightning Bolt".to_string(),
            "1 Lightning Bolt (M11) 149 *F*".to_string(),
        ],
        &[],
        None,
    );
    assert!(builder.remove(0, Zone::Main));
    assert_eq!(builder.rows(Zone::Main), vec!["1 Lightning Bolt"]);
}

/// A pool where one card may lead a deck and the rest may not.
fn commander_pool() -> DeckBuilder {
    let mut cards = pool();
    let mut general = card(
        6,
        "Nissa, Who Shakes the World",
        "{3}{G}{G}",
        5,
        &["Legendary", "Planeswalker"],
    );
    general.commander = true;
    cards.push(general);
    let mut b = DeckBuilder::new();
    b.set_pool(cards, true);
    b.set_name("Test");
    b
}

/// The pool says which cards the rules can seat as a commander, and the
/// gateway rejects the rest on save. Offering the choice on a card that
/// would be refused is worse than not offering it at all.
#[test]
fn a_card_that_cannot_lead_a_deck_is_refused_as_its_commander() {
    let mut b = commander_pool();
    let bears = b.slot_of("Grizzly Bears").unwrap();
    assert!(!b.set_commander(bears));
    assert_eq!(b.commander(), None);
}

/// A commander is one of the cards in the deck, so naming one that is not
/// in it yet puts it there — a leader outside the list is a deck nobody
/// meant to build.
#[test]
fn naming_a_commander_seats_it_in_the_deck() {
    let mut b = commander_pool();
    let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
    assert!(b.set_commander(nissa));
    assert_eq!(b.commander(), Some(nissa));
    assert_eq!(b.count_of(nissa, Zone::Main), 1);
    assert_eq!(b.commander_name(), Some("Nissa, Who Shakes the World"));
}

/// Clearing the mark leaves the card where it is: a player demoting their
/// commander is not asking to lose the card.
#[test]
fn clearing_the_commander_keeps_the_card() {
    let mut b = commander_pool();
    let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
    b.set_commander(nissa);
    b.clear_commander();
    assert_eq!(b.commander(), None);
    assert_eq!(b.count_of(nissa, Zone::Main), 1);
}

/// The commander rides the save request and comes back on load — and it
/// is a *name* on the wire, so it races the pool exactly as the rows do.
#[test]
fn a_commander_survives_a_save_and_a_reload() {
    let mut b = commander_pool();
    let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
    b.set_commander(nissa);
    let Some(crate::lobby::LobbyRequest::SaveDeck { commander, .. }) = b.save() else {
        panic!("a named deck with cards saves");
    };
    assert_eq!(commander.as_deref(), Some("Nissa, Who Shakes the World"));

    // Loaded into a builder whose pool has not arrived yet.
    let mut fresh = DeckBuilder::new();
    fresh.load(
        "d",
        "Superfriends",
        &["1 Nissa, Who Shakes the World".to_string()],
        &[],
        Some("Nissa, Who Shakes the World"),
    );
    assert_eq!(
        fresh.commander(),
        None,
        "no pool, nothing to resolve against"
    );
    let mut cards = pool();
    let mut general = card(
        6,
        "Nissa, Who Shakes the World",
        "{3}{G}{G}",
        5,
        &["Legendary", "Planeswalker"],
    );
    general.commander = true;
    cards.push(general);
    fresh.set_pool(cards, true);
    assert_eq!(fresh.commander_name(), Some("Nissa, Who Shakes the World"));
}

/// Moving a card between the lists must not quietly reprint it: the
/// sideboard copy is the same piece of cardboard the deck held.
#[test]
fn a_card_moved_to_the_sideboard_keeps_its_printing() {
    let mut builder = picking();
    builder.picker_set_finish(Finish::Foil);
    assert!(builder.picker_confirm());
    let before = builder.rows(Zone::Main);
    assert_eq!(before.len(), 1);
    assert!(builder.move_entry(0, Zone::Main, Zone::Side));
    assert!(builder.rows(Zone::Main).is_empty());
    assert_eq!(builder.rows(Zone::Side), before);
}

/// A move to the zone the card is already in is not a move, and must not
/// silently duplicate or drop it.
#[test]
fn moving_a_card_to_the_zone_it_is_in_does_nothing() {
    let mut b = builder();
    let forest = b.slot_of("Forest").unwrap();
    b.add(forest, Zone::Main);
    assert!(!b.move_entry(0, Zone::Main, Zone::Main));
    assert_eq!(b.count_of(forest, Zone::Main), 1);
}
