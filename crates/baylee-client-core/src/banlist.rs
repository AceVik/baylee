//! A ban list being read or edited, as the deck builder holds it.
//!
//! The search is the deck builder's own: a box and its gear
//! ([`crate::cardsearch::SearchBox`]) over the same pool, read by
//! [`crate::cardsearch::pool_card_matches`], so it speaks the client's language
//! and English and no other, and a key added to the query language is here
//! too. An official list and one a table holds are read and never written:
//! [`ListEditor::read_only`] says so, and nothing here sends one.

use crate::cardquery::{Query, Surface};
use crate::cardsearch::{SearchBox, pool_card_matches};
use crate::deckbuilder::PoolCard;
use crate::lobby::LobbyRequest;
use crate::textbuf::TextBuffer;
use baylee_cardtext::{ListBody, ListEntry, ListStatus, MAX_LIST_ENTRIES, MAX_LIST_NAME_CHARS};

/// How many rows the search offers. A list is built a card at a time, and
/// the deck builder's own search is where a player browses.
const SHOWN: usize = 40;

/// The next word of the three, for one control that cycles them.
#[must_use]
pub const fn next_status(status: ListStatus) -> ListStatus {
    match status {
        ListStatus::Banned => ListStatus::Restricted,
        ListStatus::Restricted => ListStatus::BannedAsCommander,
        ListStatus::BannedAsCommander => ListStatus::Banned,
    }
}

/// A list open in the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListEditor {
    /// Its id; `None` until it is first saved.
    id: Option<String>,
    /// Its name.
    pub(crate) name: TextBuffer,
    format: String,
    entries: Vec<ListEntry>,
    /// The search over the pool.
    pub(crate) search: SearchBox,
    query: Query,
    results: Vec<usize>,
    /// Whether nobody may write it from here.
    read_only: bool,
    dirty: bool,
}

impl ListEditor {
    /// An empty list of the caller's own, not yet saved.
    #[must_use]
    pub fn blank(name: &str, format: &str) -> Self {
        Self {
            id: None,
            name: TextBuffer::new(name),
            format: format.to_string(),
            entries: Vec::new(),
            search: SearchBox::default(),
            query: Query::default(),
            results: Vec::new(),
            read_only: false,
            dirty: true,
        }
    }

    /// A list as the gateway sent it.
    #[must_use]
    pub fn of(body: &ListBody) -> Self {
        Self {
            id: Some(body.row.id.clone()),
            name: TextBuffer::new(&body.row.name),
            format: body.row.format.clone(),
            entries: body.entries.clone(),
            search: SearchBox::default(),
            query: Query::default(),
            results: Vec::new(),
            read_only: body.row.builtin || !body.row.mine,
            dirty: false,
        }
    }

    /// Its id, once it has one.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// What it is called.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.text()
    }

    /// The format it is for.
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Chooses the format it is for.
    pub fn set_format(&mut self, format: &str) {
        if !self.read_only && self.format != format {
            self.format = format.to_string();
            self.dirty = true;
        }
    }

    /// Whether this is an official list, or one a table holds.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    /// Whether it holds something the gateway has not been told.
    #[must_use]
    pub const fn dirty(&self) -> bool {
        self.dirty
    }

    /// Marks an edit of the name.
    pub(crate) const fn renamed(&mut self) {
        self.dirty = true;
    }

    /// Every card on it, by name.
    #[must_use]
    pub fn entries(&self) -> &[ListEntry] {
        &self.entries
    }

    /// The pool rows the search answers.
    #[must_use]
    pub fn results(&self) -> &[usize] {
        &self.results
    }

    /// What the list says about this card.
    #[must_use]
    pub fn status_of(&self, oracle_id: &str) -> Option<ListStatus> {
        self.entries
            .iter()
            .find(|entry| entry.oracle_id == oracle_id)
            .map(|entry| entry.status)
    }

    /// Writes what the list says about a pool card, or takes it off. Answers
    /// whether anything changed: a full list takes no new card.
    pub fn put(&mut self, card: &PoolCard, status: Option<ListStatus>) -> bool {
        if self.read_only || card.oracle_id.is_empty() {
            return false;
        }
        let at = self
            .entries
            .iter()
            .position(|entry| entry.oracle_id == card.oracle_id);
        match (at, status) {
            (Some(at), Some(status)) if self.entries[at].status != status => {
                self.entries[at].status = status;
            }
            (Some(at), None) => {
                self.entries.remove(at);
            }
            (None, Some(status)) if self.entries.len() < MAX_LIST_ENTRIES => {
                self.entries.push(ListEntry {
                    oracle_id: card.oracle_id.clone(),
                    status,
                    index: Some(card.index),
                    name: card.english_name.clone(),
                });
                self.entries.sort_by(|a, b| a.name.cmp(&b.name));
            }
            _ => return false,
        }
        self.dirty = true;
        true
    }

    /// Moves the word of the card at `at` on to the next of the three.
    pub fn cycle(&mut self, at: usize) {
        if self.read_only {
            return;
        }
        if let Some(entry) = self.entries.get_mut(at) {
            entry.status = next_status(entry.status);
            self.dirty = true;
        }
    }

    /// Takes the card at `at` off the list.
    pub fn remove(&mut self, at: usize) {
        if !self.read_only && at < self.entries.len() {
            self.entries.remove(at);
            self.dirty = true;
        }
    }

    /// Reads the box again and asks the pool.
    pub fn refilter(&mut self, pool: &[PoolCard], surface: Surface<'_>) {
        self.query = self.search.parsed();
        self.results = if self.search.field.text().trim().is_empty() {
            Vec::new()
        } else {
            pool.iter()
                .enumerate()
                .filter(|(_, card)| pool_card_matches(&self.query, card, surface))
                .map(|(slot, _)| slot)
                .take(SHOWN)
                .collect()
        };
    }

    /// Whether it can be saved: a name, and something to say.
    #[must_use]
    pub fn saveable(&self) -> bool {
        let name = self.name.text().trim();
        !self.read_only
            && self.dirty
            && !name.is_empty()
            && name.chars().count() <= MAX_LIST_NAME_CHARS
    }

    /// The request that saves it, or `None` when it cannot be.
    #[must_use]
    pub fn save(&self) -> Option<LobbyRequest> {
        self.saveable().then(|| LobbyRequest::SaveBanlist {
            id: self.id.clone(),
            name: self.name.text().trim().to_string(),
            format: self.format.clone(),
            entries: self
                .entries
                .iter()
                .map(|entry| (entry.oracle_id.clone(), entry.status))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cardtext::ListRow;

    fn card(index: u32, name: &str, oracle_id: &str) -> PoolCard {
        PoolCard {
            index,
            name: name.to_string(),
            english_name: name.to_string(),
            oracle_id: oracle_id.to_string(),
            ..PoolCard::default()
        }
    }

    fn body(builtin: bool, mine: bool) -> ListBody {
        ListBody {
            row: ListRow {
                id: "l1".to_string(),
                name: "Kitchen table".to_string(),
                format: "modern".to_string(),
                builtin,
                mine,
                ..ListRow::default()
            },
            entries: vec![ListEntry {
                oracle_id: "a".to_string(),
                status: ListStatus::Banned,
                index: Some(1),
                name: "Black Lotus".to_string(),
            }],
        }
    }

    #[test]
    fn a_card_goes_on_changes_its_word_and_comes_off() {
        let mut list = ListEditor::blank("Mine", "modern");
        let ring = card(2, "Sol Ring", "b");
        assert!(list.put(&ring, Some(ListStatus::Restricted)));
        assert_eq!(list.status_of("b"), Some(ListStatus::Restricted));
        assert!(!list.put(&ring, Some(ListStatus::Restricted)), "no change");
        assert!(list.put(&ring, Some(ListStatus::Banned)));
        assert_eq!(list.entries().len(), 1, "a card stands on a list once");
        assert!(list.put(&ring, None));
        assert!(list.entries().is_empty());
        assert!(!list.put(&ring, None), "and comes off once");
    }

    #[test]
    fn a_word_cycles_through_the_three() {
        let mut list = ListEditor::of(&body(false, true));
        list.cycle(0);
        assert_eq!(list.entries()[0].status, ListStatus::Restricted);
        list.cycle(0);
        assert_eq!(list.entries()[0].status, ListStatus::BannedAsCommander);
        list.cycle(0);
        assert_eq!(list.entries()[0].status, ListStatus::Banned);
    }

    #[test]
    fn an_official_list_and_someone_elses_are_read_only() {
        for mut list in [
            ListEditor::of(&body(true, false)),
            ListEditor::of(&body(false, false)),
        ] {
            assert!(list.read_only());
            let ring = card(2, "Sol Ring", "b");
            assert!(!list.put(&ring, Some(ListStatus::Banned)));
            list.cycle(0);
            list.remove(0);
            list.set_format("legacy");
            assert_eq!(list.entries().len(), 1);
            assert_eq!(list.entries()[0].status, ListStatus::Banned);
            assert_eq!(list.format(), "modern");
            assert!(list.save().is_none(), "and nothing here sends one");
        }
    }

    #[test]
    fn a_list_saves_once_it_has_a_name_and_a_change() {
        let mut list = ListEditor::of(&body(false, true));
        assert!(list.save().is_none(), "nothing to tell the gateway");
        list.cycle(0);
        let Some(LobbyRequest::SaveBanlist { id, entries, .. }) = list.save() else {
            panic!("it saves");
        };
        assert_eq!(id.as_deref(), Some("l1"));
        assert_eq!(entries, [("a".to_string(), ListStatus::Restricted)]);
        assert!(ListEditor::blank("", "modern").save().is_none());
    }

    /// The search is the deck builder's: a query over the pool, in the two
    /// languages a row carries.
    #[test]
    fn the_search_reads_the_pools_two_languages_and_no_third() {
        let mut bolt = card(1, "Blitzschlag", "a");
        bolt.english_name = "Lightning Bolt".to_string();
        bolt.foreign_names = vec!["Rayo".to_string()];
        let pool = vec![bolt, card(2, "Sol Ring", "b")];
        let mut list = ListEditor::blank("Mine", "modern");
        list.search.field = TextBuffer::new("blitz");
        list.refilter(&pool, Surface::POOL);
        assert_eq!(list.results(), [0]);
        list.search.field = TextBuffer::new("lightning");
        list.refilter(&pool, Surface::POOL);
        assert_eq!(list.results(), [0]);
        list.search.field = TextBuffer::new("rayo");
        list.refilter(&pool, Surface::POOL);
        assert!(list.results().is_empty());
        list.search.field = TextBuffer::new("");
        list.refilter(&pool, Surface::POOL);
        assert!(list.results().is_empty(), "an empty box offers nothing");
    }
}
