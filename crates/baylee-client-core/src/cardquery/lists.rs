//! The ban lists a search may name, for `list:`.
//!
//! What a client holds is `GET /banlists`: the official lists and the
//! caller's own, each with the registry indices of its cards. A shelf is that
//! read for a search, so a row is asked by the same [`Facts::card`] the
//! gateway's table is asked by, and a row with no card (a token, a face-down
//! card) answers nothing here either, which is what keeps this key from being
//! a way of learning what a hidden card is.
//!
//! An official list goes by its format: its key, and what the format is
//! called in the client's language and in English. An own list goes by the
//! name its owner gave it. Case and accents do not count, on either side.
//!
//! [`Facts::card`]: super::Facts::card

use super::folded;
use crate::deckcheck::format_name;
use crate::i18n::Lang;
use baylee_cardtext::ListRow;

/// One list as a search reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShelfList {
    word: String,
    official: bool,
    shown: String,
    names: Vec<String>,
    cards: Vec<u32>,
}

impl ShelfList {
    /// What a row holds when this list is chosen: an own list's name, an
    /// official list's format key.
    #[must_use]
    pub fn word(&self) -> &str {
        &self.word
    }

    /// What it is called on the screen.
    #[must_use]
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// Whether the list names the card with this registry index.
    #[must_use]
    pub fn holds(&self, card: u32) -> bool {
        self.cards.binary_search(&card).is_ok()
    }

    /// Whether this list goes by a name holding these folded letters.
    fn called(&self, typed: &str) -> bool {
        self.names.iter().any(|name| name.contains(typed))
    }
}

/// Every list a search here may name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListShelf {
    lists: Vec<ShelfList>,
}

impl ListShelf {
    /// The shelf these rows make. A player's own lists come first, so a list
    /// they named after a format answers to its name.
    #[must_use]
    pub fn of(rows: &[ListRow], lang: Lang) -> Self {
        let mut lists: Vec<ShelfList> = rows.iter().map(|row| shelved(row, lang)).collect();
        // Stable: within each half the gateway's order, by name, stands.
        lists.sort_by_key(|list| list.official);
        Self { lists }
    }

    /// Every list, a player's own first.
    #[must_use]
    pub fn lists(&self) -> &[ShelfList] {
        &self.lists
    }

    /// The list a written word names.
    #[must_use]
    pub fn named(&self, word: &str) -> Option<&ShelfList> {
        let want = folded(word);
        self.lists.iter().find(|list| list.names.contains(&want))
    }

    /// The lists whose name holds what has been typed so far.
    pub fn called<'a>(&'a self, typed: &str) -> impl Iterator<Item = &'a ShelfList> {
        let typed = folded(typed);
        self.lists.iter().filter(move |list| list.called(&typed))
    }
}

/// One row, read for a search.
fn shelved(row: &ListRow, lang: Lang) -> ShelfList {
    let mut cards = row.indices.clone();
    cards.sort_unstable();
    if !row.builtin {
        return ShelfList {
            word: row.name.clone(),
            official: false,
            shown: row.name.clone(),
            names: vec![folded(&row.name)],
            cards,
        };
    }
    let shown = format_name(&row.format, lang).to_string();
    let mut names = vec![
        folded(&row.format),
        folded(&shown),
        folded(format_name(&row.format, Lang::En)),
    ];
    names.sort_unstable();
    names.dedup();
    ShelfList {
        word: row.format.clone(),
        official: true,
        shown,
        names,
        cards,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, format: &str, builtin: bool, indices: &[u32]) -> ListRow {
        ListRow {
            name: name.to_string(),
            format: format.to_string(),
            builtin,
            indices: indices.to_vec(),
            ..ListRow::default()
        }
    }

    #[test]
    fn a_list_answers_to_its_name_whatever_the_case() {
        let shelf = ListShelf::of(&[row("My Commander", "commander", false, &[7])], Lang::En);
        assert!(shelf.named("my commander").is_some());
        assert!(shelf.named("MY COMMANDER").is_some());
        assert!(
            shelf.named("commander").is_none(),
            "its format is not its name"
        );
        assert!(
            shelf.named("my").is_none(),
            "a part of a name names nothing"
        );
    }

    #[test]
    fn an_official_list_goes_by_its_format_in_both_languages() {
        let shelf = ListShelf::of(&[row("duel", "duel", true, &[])], Lang::De);
        for spelled in ["duel", "Duel Commander"] {
            assert_eq!(
                shelf.named(spelled).map(ShelfList::word),
                Some("duel"),
                "{spelled}"
            );
        }
    }

    #[test]
    fn a_players_own_list_comes_first_and_keeps_its_name() {
        let shelf = ListShelf::of(
            &[
                row("modern", "modern", true, &[1]),
                row("Modern", "legacy", false, &[2]),
            ],
            Lang::En,
        );
        let found = shelf.named("modern").expect("a list");
        assert!(found.holds(2), "the player's, which they named so");
    }

    #[test]
    fn a_list_knows_its_cards_in_any_order() {
        let shelf = ListShelf::of(&[row("Mine", "freeform", false, &[9, 3, 5])], Lang::En);
        let list = shelf.named("mine").expect("a list");
        assert!(list.holds(3) && list.holds(5) && list.holds(9));
        assert!(!list.holds(4));
    }

    #[test]
    fn typing_narrows_the_lists_by_what_they_go_by() {
        let shelf = ListShelf::of(
            &[
                row("modern", "modern", true, &[]),
                row("Kitchen table", "freeform", false, &[]),
            ],
            Lang::En,
        );
        let words = |typed: &str| shelf.called(typed).map(ShelfList::word).collect::<Vec<_>>();
        assert_eq!(words("kit"), ["Kitchen table"]);
        assert_eq!(words("MOD"), ["modern"]);
        assert_eq!(words("").len(), 2);
    }
}
