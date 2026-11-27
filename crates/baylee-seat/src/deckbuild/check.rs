//! A deck held to what a ranked host checks when it seats one, so a deck
//! that passes here is never refused at the table.
//!
//! The verdict is the host's own code: `baylee_cards::ranked::registration`
//! (every row a card the engine plays, counts and sizes a registration
//! takes), the gateway's blocked cards, then `baylee_cards::ranked::admit`
//! against the host's catalog table (the format's shape, every card's
//! verdict, rules-complete coverage, Highlander's singleton, a commander's
//! identity) — the steps `crates/baylee-gateway/src/ranked/deck.rs` takes —
//! and last [`Frozen::from_rows`], which is how the runner freezes the deck
//! it offers. `admit` answers one sentence for every fault; the model is
//! told each fault on its own, by card ([`Verdict::complaints`]), worded
//! from the same checks `admit` runs. A deck passes only when all of them
//! do, and then it has no complaint; one that does not has at least one.

use super::facts::Facts;
use crate::ranked::Frozen;
use baylee_account::ranked::Format;
use baylee_cards::formats::{self, Problem, Rules};
use baylee_client_core::cardquery::{Legality, PoolFacts};
use baylee_core::generated::index::CRYPTIC_SPIRES;
use baylee_core::ids::CardIndex;
use std::collections::BTreeSet;

/// A deck as its file holds it: rows of `N Name`, the commanders by name
/// and not among the main rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// The deck's name.
    pub name: String,
    /// The main deck, `N Name` each.
    pub main: Vec<String>,
    /// The sideboard; the decks made here have none.
    pub sideboard: Vec<String>,
    /// The commanders, by name.
    pub commanders: Vec<String>,
}

impl Rows {
    /// How many cards the main deck holds, commanders aside.
    #[must_use]
    pub fn main_count(&self) -> u32 {
        self.main
            .iter()
            .filter_map(|row| baylee_core::deckrow::parse(row).ok())
            .map(|row| row.count)
            .sum()
    }
}

/// What the check says about a deck.
#[derive(Debug)]
pub struct Verdict {
    /// Every fault, one sentence each, the card named; empty when the deck
    /// passed.
    pub complaints: Vec<String>,
    /// The deck frozen as the runner offers it, when it passed.
    pub frozen: Option<Frozen>,
}

impl Verdict {
    /// Whether the deck passed.
    #[must_use]
    pub const fn passed(&self) -> bool {
        self.frozen.is_some()
    }

    fn refused(complaints: Vec<String>) -> Self {
        Self {
            complaints,
            frozen: None,
        }
    }
}

/// The host's legality inputs, decoded once.
pub struct Checker<'a> {
    facts: &'a Facts,
    table: PoolFacts,
}

/// The catalog's key a ranked format is checked under
/// (`docs/banlists-and-formats.md` §"Ranked duels").
#[must_use]
pub const fn catalog_key(format: Format) -> &'static str {
    match format {
        Format::Standard => "standard",
        Format::Highlander => "vintage",
        Format::Commander => "commander",
    }
}

/// A card's name, for a sentence.
#[must_use]
pub fn name_of(card: CardIndex) -> String {
    baylee_cards::by_index(card).map_or_else(
        || format!("card #{}", card.get()),
        |def| def.name().to_string(),
    )
}

/// Whether the engine plays `card` rules-complete, which Ranked asks of
/// every card.
#[must_use]
pub fn complete(card: CardIndex) -> bool {
    baylee_cards::by_index(card).is_some_and(|def| {
        matches!(
            def.coverage,
            baylee_cards_dsl::Coverage::Implemented | baylee_cards_dsl::Coverage::Generated
        )
    })
}

impl<'a> Checker<'a> {
    /// A checker against `facts`.
    #[must_use]
    pub fn new(facts: &'a Facts) -> Self {
        Self {
            table: facts.table(),
            facts,
        }
    }

    /// The facts it checks against.
    #[must_use]
    pub const fn facts(&self) -> &Facts {
        self.facts
    }

    /// What the host's catalog says about `card` in `format`.
    #[must_use]
    pub fn verdict(&self, format: Format, card: CardIndex) -> Option<Legality> {
        self.table
            .of(card.get())
            .and_then(|facts| facts.legality(catalog_key(format)))
    }

    /// Whether a deck of `format` may hold `card` at `now`, as far as the
    /// card alone decides: rules-complete, played in the format by the
    /// catalog, not blocked. Cryptic Spires is left out: it needs two
    /// circled colours chosen, which the decks made here do not choose.
    #[must_use]
    pub fn playable(&self, format: Format, card: CardIndex, now: &str) -> bool {
        card != CRYPTIC_SPIRES
            && complete(card)
            && self.verdict(format, card).is_some_and(Legality::playable)
            && self.facts.blocked_at(card.get(), now).is_none()
    }

    /// `rows` held to what a ranked host of `format` checks at `now` (RFC
    /// 3339 UTC).
    #[must_use]
    pub fn check(&self, format: Format, rows: &Rows, now: &str) -> Verdict {
        let mut complaints = Vec::new();
        let main = plain_rows(&rows.main, &mut complaints);
        let sideboard = plain_rows(&rows.sideboard, &mut complaints);
        for name in &rows.commanders {
            if baylee_cards::decks::by_name(name).is_none() {
                complaints.push(unknown(name));
            }
        }
        if !complaints.is_empty() {
            return Verdict::refused(complaints);
        }
        let registration = match baylee_cards::ranked::registration(
            format.as_str(),
            &main,
            &sideboard,
            &rows.commanders,
        ) {
            Ok(registration) => registration,
            Err(why) => {
                return Verdict::refused(vec![format!("the deck cannot be registered: {why}")]);
            }
        };
        let deck = &registration.deck;
        let mut seen = BTreeSet::new();
        for card in deck
            .main
            .iter()
            .chain(&deck.sideboard)
            .chain(&deck.commanders)
        {
            if !seen.insert(card.index) {
                continue;
            }
            if let Some(block) = self.facts.blocked_at(card.index.get(), now) {
                let until = block
                    .until_said()
                    .map_or_else(String::new, |at| format!(" until {at}"));
                complaints.push(format!(
                    "«{}» is blocked on {}{until}: {}",
                    name_of(card.index),
                    self.facts.gateway,
                    block.reason
                ));
            }
            if !complete(card.index) {
                complaints.push(format!(
                    "«{}» is not fully scripted in this build, and Ranked takes only cards the \
                     engine plays completely",
                    name_of(card.index)
                ));
            }
        }
        complaints.extend(self.worded(format, deck));
        let admitted = baylee_cards::ranked::admit(format.as_str(), deck, &self.table);
        if let Err(why) = &admitted
            && complaints.is_empty()
        {
            complaints.push(format!("the host refuses it: {why}"));
        }
        if !complaints.is_empty() {
            return Verdict::refused(complaints);
        }
        match Frozen::from_rows(
            &rows.name,
            format.as_str(),
            format,
            &main,
            &sideboard,
            &rows.commanders,
        ) {
            Ok(frozen) => Verdict {
                complaints,
                frozen: Some(frozen),
            },
            Err(why) => Verdict::refused(vec![format!("the runner could not freeze it: {why:#}")]),
        }
    }

    /// Every fault `admit` finds in `deck`, by card, in words.
    fn worded(&self, format: Format, deck: &baylee_cards::decks::LoadedDeck) -> Vec<String> {
        let key = catalog_key(format);
        let report = formats::validate_loaded(
            deck,
            Rules {
                format: key,
                facts: Some(&self.table),
                list: None,
            },
        );
        let mut problems: Vec<Problem> = report.problems.clone();
        if format == Format::Highlander {
            // Vintage's own shape is sixty cards and four copies; Baylee
            // Highlander asks for its singleton shape instead, which
            // `admit` checks over the deck and its sideboard. Gladiator is
            // that shape under its own key, asked of no table.
            let cards = |list: &[baylee_cards::decks::DeckCard]| -> Vec<CardIndex> {
                list.iter().map(|c| c.index).collect()
            };
            let shape = formats::validate(
                formats::DeckCards {
                    main: &cards(&deck.main),
                    side: &cards(&deck.sideboard),
                    leaders: &cards(&deck.commanders),
                },
                Rules {
                    format: "gladiator",
                    facts: None,
                    list: None,
                },
            );
            problems.retain(|problem| !is_shape(*problem));
            problems.extend(shape.problems);
        }
        let mut out: Vec<String> = problems.iter().map(|p| word(*p, format)).collect();
        if report.unchecked.statuses {
            out.push(format!(
                "the legality facts from {} hold no verdicts for {key}: fetch them again",
                self.facts.gateway
            ));
        }
        for card in &report.unchecked.cards {
            out.push(format!(
                "the host's catalog has no {key} verdict for «{}», and Ranked takes no card \
                 without one",
                name_of(CardIndex::new(*card))
            ));
        }
        if format == Format::Commander {
            let allowed = deck
                .commanders
                .iter()
                .filter_map(|c| baylee_cards::print_rules::identity(c.index, c.choices))
                .fold(
                    baylee_core::color::ColorSet::EMPTY,
                    baylee_core::color::ColorSet::union,
                );
            for card in &deck.main {
                if !card.choices.colors().difference(allowed).is_empty() {
                    out.push(format!(
                        "«{}» has circled colours outside the commanders' identity",
                        name_of(card.index)
                    ));
                }
            }
        }
        out.dedup();
        out
    }
}

/// Whether `problem` is about a deck's shape rather than a card's verdict.
const fn is_shape(problem: Problem) -> bool {
    matches!(
        problem,
        Problem::TooFew { .. }
            | Problem::WrongSize { .. }
            | Problem::TooManyCopies { .. }
            | Problem::SideboardTooBig { .. }
            | Problem::Leaders { .. }
            | Problem::NotALeader { .. }
            | Problem::CannotLeadTogether { .. }
            | Problem::OutsideIdentity { .. }
            | Problem::LeadersNotTaken
    )
}

/// `problem` in a sentence, its card named.
fn word(problem: Problem, format: Format) -> String {
    let card = |n: u32| name_of(CardIndex::new(n));
    let key = catalog_key(format);
    match problem {
        Problem::UnknownCard { card: n } => format!("card #{n} is no card this build knows"),
        Problem::TooFew { have, need } => {
            format!("the main deck holds {have} cards, and needs at least {need}")
        }
        Problem::WrongSize { have, need } => format!(
            "the deck holds {have} cards{}, and must hold exactly {need}",
            if format == Format::Commander {
                " with its commanders"
            } else {
                ""
            }
        ),
        Problem::TooManyCopies {
            card: n,
            have,
            limit,
        } => format!(
            "«{}» is in the deck {have} times, and at most {limit} copies may be (basic lands \
             aside)",
            card(n)
        ),
        Problem::SideboardTooBig { have, limit } => {
            format!("the sideboard holds {have} cards, and may hold at most {limit}")
        }
        Problem::Leaders { have } => {
            format!(
                "a Commander deck has one commander, or two that may lead together; this one has {have}"
            )
        }
        Problem::NotALeader { card: n } => format!("«{}» may not be a commander", card(n)),
        Problem::CannotLeadTogether { card: a, with: b } => {
            format!("«{}» and «{}» may not lead together", card(a), card(b))
        }
        Problem::OutsideIdentity { card: n } => {
            format!("«{}» is outside the commanders' colour identity", card(n))
        }
        Problem::LeadersNotTaken => "only a Commander deck has commanders".to_string(),
        Problem::Banned { card: n } => format!("«{}» is banned in {key}", card(n)),
        Problem::Restricted { card: n, have } => format!(
            "«{}» is restricted in {key}: one copy, and the deck holds {have}",
            card(n)
        ),
        Problem::NotLegal { card: n } => format!("«{}» is not legal in {key}", card(n)),
        Problem::ListBanned { card: n } | Problem::ListBannedAsCommander { card: n } => {
            format!("«{}» is banned by the list", card(n))
        }
        Problem::ListRestricted { card: n, have } => {
            format!(
                "«{}» is restricted by the list, and the deck holds {have}",
                card(n)
            )
        }
    }
}

/// A name that is no card the engine plays, said as what it is: a real
/// card this build has no script for, or no card at all.
fn unknown(name: &str) -> String {
    if baylee_cards_index::row_by_name(name).is_some() {
        format!("«{name}» is a real card, but not one this build plays")
    } else {
        format!("«{name}» is no card this build knows (names are exact, in English)")
    }
}

/// `rows` as plain `N Name` rows (a printing, language, finish and note
/// dropped: the host keeps none of them for a deck it is not shown a
/// printing of), each name a card the engine plays; a fault of each is
/// pushed to `complaints`.
fn plain_rows(rows: &[String], complaints: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(rows.len());
    for line in rows {
        let row = match baylee_core::deckrow::parse(line) {
            Ok(row) => row,
            Err(e) => {
                complaints.push(format!("the row «{line}» cannot be read: {e}"));
                continue;
            }
        };
        if baylee_cards::decks::by_name(&row.name).is_none() {
            complaints.push(unknown(&row.name));
            continue;
        }
        let mut plain = baylee_core::deckrow::Row::plain(row.count, row.name);
        plain.print.choices = row.print.choices;
        out.push(plain.to_string());
    }
    out
}
