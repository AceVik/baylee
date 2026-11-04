//! One search over cards, for every list of them.
//!
//! The deck builder, the zone dialog and the name chooser search the same
//! way: a box holding a line of the query language ([`crate::cardquery`]), a
//! gear that takes that line apart into the builder's controls
//! ([`crate::filterdialog`]), and a list that answers. This is what they
//! share: the box and the builder as one thing ([`SearchBox`]), the keys both
//! answer ([`Gesture`]), and a pool row as the language reads it
//! ([`pool_card_matches`]). A criterion is a [`crate::cardquery::Key`] with a
//! [`crate::filterdialog::Control`], and adding one there adds it to all three.
//!
//! # Which languages a search speaks
//!
//! **The client's language and English, and no other** (owner, 4.11.2026): a
//! Spanish client finds a card by its Spanish and its English name, never by
//! its German one. So a row hands the query its name as the client shows it,
//! its English name, and the other spellings of those two
//! ([`PoolCard::alt_names`]), and names in any other language stay out of
//! [`crate::cardquery::Facts`] altogether ([`PoolCard::foreign_names`]).

use crate::cardquery::{Colors, Facts, Flag, Query, Surface};
use crate::deckbuilder::{Coverage, PoolCard};
use crate::filterdialog::{Act, FilterPanel};
use crate::textbuf::{Dir, Step, TextBuffer};

/// A search box and the filter builder its gear opens.
///
/// The builder is a *mode* of the box, not a second field: the box keeps
/// showing the string, and what the builder changes is written back into it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchBox {
    /// The line in the box.
    pub field: TextBuffer,
    /// The builder, while the gear is open.
    panel: Option<FilterPanel>,
}

impl SearchBox {
    /// The line in the box, read as a query.
    #[must_use]
    pub fn parsed(&self) -> Query {
        crate::cardquery::parse(self.field.text())
    }

    /// The builder, while the gear is open.
    #[must_use]
    pub const fn panel(&self) -> Option<&FilterPanel> {
        self.panel.as_ref()
    }

    /// Opens the builder on what is in the box, or closes it.
    pub fn toggle_panel(&mut self) {
        self.panel = if self.panel.is_some() {
            None
        } else {
            Some(FilterPanel::open(&self.parsed()))
        };
    }

    /// Shuts the builder.
    pub fn close_panel(&mut self) {
        self.panel = None;
    }

    /// One editing gesture in the builder, and the box written after it.
    /// Answers whether the box changed, for an owner that filters on it.
    ///
    /// The box is written from [`FilterPanel::written`], which answers `None`
    /// until a row has actually been changed, so opening the builder to look
    /// at a string leaves the string alone, down to its spelling.
    pub fn in_panel(&mut self, edit: impl FnOnce(&mut FilterPanel)) -> bool {
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        edit(panel);
        let Some(query) = panel.written() else {
            return false;
        };
        let text = crate::cardquery::render(&query);
        if text == self.field.text() {
            return false;
        }
        self.field = TextBuffer::new(&text);
        true
    }

    /// Does what a button in the builder means.
    pub fn act(&mut self, act: Act) -> bool {
        self.in_panel(|panel| panel.act(act))
    }

    /// Whether a row of the builder holds the caret, rather than the box.
    #[must_use]
    pub fn typing_in_panel(&self) -> bool {
        self.panel.as_ref().is_some_and(|p| p.typing().is_some())
    }

    /// One key, to whichever field holds the caret. Answers whether it was
    /// the box that was told it is done, so its owner can let the keyboard go.
    pub fn gesture(&mut self, gesture: Gesture) -> bool {
        if self.typing_in_panel() {
            self.in_panel(|panel| gesture.on_panel(panel));
            false
        } else {
            gesture.on_field(&mut self.field)
        }
    }
}

/// One editing key, before it is aimed at a field. The box and a builder row
/// take the same keys because they are the same kind of thing, on every
/// surface: a table of keys written twice agrees until somebody adds a chord
/// to one half of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Backspace.
    Back,
    /// Delete.
    Forward,
    /// An arrow, Home or End: how far, which way, and whether it selects.
    Move(Step, Dir, bool),
    /// Select everything.
    SelectAll,
    /// Enter: hand the keyboard back.
    Done,
    /// A character, or a whole run of them from an IME or a paste.
    Insert(String),
}

impl Gesture {
    /// Does it to the box's own text. Answers whether it was [`Self::Done`].
    pub fn on_field(self, field: &mut TextBuffer) -> bool {
        match self {
            Self::Back => {
                field.delete_back();
            }
            Self::Forward => field.delete_forward(),
            Self::Move(step, dir, select) => field.move_caret(step, dir, select),
            Self::SelectAll => field.select_all(),
            Self::Done => return true,
            Self::Insert(mut text) => {
                text.retain(|c| !c.is_control());
                if !text.is_empty() {
                    field.insert(&text);
                }
            }
        }
        false
    }

    /// Does it to the builder row that holds the caret.
    pub fn on_panel(self, panel: &mut FilterPanel) {
        match self {
            Self::Back => {
                panel.pop_typed();
            }
            Self::Forward => panel.delete_typed_forward(),
            Self::Move(step, dir, select) => panel.move_typing_caret(step, dir, select),
            Self::SelectAll => panel.select_all_typed(),
            Self::Done => panel.stop_typing(),
            Self::Insert(text) => panel.type_text(&text),
        }
    }
}

/// Whether a pool row answers `query`.
#[must_use]
pub fn pool_card_matches(query: &Query, card: &PoolCard) -> bool {
    if query.is_anything() {
        return true;
    }
    let flags = pool_flags(card);
    crate::cardquery::matches(query, &pool_facts(card, &flags), Surface::POOL).shown()
}

/// The flags a pool row carries, for `is:`.
///
/// A list and not a set of `bool` fields: [`Facts::flags`] is what a card
/// *is*, and [`Surface::POOL`] is what may be asked — the pair is how
/// "this card is not a commander" and "nobody here knows about commanders"
/// stay two different answers.
fn pool_flags(card: &PoolCard) -> Vec<Flag> {
    let mut flags = Vec::with_capacity(4);
    flags.push(match card.coverage {
        // Generated is playable too, so `is:playable` finds it;
        // `is:generated` apart below.
        Coverage::Implemented | Coverage::Generated => Flag::Playable,
        Coverage::Partial => Flag::Partial,
        Coverage::Unimplemented => Flag::Stub,
    });
    if card.coverage == Coverage::Generated {
        flags.push(Flag::Generated);
    }
    if card.commander {
        flags.push(Flag::Commander);
    }
    if card.basic_land {
        flags.push(Flag::Basic);
    }
    // The rules question, not the picture one: this filter is printed to the
    // player as "double-faced" and reached by the alias `transform`, and
    // CR 715.1 makes an adventurer card a two-part frame rather than a second
    // side. It asked `two_faced` — the compiled face count — until #115, and
    // answered with nine Adventures and two Splits.
    if card.double_faced {
        flags.push(Flag::Dfc);
    }
    flags
}

/// A pool row, as the query language reads it.
fn pool_facts<'a>(card: &'a PoolCard, flags: &'a [Flag]) -> Facts<'a> {
    let (power, toughness, loyalty) = printed_numbers(card.stats.as_deref());
    Facts {
        name: &card.name,
        english_name: &card.english_name,
        alt_names: &card.alt_names,
        type_line: &card.type_line,
        kinds: &card.kinds,
        oracle: &card.oracle_text,
        colors: Colors::of_letters(&card.colors),
        identity: Colors::of_letters(&card.identity),
        mana_cost: &card.mana_cost,
        mana_value: card.cmc,
        power,
        toughness,
        loyalty,
        flags,
    }
}

/// The numbers `PoolCard::stats` is holding, told apart by their shape.
///
/// `baylee_cards::pool::stats` writes `p/t` for anything with a power and a
/// toughness and the bare number otherwise, so the slash is the whole of
/// what distinguishes a creature from a planeswalker here. Reading it back
/// rather than asking the pool for three fields keeps the wire as it is;
/// what it costs is that a card printing both — none in this pool — would
/// be read as a creature, which is what the row already draws it as.
fn printed_numbers(stats: Option<&str>) -> (Option<i32>, Option<i32>, Option<i32>) {
    let Some(stats) = stats else {
        return (None, None, None);
    };
    match stats.split_once('/') {
        Some((power, toughness)) => (
            power.trim().parse().ok(),
            toughness.trim().parse().ok(),
            None,
        ),
        None => (None, None, stats.trim().parse().ok()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardquery::parse;

    /// A card as a Spanish client is served it: a German name is a third
    /// language there, and the gateway files it where no search reads.
    fn bolt() -> PoolCard {
        PoolCard {
            name: "Rayo".to_string(),
            english_name: "Lightning Bolt".to_string(),
            foreign_names: vec!["Blitzschlag".to_string()],
            ..PoolCard::default()
        }
    }

    #[test]
    fn a_search_speaks_the_clients_language_and_english_and_no_other() {
        let card = bolt();
        for found in ["rayo", "lightning", "!\"Lightning Bolt\"", "name:rayo"] {
            assert!(pool_card_matches(&parse(found), &card), "{found}");
        }
        for missed in ["blitzschlag", "name:blitz", "!blitzschlag"] {
            assert!(!pool_card_matches(&parse(missed), &card), "{missed}");
        }
    }

    #[test]
    fn a_change_in_the_builder_writes_the_box_and_says_so() {
        let mut search = SearchBox {
            field: TextBuffer::new("t:creature"),
            panel: None,
        };
        assert!(!search.in_panel(|_| {}), "no builder is open");
        search.toggle_panel();
        assert!(!search.in_panel(|_| {}), "looking changes nothing");
        assert_eq!(search.field.text(), "t:creature");
        assert!(search.act(Act::Remove(0)), "taking the row out does");
        assert_eq!(search.field.text(), "");
    }

    #[test]
    fn a_key_goes_to_the_field_that_holds_the_caret() {
        let mut search = SearchBox::default();
        assert!(!search.gesture(Gesture::Insert("t:land".to_string())));
        assert_eq!(search.field.text(), "t:land");
        search.toggle_panel();
        search.act(Act::Edit(0));
        assert!(search.typing_in_panel());
        assert!(
            !search.gesture(Gesture::Done),
            "the row gives the caret back"
        );
        assert!(!search.typing_in_panel());
        assert!(search.gesture(Gesture::Done), "and then the box is done");
    }
}
