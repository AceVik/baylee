//! References in a report's text: `[Lightning Bolt]` for a card and
//! `[@steady 1]` for a player (window B, `windows-b6/DESIGN.md` §B.3).
//!
//! The text stays plain and reads on its own; the references are derived
//! from it. Typing `#` and a letter, or `@` and a character, offers what may
//! be named ([`trigger`], [`best`]); taking one writes the name in brackets
//! ([`taken`]); and on Send the brackets that name a candidate become
//! [`Refs`] in the report's `client` object, with the identity the text
//! cannot carry: the card's registry index, its printing, the object and
//! zone, the seat number. A bracket that names nothing stays text.
//!
//! # Which cards `#` may offer
//!
//! At a table, exactly the card identities the seat's own [`PlayerView`]
//! carries, through the one walk the print table is built from
//! ([`PlayerView::identities`]): a zone added to the view later is in both
//! or in neither, and a library or another seat's hand, being counts in the
//! view, cannot be offered at all. Narrower still on purpose ([`zone_of`]):
//! never a hand but the seat's own (a teammate's shared hand was shown for
//! play, not for a report that leaves the table), and never a face-down
//! object, not even one the seat controls — a report leaves the table, and a
//! morph's identity is the one secret its controller keeps from everyone at
//! it. Nothing is matched against anything but this list, so a prefix query
//! cannot reach a hidden name.
//!
//! Two more sources widen it without widening what the seat knows (#14):
//! a token that copies a card is offered as that card ([`candidates`]'s
//! `copies`), and every card the seat's own game log named to it that the
//! view no longer shows comes after the rest ([`logged`], [`RefZone::Logged`]),
//! because a card revealed and shuffled away, or cast and exiled since, was
//! seen at this table all the same. The log names a card to a seat exactly
//! as its view showed it then, so this is the same rule over time.
//!
//! Outside a game the client offers the compiled pool instead (no object,
//! no zone), which it builds itself: client-core links no card registry.

use std::collections::BTreeSet;
use std::ops::Range;

use baylee_core::ids::{CardIndex, ObjectId, PlayerId, PrintRef};
use baylee_view::{GameStatic, PlayerView, SeenIn};
use serde::Serialize;

use crate::card_face::shown_name;
use crate::gamelog::CardTextLookup;

/// How many suggestions stand at once.
pub const SHOWN: usize = 6;

/// The longest run after a sigil that is still read as a query: past this
/// it is prose that happens to follow a `#`.
const QUERY_CHARS: usize = 40;

/// Where a referenced card was, in the words of the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefZone {
    /// The seat's own hand.
    Hand,
    /// The battlefield.
    Battlefield,
    /// The stack.
    Stack,
    /// A graveyard.
    Graveyard,
    /// Exile, face up.
    Exile,
    /// A command zone, or a commander wherever it is (CR 903.3).
    Command,
    /// Shown to the seat for the question it is answering.
    Shown,
    /// A library's revealed top card.
    LibraryTop,
    /// Named to the seat by its game log, and no longer anywhere its view
    /// shows: a card revealed, cast or seen in a public zone earlier in the
    /// game (#14).
    Logged,
}

/// Whether a card the view showed in `seen` may be offered, and under what
/// zone. Exhaustive on purpose: a zone the view gains later stops here until
/// somebody decides it.
#[expect(
    clippy::match_same_arms,
    reason = "two different reasons to offer nothing, each said where it is"
)]
#[must_use]
pub const fn zone_of(seen: SeenIn) -> Option<RefZone> {
    match seen {
        SeenIn::Hand => Some(RefZone::Hand),
        // Q-B1: a teammate's hand was shown for play; a report is a new
        // audience. A hand seen through control (CR 722.4) is another
        // player's hand all the same.
        SeenIn::SharedHand | SeenIn::ControlledHand => None,
        SeenIn::Battlefield => Some(RefZone::Battlefield),
        SeenIn::Stack => Some(RefZone::Stack),
        SeenIn::Graveyard => Some(RefZone::Graveyard),
        SeenIn::Exile => Some(RefZone::Exile),
        SeenIn::Command | SeenIn::Commander => Some(RefZone::Command),
        SeenIn::LookingAt => Some(RefZone::Shown),
        SeenIn::LibraryTop => Some(RefZone::LibraryTop),
        // Descriptions of an object as it was (a source, a target), and the
        // source of the question being asked: each is also where it is now,
        // or was a moment the player has already moved past.
        SeenIn::TargetingSource | SeenIn::DamageSource | SeenIn::TargetObject => None,
    }
}

/// A printing, as a reference names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefPrint {
    /// Scryfall's printing id.
    pub scryfall_id: String,
    /// The printing's language, lower case.
    pub lang: String,
    /// Its finish, lower case.
    pub finish: String,
}

/// A card `#` may offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRef {
    /// The name the player sees: the localized one where the client has it.
    pub name: String,
    /// The English name, matched as well.
    pub english: String,
    /// The registry index.
    pub card: CardIndex,
    /// The face showing.
    pub face: u8,
    /// The table's printing, for the preview (a table's candidates only).
    pub art: Option<PrintRef>,
    /// The printing, as the wire names it.
    pub print: Option<RefPrint>,
    /// The object, at a table.
    pub object: Option<ObjectId>,
    /// Where it was, at a table.
    pub zone: Option<RefZone>,
    /// Whose it is, at a table.
    pub owner: Option<PlayerId>,
    /// Its type line, outside a table (the list's second column there).
    pub kind: Option<String>,
}

/// A player `@` may offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerRef {
    /// The name exactly as the player sees it.
    pub name: String,
    /// The seat number the view uses; never an account.
    pub seat: Option<u8>,
    /// The account's tag, where the chat knows it (a friend, a member, a
    /// handle `Name#tag` at a table); a report leaves it out.
    pub tag: Option<String>,
}

/// Everything a report's text may name, as gathered when the form opened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidates {
    /// Cards, in the order they are offered on a tie.
    pub cards: Vec<CardRef>,
    /// Players.
    pub players: Vec<PlayerRef>,
}

/// The cards the seat's own `view` may name: one row per object the walk
/// shows face up in an offered zone ([`zone_of`]), one per commander that
/// stands in no such zone, and one per printing where the same card lies
/// twice in one zone of one player (four Islands are one suggestion).
#[must_use]
pub fn candidates(
    view: &PlayerView,
    statics: Option<&GameStatic>,
    texts: &dyn CardTextLookup,
) -> Vec<CardRef> {
    let mut objects: BTreeSet<ObjectId> = BTreeSet::new();
    let mut rows: Vec<CardRef> = Vec::new();
    for seen in view.identities() {
        if seen.face_down {
            continue;
        }
        let Some(zone) = zone_of(seen.zone) else {
            continue;
        };
        // A commander in its command zone is that zone's object; one the
        // seat list names elsewhere (a hand, a library) is named without one.
        let object = if seen.zone == SeenIn::Commander {
            if seen.object.is_some_and(|o| objects.contains(&o)) {
                continue;
            }
            None
        } else {
            seen.object
        };
        if let Some(object) = object {
            objects.insert(object);
        }
        let card = seen.card;
        let same = |row: &CardRef| {
            row.card == card.index
                && row.art == Some(card.print)
                && row.zone == Some(zone)
                && row.owner == seen.owner
        };
        if rows.iter().any(same) {
            continue;
        }
        let text = texts.text(card.index, card.face);
        rows.push(CardRef {
            name: shown_name(seen.name, text.as_ref()).to_string(),
            english: seen.name.to_string(),
            card: card.index,
            face: card.face,
            art: Some(card.print),
            print: statics
                .and_then(|s| s.print(card.print))
                .map(|entry| RefPrint {
                    scryfall_id: entry.scryfall_id.clone(),
                    lang: entry.lang.to_lowercase(),
                    finish: format!("{:?}", entry.finish).to_lowercase(),
                }),
            object,
            zone: Some(zone),
            owner: seen.owner,
            kind: None,
        });
    }
    copies(view, texts, &mut rows);
    rows
}

/// A token that is a copy of a card (Progenitor Mimic's, a Clone's token),
/// face up on the battlefield or the stack: it has no card of its own for
/// the print table's walk to find, but it is the card its rules are
/// printed on (CR 707.2), by the name it shows.
fn copies(view: &PlayerView, texts: &dyn CardTextLookup, rows: &mut Vec<CardRef>) {
    let public = view
        .battlefield
        .iter()
        .map(|o| (o, RefZone::Battlefield))
        .chain(view.stack.iter().map(|o| (o, RefZone::Stack)));
    for (object, zone) in public {
        let (None, Some(rules)) = (object.card, object.rules) else {
            continue;
        };
        if object.status.is_face_down() || object.stack_item.is_some() {
            continue;
        }
        let same = |row: &CardRef| {
            row.card == rules.card && row.zone == Some(zone) && row.owner == Some(object.owner)
        };
        if rows.iter().any(same) {
            continue;
        }
        let text = texts.text(rules.card, rules.face);
        rows.push(CardRef {
            name: shown_name(&object.name, text.as_ref()).to_string(),
            english: object.name.clone(),
            card: rules.card,
            face: rules.face,
            art: None,
            print: None,
            object: Some(object.id),
            zone: Some(zone),
            owner: Some(object.owner),
            kind: None,
        });
    }
}

/// Every card the seat's game log named to it that `rows` does not offer
/// yet, once each, after them ([`RefZone::Logged`]): a card an opponent
/// revealed and shuffled away, a spell that was cast and exiled since, a
/// creature that died and left the graveyard. The log names a card to a
/// seat exactly as its view showed it then, so nothing here is a card the
/// seat was not shown: a hidden card is named by no line, and a face-down
/// one by its handle alone.
pub fn logged(
    book: &crate::gamelog::LogBook,
    statics: Option<&GameStatic>,
    texts: &dyn CardTextLookup,
    rows: &mut Vec<CardRef>,
) {
    for entry in book.entries() {
        for object in entry.event.objects() {
            let baylee_view::LogObject::Known {
                card: Some(card),
                name,
                ..
            } = object
            else {
                continue;
            };
            if rows.iter().any(|row| row.card == card.index) {
                continue;
            }
            let text = texts.text(card.index, card.face);
            rows.push(CardRef {
                name: shown_name(name, text.as_ref()).to_string(),
                english: name.clone(),
                card: card.index,
                face: card.face,
                art: Some(card.print),
                print: statics
                    .and_then(|s| s.print(card.print))
                    .map(|entry| RefPrint {
                        scryfall_id: entry.scryfall_id.clone(),
                        lang: entry.lang.to_lowercase(),
                        finish: format!("{:?}", entry.finish).to_lowercase(),
                    }),
                object: None,
                zone: Some(RefZone::Logged),
                owner: None,
                kind: None,
            });
        }
    }
}

/// The players `@` may name at a table: every seat but the reader's own
/// (the player writes "I"), by the name the roster shows.
#[must_use]
pub fn table_players(statics: &GameStatic, me: PlayerId) -> Vec<PlayerRef> {
    statics
        .seats
        .iter()
        .filter(|seat| seat.player != me)
        .map(|seat| PlayerRef {
            name: seat.display_name.clone(),
            seat: Some(seat.player.get()),
            tag: None,
        })
        .collect()
}

/// What a reference is to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sigil {
    /// `#`: a card.
    Card,
    /// `@`: a player.
    Player,
}

impl Sigil {
    /// The character that opens it.
    #[must_use]
    pub const fn char(self) -> char {
        match self {
            Self::Card => '#',
            Self::Player => '@',
        }
    }
}

/// A reference being typed: its sigil, what follows it up to the caret, and
/// the bytes from the sigil to the caret (what taking one replaces).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trigger {
    /// Card or player.
    pub sigil: Sigil,
    /// What was typed after the sigil.
    pub query: String,
    /// The sigil's byte, up to the caret.
    pub at: Range<usize>,
}

/// The reference the caret is at the end of, if any: a run `#` + a letter
/// (`#300` is an issue number, `# ` a heading, `##` neither) or `@` + any
/// character, the sigil at the start of the text or after a character that
/// is no letter or digit (an address's `@` is not one), on one line, no
/// bracket in it and at most [`QUERY_CHARS`] long.
///
/// Read off the text after each change, never off a key: on a German
/// keyboard `@` is `AltGr`+`Q`, and a browser types for the client.
#[must_use]
pub fn trigger(text: &str, caret: usize) -> Option<Trigger> {
    let before = text.get(..caret)?;
    let (at, sigil) = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| !matches!(c, '\n' | '[' | ']'))
        .find_map(|(i, c)| match c {
            '#' => Some((i, Sigil::Card)),
            '@' => Some((i, Sigil::Player)),
            _ => None,
        })?;
    let query = &before[at + 1..];
    if query.chars().count() > QUERY_CHARS {
        return None;
    }
    if before[..at]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || matches!(c, '#' | '@' | '['))
    {
        return None;
    }
    let first = query.chars().next()?;
    let opens = match sigil {
        Sigil::Card => first.is_alphabetic(),
        Sigil::Player => !first.is_whitespace() && first != '#' && first != '@',
    };
    opens.then(|| Trigger {
        sigil,
        query: query.to_string(),
        at: at..caret,
    })
}

/// Where `query` is in `name`, case aside: `Some(true)` at the start of the
/// first word, `Some(false)` at a later word or inside one.
fn word_prefix(name: &str, query: &str) -> Option<bool> {
    let name = name.to_lowercase();
    let query = query.to_lowercase();
    if name.starts_with(&query) {
        return Some(true);
    }
    let mut prior = ' ';
    for (i, c) in name.char_indices() {
        let start = !prior.is_alphanumeric() && c.is_alphanumeric();
        prior = c;
        if i > 0 && start && name[i..].starts_with(&query) {
            return Some(false);
        }
    }
    // Inside a word too (owner, beta.7: `#mimic` should find a
    // Mimic within a longer word), ranked with the later words.
    name.contains(&query).then_some(false)
}

/// The best [`SHOWN`] cards for `query`, as indexes into `cards`: a prefix
/// of any word of the shown or the English name, case aside; the first
/// word first, then the shorter name, then the zone (hand, battlefield,
/// stack, …), then the order offered.
#[must_use]
pub fn best(cards: &[CardRef], query: &str) -> Vec<usize> {
    let mut found: Vec<(bool, usize, Option<RefZone>, usize)> = cards
        .iter()
        .enumerate()
        .filter_map(|(i, card)| {
            let first = match (
                word_prefix(&card.name, query),
                word_prefix(&card.english, query),
            ) {
                (None, None) => return None,
                (a, b) => a.unwrap_or(false) || b.unwrap_or(false),
            };
            Some((!first, card.name.chars().count(), card.zone, i))
        })
        .collect();
    found.sort_by(|a, b| {
        (a.0, a.1, a.2.is_none(), a.2, a.3).cmp(&(b.0, b.1, b.2.is_none(), b.2, b.3))
    });
    found.into_iter().take(SHOWN).map(|f| f.3).collect()
}

/// The best [`SHOWN`] players for `query`, as indexes into `players`.
#[must_use]
pub fn best_players(players: &[PlayerRef], query: &str) -> Vec<usize> {
    let mut found: Vec<(bool, usize, usize)> = players
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            word_prefix(&p.name, query).map(|first| (!first, p.name.chars().count(), i))
        })
        .collect();
    found.sort_unstable();
    found.into_iter().take(SHOWN).map(|f| f.2).collect()
}

/// What taking a suggestion writes over the typed run: the name in
/// brackets, `@` inside them for a player, and a space after.
#[must_use]
pub fn taken(sigil: Sigil, name: &str) -> String {
    match sigil {
        Sigil::Card => format!("[{name}] "),
        Sigil::Player => format!("[@{name}] "),
    }
}

/// One finished reference in a text: its bytes, brackets included, and
/// what it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The bytes from `[` to `]`, both included.
    pub at: Range<usize>,
    /// What it names.
    pub target: Target,
}

/// What a finished reference names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A card.
    Card(CardRef),
    /// A player.
    Player(PlayerRef),
}

/// Every `[Name]` and `[@Name]` in `text` that names a candidate: a card by
/// its shown or English name (case aside; one the player `picked` from the
/// suggestions before any other of that name), a player by the name as
/// shown. A bracket naming nothing is no span: it is the player's own
/// bracket and stays text.
#[must_use]
pub fn parse(text: &str, candidates: &Candidates, picked: &[CardRef]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match c {
            '[' => open = Some(i),
            '\n' => open = None,
            ']' => {
                if let Some(start) = open.take()
                    && let Some(target) = named(&text[start + 1..i], candidates, picked)
                {
                    spans.push(Span {
                        at: start..i + 1,
                        target,
                    });
                }
            }
            _ => {}
        }
    }
    spans
}

/// What the words inside one pair of brackets name.
fn named(inner: &str, candidates: &Candidates, picked: &[CardRef]) -> Option<Target> {
    if let Some(name) = inner.strip_prefix('@') {
        return candidates
            .players
            .iter()
            .find(|p| p.name == name)
            .cloned()
            .map(Target::Player);
    }
    if inner.is_empty() {
        return None;
    }
    let lower = inner.to_lowercase();
    let is =
        |card: &&CardRef| card.name.to_lowercase() == lower || card.english.to_lowercase() == lower;
    picked
        .iter()
        .find(is)
        .or_else(|| candidates.cards.iter().find(is))
        .cloned()
        .map(Target::Card)
}

/// A card reference as the report carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CardMention {
    /// The name as written.
    pub text: String,
    /// Where in `text`, in characters, brackets included: `[start, end)`.
    pub at: [usize; 2],
    /// The registry index (the append-only ledger's: stable across builds).
    pub card: u32,
    /// The printing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print: Option<RefPrint>,
    /// The face showing.
    pub face: u8,
    /// The object, at a table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ObjectId>,
    /// Where it was, at a table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<RefZone>,
    /// The owner's seat number, at a table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<u8>,
}

/// A player reference as the report carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlayerMention {
    /// The name as written.
    pub text: String,
    /// Where in `text`, in characters, brackets included: `[start, end)`.
    pub at: [usize; 2],
    /// The seat number the view uses; never an account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seat: Option<u8>,
    /// The account's tag (the chat's mentions); `None` in a report, which
    /// then serialises exactly as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// The references a report's text makes: `client.refs`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Refs {
    /// The cards.
    pub cards: Vec<CardMention>,
    /// The players.
    pub players: Vec<PlayerMention>,
}

impl Refs {
    /// Whether the text names nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.players.is_empty()
    }
}

/// The references `text` makes ([`parse`]), positioned in characters, not
/// bytes; `None` when it makes none, so a report with no reference carries
/// no key for them.
#[must_use]
pub fn refs_for(text: &str, candidates: &Candidates, picked: &[CardRef]) -> Option<Refs> {
    let chars = |byte: usize| text[..byte].chars().count();
    let mut refs = Refs::default();
    for span in parse(text, candidates, picked) {
        let at = [chars(span.at.start), chars(span.at.end)];
        let inner = &text[span.at.start + 1..span.at.end - 1];
        match span.target {
            Target::Card(card) => refs.cards.push(CardMention {
                text: inner.to_string(),
                at,
                card: card.card.get(),
                print: card.print,
                face: card.face,
                object: card.object,
                zone: card.zone,
                owner: card.owner.map(PlayerId::get),
            }),
            Target::Player(player) => refs.players.push(PlayerMention {
                text: inner.trim_start_matches('@').to_string(),
                at,
                seat: player.seat,
                tag: player.tag,
            }),
        }
    }
    (!refs.is_empty()).then_some(refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_face::CardText;
    use crate::test_support::{ViewBuilder, printed, statics, token};
    use baylee_view::{ObjectStatus, PublicObject, SharedHand};

    fn no_text(_: CardIndex, _: u8) -> Option<CardText> {
        None
    }

    fn german(card: CardIndex, _: u8) -> Option<CardText> {
        (card.get() == 3).then(|| CardText {
            lang: "de".into(),
            name: "Blitzschlag".into(),
            type_line: String::new(),
            oracle_text: String::new(),
            mana_cost: String::new(),
            english_name: "Lightning Bolt".into(),
        })
    }

    fn opens(text: &str) -> Option<(Sigil, String)> {
        trigger(text, text.len()).map(|t| (t.sigil, t.query))
    }

    /// `#` and a letter opens, `@` and a character opens; a number, a
    /// heading, a doubled sigil, a word's inner `@` and a bracket do not.
    #[test]
    fn the_trigger_opens_on_a_letter_and_on_nothing_else() {
        assert_eq!(opens("#W"), Some((Sigil::Card, "W".into())));
        assert_eq!(opens("cast #Wr"), Some((Sigil::Card, "Wr".into())));
        assert_eq!(opens("hit (#bolt"), Some((Sigil::Card, "bolt".into())));
        assert_eq!(opens("@s"), Some((Sigil::Player, "s".into())));
        assert_eq!(opens("then @1"), Some((Sigil::Player, "1".into())));
        assert_eq!(opens("#Wrath of"), Some((Sigil::Card, "Wrath of".into())));
        for shut in [
            "#",
            "#3",
            "issue #300",
            "# heading",
            "## h",
            "##W",
            "@",
            "@ x",
            "mail me@host",
            "a#W",
            "[#W",
            "#W]",
            "#W\nx",
            "@#x",
        ] {
            assert_eq!(opens(shut), None, "{shut:?}");
        }
        // Only up to the caret: a run after it is not being typed.
        assert_eq!(trigger("#Wr x", 3).map(|t| t.query), Some("Wr".into()));
        assert_eq!(trigger("#Wr", 1), None);
        // `@` typed by `AltGr`+`Q` is a text change like any other.
        assert_eq!(opens("AltGr gives @q"), Some((Sigil::Player, "q".into())));
        let long = format!("#{}", "a".repeat(QUERY_CHARS + 1));
        assert_eq!(opens(&long), None, "prose after a sigil is not a query");
    }

    fn card(name: &str, index: u32, zone: Option<RefZone>) -> CardRef {
        CardRef {
            name: name.into(),
            english: name.into(),
            card: CardIndex::new(index),
            face: 0,
            art: Some(PrintRef::new(u16::try_from(index).expect("small"))),
            print: None,
            object: zone.map(|_| ObjectId::new(index, 0)),
            zone,
            owner: zone.map(|_| PlayerId::new(0)),
            kind: None,
        }
    }

    /// Any word's prefix matches, the first word's first, then the shorter
    /// name, then the zone; six at most.
    #[test]
    fn the_best_six_by_first_word_then_length_then_zone() {
        let cards = vec![
            card("Wrenn and Six", 1, Some(RefZone::Battlefield)),
            card("Wrath of God", 2, Some(RefZone::Graveyard)),
            card("Day of Wrath", 3, Some(RefZone::Hand)),
            card("Wrath", 4, None),
            card("Lightning Bolt", 5, Some(RefZone::Hand)),
        ];
        assert_eq!(best(&cards, "wr"), [3, 1, 0, 2]);
        assert_eq!(best(&cards, "WRATH"), [3, 1, 2]);
        assert_eq!(best(&cards, "of w"), [2]);
        assert_eq!(best(&cards, "bolt"), [4]);
        assert!(best(&cards, "zz").is_empty());
        // Inside a word too (owner, beta.7).
        let inside = vec![
            card("Phantasmal Image", 1, None),
            card("Metamimic", 2, None),
        ];
        assert_eq!(best(&inside, "mimic"), [1]);
        assert_eq!(best(&inside, "TASM"), [0]);
        let many: Vec<CardRef> = (0..10)
            .map(|i| card(&format!("Island {i}"), i, Some(RefZone::Battlefield)))
            .collect();
        assert_eq!(best(&many, "isl").len(), SHOWN);
    }

    fn with(cards: Vec<CardRef>, players: Vec<PlayerRef>) -> Candidates {
        Candidates { cards, players }
    }

    /// `[Name]` and `[@Name]` naming a candidate are spans; a bracket the
    /// player typed around anything else stays text.
    #[test]
    fn a_bracket_is_a_reference_only_when_it_names_a_candidate() {
        let candidates = with(
            vec![card("Lightning Bolt", 3, Some(RefZone::Stack))],
            vec![PlayerRef {
                name: "steady 1".into(),
                seat: Some(1),
                tag: None,
            }],
        );
        let text = "cast [Lightning Bolt] at [@steady 1], not [x] or [@nobody] or [lightning bolt]";
        let spans = parse(text, &candidates, &[]);
        let named: Vec<&str> = spans.iter().map(|s| &text[s.at.clone()]).collect();
        assert_eq!(
            named,
            ["[Lightning Bolt]", "[@steady 1]", "[lightning bolt]"]
        );
        assert!(matches!(spans[1].target, Target::Player(_)));
        assert!(parse("[Lightning\nBolt]", &candidates, &[]).is_empty());
        assert!(parse("[]", &candidates, &[]).is_empty());
        // Taking writes what parsing reads back.
        let taken_text = format!("x {}", taken(Sigil::Card, "Lightning Bolt"));
        assert_eq!(parse(&taken_text, &candidates, &[]).len(), 1);
        let player = taken(Sigil::Player, "steady 1");
        assert_eq!(player, "[@steady 1] ");
        assert_eq!(parse(&player, &candidates, &[]).len(), 1);
    }

    /// Positions count characters, not bytes: a `ü` before a reference
    /// moves it by one.
    #[test]
    fn positions_are_characters_not_bytes() {
        let candidates = with(
            vec![card("Lightning Bolt", 3, Some(RefZone::Stack))],
            vec![PlayerRef {
                name: "Jörg#00a1".into(),
                seat: None,
                tag: None,
            }],
        );
        let refs = refs_for("für [Lightning Bolt] [@Jörg#00a1]", &candidates, &[])
            .expect("two references");
        assert_eq!(refs.cards[0].at, [4, 20]);
        assert_eq!(refs.cards[0].text, "Lightning Bolt");
        assert_eq!(refs.players[0].at, [21, 33]);
        assert_eq!(
            refs.players[0].text, "Jörg#00a1",
            "what is shown, tag and all"
        );
        assert_eq!(refs_for("no references [here]", &candidates, &[]), None);
    }

    /// A name two candidates share names the one the player picked.
    #[test]
    fn a_picked_card_wins_its_name() {
        let graveyard = card("Lightning Bolt", 3, Some(RefZone::Graveyard));
        let mut stack = card("Lightning Bolt", 3, Some(RefZone::Stack));
        stack.object = Some(ObjectId::new(99, 0));
        let candidates = with(vec![graveyard, stack.clone()], vec![]);
        let first = refs_for("[Lightning Bolt]", &candidates, &[]).expect("one");
        assert_eq!(first.cards[0].zone, Some(RefZone::Graveyard));
        let picked = refs_for("[Lightning Bolt]", &candidates, &[stack]).expect("one");
        assert_eq!(picked.cards[0].zone, Some(RefZone::Stack));
    }

    fn face_down(mut object: PublicObject) -> PublicObject {
        object.status = ObjectStatus::FACE_DOWN;
        object
    }

    /// One object in every zone the view has: each offered zone gives one
    /// row under its own zone; the seat's own morph, a teammate's shared
    /// hand and a token (no card) give none.
    #[test]
    fn each_zone_gives_its_row_and_the_hidden_ones_none() {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Opt", 1, 10)])
            .with_battlefield(
                0,
                [
                    printed(11, 0, "Island", 11),
                    token(12, 0, "Soldier", 1, 1),
                    face_down(printed(13, 0, "Morph Secret", 13)),
                ],
            )
            .with_stack(vec![printed(14, 1, "Lightning Bolt", 14)])
            .with_graveyard(1, vec![printed(15, 1, "Wrath of God", 15)])
            .with_exile(1, vec![printed(16, 1, "Path", 16)])
            .with_command(1, vec![printed(17, 1, "Atraxa", 17)])
            .with_looking_at(vec![printed(18, 0, "Tutored", 18)])
            .build();
        view.library_tops = vec![printed(19, 1, "Top Card", 19)];
        view.shared_hands = vec![SharedHand {
            public: false,
            player: PlayerId::new(1),
            cards: ViewBuilder::new(2)
                .with_hand(vec![("Shared Secret", 2, 20)])
                .build()
                .hand,
        }];
        let rows = candidates(&view, Some(&statics(30)), &no_text);
        let got: Vec<(&str, Option<RefZone>)> =
            rows.iter().map(|r| (r.name.as_str(), r.zone)).collect();
        assert_eq!(
            got,
            [
                ("Opt", Some(RefZone::Hand)),
                ("Island", Some(RefZone::Battlefield)),
                ("Lightning Bolt", Some(RefZone::Stack)),
                ("Wrath of God", Some(RefZone::Graveyard)),
                ("Path", Some(RefZone::Exile)),
                ("Atraxa", Some(RefZone::Command)),
                ("Tutored", Some(RefZone::Shown)),
                ("Top Card", Some(RefZone::LibraryTop)),
            ]
        );
        // The morph is its controller's to know, and still never offered:
        // not by its name, and not by a prefix of it.
        assert!(
            view.battlefield
                .iter()
                .any(|o| o.card.is_some() && o.status.is_face_down()),
            "the injected morph is in the view, card and all"
        );
        for hidden in ["Morph", "mor", "Secret", "Shared", "sh"] {
            assert!(
                best(&rows, hidden).is_empty(),
                "{hidden:?} reached a hidden name"
            );
        }
        let printing = rows[2].print.clone().expect("the stack's printing");
        assert_eq!(
            (printing.lang.as_str(), printing.finish.as_str()),
            ("en", "normal")
        );
    }

    /// The candidates are the walk the print table is built from: on a view
    /// with one card in each offered zone (and nothing in the zones a report
    /// leaves out), the printings offered are exactly the printings the
    /// view is entitled to.
    #[test]
    fn the_candidates_and_the_print_table_are_one_walk() {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Opt", 1, 10)])
            .with_battlefield(1, [printed(11, 1, "Island", 11)])
            .with_stack(vec![printed(14, 1, "Lightning Bolt", 14)])
            .with_graveyard(1, vec![printed(15, 1, "Wrath of God", 15)])
            .with_exile(0, vec![printed(16, 0, "Path", 16)])
            .with_command(1, vec![printed(17, 1, "Atraxa", 17)])
            .with_looking_at(vec![printed(18, 0, "Tutored", 18)])
            .build();
        view.library_tops = vec![printed(19, 1, "Top Card", 19)];
        let offered: BTreeSet<PrintRef> = candidates(&view, None, &no_text)
            .into_iter()
            .filter_map(|r| r.art)
            .collect();
        let entitled: BTreeSet<PrintRef> = view.prints().collect();
        assert_eq!(offered, entitled);
        assert_eq!(offered.len(), 8);
    }

    /// The injected finding the hidden-information test leans on: a view
    /// whose opponent holds three cards and shows one library top offers
    /// exactly that one card.
    #[test]
    fn a_counted_hand_offers_nothing_and_a_public_top_one_card() {
        let mut view = ViewBuilder::new(2).build();
        view.seats[1].hand_count = 3;
        view.library_tops = vec![printed(19, 1, "Top Card", 19)];
        let rows = candidates(&view, None, &no_text);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].zone, Some(RefZone::LibraryTop));
    }

    /// A name is what the player reads: the localized name, with the
    /// English one kept for matching; four of one card in one zone are one
    /// suggestion.
    #[test]
    fn a_card_is_named_as_shown_and_matched_in_english_too() {
        let view = ViewBuilder::new(2)
            .with_stack(vec![printed(3, 1, "Lightning Bolt", 3)])
            .with_battlefield(0, (20..24).map(|slot| printed(slot, 0, "Island", 9)))
            .build();
        let rows = candidates(&view, None, &german);
        assert_eq!(rows.len(), 2, "four Islands are one row");
        assert_eq!(
            (rows[1].name.as_str(), rows[1].english.as_str()),
            ("Blitzschlag", "Lightning Bolt")
        );
        assert_eq!(best(&rows, "light"), [1]);
        assert_eq!(best(&rows, "blitz"), [1]);
    }

    /// At a table every seat but the reader's own may be named, as the
    /// roster shows it.
    #[test]
    fn every_other_seat_may_be_named() {
        let mut table = statics(1);
        table.seats.push(baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "steady 1".into(),
            is_ai: true,
            away: false,
            team: None,
        });
        let players = table_players(&table, PlayerId::new(0));
        assert_eq!(
            players,
            [PlayerRef {
                name: "steady 1".into(),
                seat: Some(1),
                tag: None,
            }]
        );
        assert_eq!(best_players(&players, "st"), [0]);
        assert_eq!(best_players(&players, "1"), [0]);
        assert!(best_players(&players, "You").is_empty());
    }

    /// A token that copies a card has no card of its own, but it is the card
    /// its rules are printed on: it is offered under that card, once for all
    /// the copies a player has, and never while face down or when it has no
    /// rules to name.
    #[test]
    fn a_copy_token_is_offered_as_the_card_it_copies() {
        let copy = |slot, controller, name: &str| {
            let mut object = token(slot, controller, name, 2, 2);
            object.rules = Some(baylee_view::RulesFace {
                card: CardIndex::new(5),
                face: 0,
            });
            object
        };
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                [
                    copy(30, 0, "Lightning Bolt"),
                    copy(31, 0, "Lightning Bolt"),
                    face_down(copy(32, 0, "Hidden Copy")),
                    token(33, 0, "Soldier", 1, 1),
                ],
            )
            .with_battlefield(1, [copy(34, 1, "Lightning Bolt")])
            .build();
        let rows = candidates(&view, None, &german_five);
        let got: Vec<(&str, u32, Option<RefZone>, Option<PlayerId>)> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.card.get(), r.zone, r.owner))
            .collect();
        assert_eq!(
            got,
            [
                (
                    "Blitzschlag",
                    5,
                    Some(RefZone::Battlefield),
                    Some(PlayerId::new(0))
                ),
                (
                    "Blitzschlag",
                    5,
                    Some(RefZone::Battlefield),
                    Some(PlayerId::new(1))
                ),
            ],
            "one row per owner; a face-down copy and a plain token give none"
        );
        assert_eq!(rows[0].english, "Lightning Bolt");
        assert_eq!(rows[0].object, Some(ObjectId::new(30, 0)));
        assert_eq!(rows[0].art, None, "a token has no printing of its own");
        assert_eq!(best(&rows, "Light").len(), 2, "matched by its English name");
        assert!(best(&rows, "Hidden").is_empty());
    }

    /// A real card already offered is not offered again as a copy of it.
    #[test]
    fn a_copy_of_a_card_already_offered_in_its_zone_adds_nothing() {
        let mut copy = token(30, 0, "Island Copy", 0, 0);
        copy.rules = Some(baylee_view::RulesFace {
            card: CardIndex::new(11),
            face: 0,
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [printed(11, 0, "Island", 11), copy])
            .build();
        let rows = candidates(&view, None, &no_text);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].name, "Island");
    }

    fn german_five(card: CardIndex, _: u8) -> Option<CardText> {
        (card.get() == 5).then(|| CardText {
            lang: "de".into(),
            name: "Blitzschlag".into(),
            type_line: String::new(),
            oracle_text: String::new(),
            mana_cost: String::new(),
            english_name: "Lightning Bolt".into(),
        })
    }

    /// A line of the log naming `objects` (a revealing, so every one of them
    /// is one a seat was shown).
    fn naming(objects: Vec<baylee_view::LogObject>) -> baylee_view::LogEntry {
        baylee_view::LogEntry {
            at: 0,
            turn: 3,
            repeat: 1,
            event: baylee_view::LogEvent::Revealed {
                player: PlayerId::new(1),
                cards: objects,
            },
            phase: None,
        }
    }

    fn named(slot: u32, card: u32, name: &str) -> baylee_view::LogObject {
        baylee_view::LogObject::Known {
            id: ObjectId::new(slot, 0),
            card: Some(baylee_view::CardIdentity {
                index: CardIndex::new(card),
                print: PrintRef::new(u16::try_from(card).expect("small")),
                face: 0,
            }),
            token: None,
            name: name.into(),
        }
    }

    fn book_of(entries: Vec<baylee_view::LogEntry>) -> crate::gamelog::LogBook {
        let mut book = crate::gamelog::LogBook::new();
        book.append(
            &baylee_view::LogTail { from: 0, entries },
            &ViewBuilder::new(2).build(),
        );
        book
    }

    /// Every card the log named that the rows lack is added once, after
    /// them, as seen this game: with its printing, but no object and no
    /// owner, since it is nowhere in sight.
    #[test]
    fn a_card_the_log_named_is_offered_once_after_the_rest() {
        let book = book_of(vec![
            naming(vec![named(40, 5, "Lightning Bolt"), named(41, 6, "Opt")]),
            // Again, in another line, and as the target of an aim.
            naming(vec![named(40, 5, "Lightning Bolt")]),
            baylee_view::LogEntry {
                at: 0,
                turn: 3,
                repeat: 1,
                event: baylee_view::LogEvent::Aimed {
                    controller: PlayerId::new(1),
                    source: named(42, 8, "Wrath of God"),
                    ability: false,
                    rules: None,
                    modes: 0,
                    targets: vec![baylee_view::LogTarget::Object(named(43, 6, "Opt"))],
                },
                phase: None,
            },
        ]);
        let mut rows = vec![card("Island", 11, Some(RefZone::Battlefield))];
        logged(&book, Some(&statics(30)), &german_five, &mut rows);
        let got: Vec<(&str, u32, Option<RefZone>)> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.card.get(), r.zone))
            .collect();
        assert_eq!(
            got,
            [
                ("Island", 11, Some(RefZone::Battlefield)),
                ("Blitzschlag", 5, Some(RefZone::Logged)),
                ("Opt", 6, Some(RefZone::Logged)),
                ("Wrath of God", 8, Some(RefZone::Logged)),
            ]
        );
        let seen = &rows[1];
        assert_eq!(seen.english, "Lightning Bolt");
        assert_eq!((seen.object, seen.owner), (None, None));
        assert_eq!(seen.art, Some(PrintRef::new(5)));
        let print = seen.print.as_ref().expect("the table's printing");
        assert_eq!(
            (print.lang.as_str(), print.finish.as_str()),
            ("en", "normal")
        );
    }

    /// A card the rows already offer, in any zone, is not offered again as
    /// seen; neither is anything the log did not show the seat: a face-down
    /// permanent, a card it hid, a token.
    #[test]
    fn the_log_adds_no_card_that_is_offered_or_was_not_shown() {
        let token_named = baylee_view::LogObject::Known {
            id: ObjectId::new(44, 0),
            card: None,
            token: None,
            name: "Soldier".into(),
        };
        let book = book_of(vec![naming(vec![
            named(40, 5, "Lightning Bolt"),
            baylee_view::LogObject::FaceDown {
                id: ObjectId::new(45, 0),
            },
            baylee_view::LogObject::Hidden,
            token_named,
        ])]);
        let mut rows = vec![card("Lightning Bolt", 5, Some(RefZone::Graveyard))];
        logged(&book, None, &no_text, &mut rows);
        assert_eq!(rows.len(), 1, "{rows:?}");

        let mut none = Vec::new();
        logged(&book, None, &no_text, &mut none);
        assert_eq!(
            none.iter().map(|r| r.card.get()).collect::<Vec<_>>(),
            [5],
            "only the card the seat was shown"
        );
    }
}
