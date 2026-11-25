//! What an edit to the deck does beside the edit itself: the orders of the
//! pool, the field a key goes to, the count of changes and a late auto-save.

use super::*;

fn card(index: u32, name: &str, colors: &str, cmc: u32, kind: &str) -> PoolCard {
    PoolCard {
        index,
        name: name.into(),
        english_name: name.into(),
        cmc,
        colors: colors.into(),
        identity: colors.into(),
        type_line: kind.into(),
        kinds: kind.split(' ').map(str::to_owned).collect(),
        coverage: Coverage::Implemented,
        ..PoolCard::default()
    }
}

fn stats(mut card: PoolCard, stats: &str) -> PoolCard {
    card.stats = Some(stats.into());
    card
}

/// Names no printed card has, so nothing here depends on the compiled pool.
fn pool() -> Vec<PoolCard> {
    vec![
        stats(card(900_001, "Zed", "G", 2, "Creature"), "2/2"),
        stats(card(900_002, "Yak", "WU", 6, "Creature"), "*/5"),
        card(900_003, "Xis", "", 0, "Artifact"),
        card(900_004, "Wyl", "R", 1, "Instant"),
        stats(card(900_005, "Vox", "G", 4, "Creature"), "2/1"),
        card(900_006, "Ulm", "W", 4, "Planeswalker"),
    ]
}

fn builder() -> DeckBuilder {
    let mut b = DeckBuilder::new();
    b.set_pool(pool(), true);
    b
}

fn names(b: &DeckBuilder) -> Vec<&str> {
    b.results()
        .iter()
        .map(|slot| b.card(*slot).unwrap().name.as_str())
        .collect()
}

// ------------------------------------------------------------ orders

#[test]
fn every_order_of_the_pool_falls_back_to_the_name() {
    let mut b = builder();
    for (sort, expected) in [
        (Sort::Name, ["Ulm", "Vox", "Wyl", "Xis", "Yak", "Zed"]),
        (Sort::Cost, ["Xis", "Wyl", "Zed", "Ulm", "Vox", "Yak"]),
        (Sort::Type, ["Zed", "Vox", "Yak", "Ulm", "Wyl", "Xis"]),
        // White, red, green, two colours, none.
        (Sort::Color, ["Ulm", "Wyl", "Zed", "Vox", "Yak", "Xis"]),
        // Common, uncommon, rare, mythic, each by cost.
        (Sort::Rarity, ["Xis", "Wyl", "Zed", "Vox", "Yak", "Ulm"]),
        // No power at all, then `*` as nothing, then by toughness.
        (Sort::Power, ["Xis", "Wyl", "Ulm", "Yak", "Vox", "Zed"]),
    ] {
        b.set_sort(sort);
        assert_eq!(names(&b), expected, "{sort:?}");
    }
}

#[test]
fn the_direction_turns_the_order_around_and_then_gives_it_up() {
    let mut b = builder();
    b.set_sort(Sort::Cost);
    b.cycle_sort_dir();
    assert_eq!(b.sort_dir(), SortDir::Desc);
    // A tie still reads by name, not backwards.
    assert_eq!(names(&b), ["Yak", "Ulm", "Vox", "Zed", "Wyl", "Xis"]);
    b.cycle_sort_dir();
    assert_eq!(b.sort_dir(), SortDir::Unset);
    assert_eq!(names(&b), ["Ulm", "Vox", "Wyl", "Xis", "Yak", "Zed"]);
    b.cycle_sort_dir();
    assert_eq!(b.sort_dir(), SortDir::Asc);
    assert_eq!(b.sort(), Sort::Cost, "the criterion is kept throughout");
}

#[test]
fn choosing_an_order_while_there_is_none_orders_upwards_again() {
    let mut b = builder();
    b.cycle_sort_dir();
    b.set_sort(Sort::Power);
    assert_eq!(b.sort_dir(), SortDir::Desc, "a direction chosen is kept");
    b.cycle_sort_dir();
    b.set_sort(Sort::Cost);
    assert_eq!(b.sort_dir(), SortDir::Asc);
    assert_eq!(names(&b)[0], "Xis");
}

#[test]
fn a_rarity_narrows_the_pool_until_it_is_chosen_again() {
    let mut b = builder();
    b.set_rarity(Some(Rarity::Mythic));
    assert_eq!(names(&b), ["Ulm"]);
    b.set_rarity(Some(Rarity::Uncommon));
    assert_eq!(names(&b), ["Vox"]);
    b.set_rarity(Some(Rarity::Uncommon));
    assert_eq!(b.rarity(), None);
    assert_eq!(names(&b).len(), 6);
}

#[test]
fn a_card_outside_the_pool_counts_as_common() {
    let b = builder();
    assert_eq!(b.card_rarity(5), Rarity::Mythic);
    assert_eq!(b.card_rarity(99), Rarity::Common);
}

#[test]
fn a_card_the_gateway_knows_no_rarity_of_is_guessed_by_what_it_is() {
    let b = builder();
    let guess = |card: &PoolCard| b.card_rarity_of(card);
    for (cmc, rarity) in [
        (3, Rarity::Common),
        (4, Rarity::Uncommon),
        (5, Rarity::Uncommon),
        (6, Rarity::Rare),
    ] {
        assert_eq!(guess(&card(1_000_000, "Q", "", cmc, "Sorcery")), rarity);
    }
    let legend = card(1_000_000, "Q", "", 1, "Legendary Creature");
    assert_eq!(guess(&legend), Rarity::Rare);
    let mut leader = card(1_000_000, "Q", "", 1, "Creature");
    leader.commander = true;
    assert_eq!(guess(&leader), Rarity::Rare);
    let land = card(1_000_000, "Q", "", 9, "Basic Land");
    assert_eq!(guess(&land), Rarity::Common);
}

// ------------------------------------------------------ the deck's rows

#[test]
fn one_copy_leaves_the_row_it_is_taken_from_and_the_last_takes_the_row() {
    let mut b = builder();
    assert!(b.add(0, Zone::Main));
    assert!(b.add(0, Zone::Main));
    assert!(b.add(3, Zone::Main));
    let at = b.row_of(0, Zone::Main).unwrap();
    assert!(b.remove_at(at, Zone::Main));
    assert_eq!(b.count_of(0, Zone::Main), 1);
    assert!(b.remove_at(at, Zone::Main));
    assert_eq!(b.row_of(0, Zone::Main), None);
    assert_eq!(b.count_of(3, Zone::Main), 1, "the other row is untouched");
    let changes = b.changes();
    assert!(!b.remove_at(1, Zone::Main), "there is no second row");
    assert!(!b.remove_at(0, Zone::Side), "nor anything beside");
    assert_eq!(b.changes(), changes, "nothing happened, nothing to save");
}

#[test]
fn the_last_cards_added_are_kept_newest_first_and_once_each() {
    let mut b = builder();
    for slot in [0, 1, 2, 0, 3] {
        assert!(b.add(slot, Zone::Main));
    }
    assert_eq!(b.last_added(), [3, 0, 2]);
}

#[test]
fn a_card_is_found_by_its_rules_identity() {
    let b = builder();
    assert_eq!(b.card_by_index(900_004).unwrap().name, "Wyl");
    assert!(b.card_by_index(900_099).is_none());
}

#[test]
fn an_add_goes_to_the_zone_chosen() {
    let mut b = builder();
    assert_eq!(b.zone(), Zone::Main);
    b.set_zone(Zone::Side);
    assert_eq!(b.zone(), Zone::Side);
}

#[test]
fn a_variant_replaces_the_cards_and_the_format_and_counts_as_one_edit() {
    let mut b = builder();
    b.set_name("Kept");
    assert!(b.add(0, Zone::Main));
    assert!(b.add(1, Zone::Side));
    let before = b.changes();
    b.apply_variant(
        "modern",
        &["3 Wyl".into(), "not a row".into(), "1 Nowhere".into()],
        &["2 Xis".into()],
        &[],
    );
    assert_eq!(b.rows(Zone::Main), ["3 Wyl"]);
    assert_eq!(b.rows(Zone::Side), ["2 Xis"]);
    assert_eq!(b.name(), "Kept");
    assert_eq!(b.format(), "modern");
    assert!(b.missing().contains(&"Nowhere".to_owned()));
    assert_eq!(b.changes(), before + 1);
    assert!(b.dirty());
}

// ------------------------------------------------------------- partners

fn leaders() -> DeckBuilder {
    let mut cards = pool();
    cards[0].commander = true;
    cards[1].commander = true;
    cards[0].partners = vec![cards[1].index, cards[0].index];
    cards[1].partners = vec![cards[0].index];
    let mut b = DeckBuilder::new();
    b.set_pool(cards, true);
    b
}

#[test]
fn only_a_card_the_commander_partners_with_is_offered_beside_it() {
    let mut b = leaders();
    assert!(!b.can_partner(1), "there is nobody to partner with");
    assert!(b.set_commander(0));
    assert!(b.is_commander(0));
    assert!(b.can_partner(1));
    assert!(!b.can_partner(0), "not with itself, whatever the pool says");
    assert!(!b.can_partner(4));
    assert!(!b.can_partner(99));
    assert!(b.add_partner(1));
    assert!(b.is_commander(1));
    assert!(!b.can_partner(1), "two is all there can be");
    assert!(!b.add_partner(4));
    assert_eq!(b.commanders(), [0, 1]);
}

/// Every pair of two colors.
fn pairs() -> impl Iterator<Item = baylee_core::color::ColorPair> {
    use baylee_core::color::{Color, ColorPair};
    Color::ALL.into_iter().enumerate().flat_map(|(at, first)| {
        Color::ALL[at + 1..]
            .iter()
            .map(move |second| ColorPair::new(first, *second))
    })
}

#[test]
fn the_circled_colors_lie_inside_the_commanders_identity() {
    let mut b = leaders();
    for pair in pairs() {
        assert!(b.circles_allowed(pair), "no commander, no bounds");
    }
    assert!(b.set_commander(1)); // white and blue
    let allowed: Vec<_> = pairs().filter(|pair| b.circles_allowed(*pair)).collect();
    assert_eq!(allowed.len(), 1);
    let colors = [allowed[0].first().symbol(), allowed[0].second().symbol()];
    assert!(colors.contains(&'W') && colors.contains(&'U'));
    // A partner's colors count too: green joins.
    assert!(b.add_partner(0));
    let allowed = pairs().filter(|pair| b.circles_allowed(*pair));
    assert_eq!(allowed.count(), 3);
}

// ----------------------------------------------------- keys and the name

#[test]
fn a_key_reaches_the_field_in_focus_and_no_other() {
    let mut b = builder();
    b.focus_on(BuildField::Search);
    for ch in "wyl".chars() {
        b.type_focused(ch);
    }
    assert_eq!(b.focused_text(), "wyl");
    assert_eq!(names(&b), ["Wyl"], "the pool follows the box");
    assert_eq!(b.changes(), 0, "searching is no edit");
    b.backspace_focused();
    assert_eq!(b.text(), "wy");

    b.focus_on(BuildField::Name);
    for ch in "Deck".chars() {
        b.type_focused(ch);
    }
    b.backspace_focused();
    assert_eq!(b.focused_text(), "Dec");
    assert_eq!(b.name(), "Dec");
    assert_eq!(b.text(), "wy");
    assert_eq!(b.changes(), 1, "a run of keys is one edit");
}

#[test]
fn a_name_is_given_only_where_there_is_none() {
    let mut b = builder();
    b.set_default_name("New deck");
    assert_eq!(b.name(), "New deck");
    assert_eq!(b.changes(), 0, "nobody typed it");
    // It is selected, so the first key replaces it.
    b.focus_on(BuildField::Name);
    b.type_focused('A');
    assert_eq!(b.name(), "A");
    b.set_default_name("New deck");
    assert_eq!(b.name(), "A");
}

#[test]
fn edits_thrown_away_leave_nothing_to_save() {
    let mut b = builder();
    b.type_name('A');
    assert!(b.add(0, Zone::Main));
    assert_eq!(b.changes(), 2);
    b.discard_edits();
    assert!(!b.dirty());
    assert_eq!(b.changes(), 0);
    b.type_name('B');
    assert_eq!(b.changes(), 1, "the next key starts a run of its own");
}

// ---------------------------------------------------- saving by itself

#[test]
fn an_auto_save_of_what_is_held_leaves_nothing_to_save() {
    let mut b = builder();
    assert!(b.add(0, Zone::Main));
    let sent = b.draft();
    b.mark_auto_saved(&sent, Some("deck-1"), false);
    assert!(!b.dirty());
    assert_eq!(b.changes(), 0);
    assert_eq!(b.editing(), Some("deck-1"));
    assert!(b.saves_in_place());
}

#[test]
fn an_edit_made_while_an_auto_save_was_under_way_is_still_unsaved() {
    let mut b = builder();
    assert!(b.add(0, Zone::Main));
    let sent = b.draft();
    assert!(b.add(1, Zone::Main));
    b.mark_auto_saved(&sent, Some("deck-1"), false);
    assert!(b.dirty());
    assert_eq!(b.changes(), 2);
    assert_eq!(b.editing(), Some("deck-1"), "but it has its place");
}

#[test]
fn a_loaded_deck_is_saved_in_place_only_after_its_old_version_is_kept() {
    let mut b = builder();
    b.load("deck-1", "Deck", &["1 Zed".into()], &[], &[]);
    assert!(!b.saves_in_place());
    let sent = b.draft();
    b.mark_auto_saved(&sent, None, false);
    assert!(!b.saves_in_place());
    b.mark_auto_saved(&sent, None, true);
    assert!(b.saves_in_place());
    assert_eq!(b.editing(), Some("deck-1"));
}

// ------------------------------------------------------ the set's filter

fn printing(set: &str, name: &str) -> Printing {
    Printing {
        set: set.into(),
        set_name: name.into(),
        collector_number: "1".into(),
        lang: "en".into(),
        ..Printing::default()
    }
}

#[test]
fn the_cursor_of_the_set_list_wraps_and_closing_forgets_what_was_typed() {
    let mut b = builder();
    b.open_picker(0, Zone::Main);
    let printings = vec![
        printing("AAA", "Alpha"),
        printing("BBB", "Beta"),
        printing("CCC", "Gamma"),
    ];
    b.set_printings(900_001, printings, true);
    let cursor = |b: &DeckBuilder| b.picker().unwrap().set_cursor;
    assert_eq!(b.picker().unwrap().matching_sets().len(), 3);
    b.picker_move_set(false);
    assert_eq!(cursor(&b), 2);
    b.picker_move_set(true);
    assert_eq!(cursor(&b), 0);
    b.picker_move_set(true);
    assert_eq!(cursor(&b), 1);

    b.focus_on(BuildField::PickerSet);
    b.type_focused('g');
    assert_eq!(b.focused_text(), "g");
    assert_eq!(cursor(&b), 0, "a narrowed list starts at its top");
    assert_eq!(b.picker().unwrap().matching_sets().len(), 1);
    b.picker_close_sets();
    assert!(!b.picker().unwrap().set_open);
    assert_eq!(b.focused_text(), "");
    assert_eq!(b.picker().unwrap().matching_sets().len(), 3);
}
