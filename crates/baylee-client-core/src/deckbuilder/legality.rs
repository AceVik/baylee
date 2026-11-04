//! The deck's standing while it is built: the format and list it is held to,
//! what [`baylee_cards::formats::validate`] finds, and how the builder says so.
//!
//! The check runs after every edit and the answer is kept, because a row asks
//! it as it is drawn. It costs a few microseconds a card, so "after every
//! edit" is what is true and no debounce stands between a press and its
//! answer. Nothing here refuses anything: saving a deck is never refused for
//! being illegal (`docs/banlists-and-formats.md`), and the gateway's own
//! verdict is the one a table reads.

use super::{DeckBuilder, Zone};
use crate::deckcheck;
use crate::i18n::{Lang, Phrase};
use baylee_cards::formats::{self, DeckCards, FREEFORM, Rules, SHAPES};
use baylee_cardtext::{ListRules, Problem, Report, Standing};
use baylee_core::ids::CardIndex;
use std::sync::Arc;

/// The list a deck is being held to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChosenList {
    /// Its id at the gateway.
    pub id: String,
    /// What it is called.
    pub name: String,
    /// What it says, shared with whoever else reads it.
    pub rules: Arc<ListRules>,
}

/// One line of the overview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// What is wrong, as one sentence.
    pub text: String,
    /// The pool slot of the card it is about, which a press opens.
    pub slot: Option<usize>,
}

/// What the builder says about the deck, in one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overview {
    /// The deck's standing; [`Standing::Unknown`] is never drawn as legal.
    pub standing: Standing,
    /// Legal or not, and for what.
    pub headline: String,
    /// How many cards it holds and how many the format takes.
    pub count: String,
    /// Every problem, each pointing at its card where it has one.
    pub problems: Vec<Line>,
    /// What could not be checked, and the cards with no script.
    pub notices: Vec<String>,
}

impl DeckBuilder {
    /// The format the deck is held to: the one chosen, else what its leaders
    /// make it, as the gateway reads a deck that does not say.
    #[must_use]
    pub fn format(&self) -> &str {
        if self.format.is_empty() {
            if self.commanders.is_empty() {
                FREEFORM
            } else {
                "commander"
            }
        } else {
            &self.format
        }
    }

    /// Chooses the format. It is saved with the deck, so it is an edit.
    pub fn set_format(&mut self, format: &str) {
        if self.format != format {
            self.format = format.to_string();
            self.touch();
        }
    }

    /// Takes the format a stored deck says. Not an edit.
    pub fn keep_format(&mut self, format: &str) {
        self.format = format.to_string();
        self.recheck();
    }

    /// The formats a deck can be held to, in the order a picker offers them:
    /// none, then the ones a player reaches for first, then the rest. The
    /// gateway's table names them; without one, the shapes this build knows.
    #[must_use]
    pub fn format_choices(&self) -> Vec<String> {
        let mut keys: Vec<String> = match self.pool_facts.as_deref() {
            Some(table) if !table.is_empty() => table.formats().to_vec(),
            _ => SHAPES.iter().map(|(key, _)| (*key).to_string()).collect(),
        };
        keys.sort_by_key(|key| crate::filterdialog::format_rank(key));
        keys.insert(0, FREEFORM.to_string());
        keys
    }

    /// The list the deck is held to.
    #[must_use]
    pub const fn list(&self) -> Option<&ChosenList> {
        self.list.as_ref()
    }

    /// Holds the deck to a list, or to none. Not saved with the deck: a list
    /// is a question put to it, and the same deck meets another at the next
    /// table.
    pub fn set_list(&mut self, list: Option<ChosenList>) {
        self.list = list;
        self.recheck();
    }

    /// What the last check found.
    #[must_use]
    pub const fn report(&self) -> &Report {
        &self.report
    }

    /// Asks the question again. Every edit lands here.
    pub(super) fn recheck(&mut self) {
        let report = self.check();
        if report != self.report {
            self.report = report;
        }
    }

    /// The deck as a check reads it: a leader sits among the main rows here
    /// (`set_commander` puts it there) and is one card, not a card in two
    /// places, so one copy of each is taken out, as the gateway's
    /// `loaded_deck` does.
    fn cards(&self) -> (Vec<CardIndex>, Vec<CardIndex>, Vec<CardIndex>) {
        let copies = |zone: Zone| -> Vec<CardIndex> {
            self.entries(zone)
                .iter()
                .filter_map(|entry| {
                    self.pool
                        .get(entry.slot)
                        .map(|card| (CardIndex::new(card.index), entry.count))
                })
                .flat_map(|(card, count)| std::iter::repeat_n(card, usize::from(count)))
                .collect()
        };
        let mut main = copies(Zone::Main);
        let side = copies(Zone::Side);
        let leaders: Vec<CardIndex> = self
            .commanders
            .iter()
            .filter_map(|slot| self.pool.get(*slot))
            .map(|card| CardIndex::new(card.index))
            .collect();
        for leader in &leaders {
            if let Some(at) = main.iter().position(|card| card == leader) {
                main.swap_remove(at);
            }
        }
        (main, side, leaders)
    }

    fn check(&self) -> Report {
        let (main, side, leaders) = self.cards();
        formats::validate(
            DeckCards {
                main: &main,
                side: &side,
                leaders: &leaders,
            },
            Rules {
                format: self.format(),
                facts: self.pool_facts.as_deref(),
                list: self.list.as_ref().map(|list| &*list.rules),
            },
        )
    }

    /// Whether the deck can be called anything yet: rows waiting for the
    /// pool, or a card it lost, stand between it and every word but this.
    fn undecided(&self) -> bool {
        !self.pending.is_empty() || !self.missing.is_empty() || !self.loaded()
    }

    /// The deck's standing, which [`Self::undecided`] holds back from legal.
    #[must_use]
    pub fn standing(&self) -> Standing {
        match self.report.standing() {
            Standing::Legal if self.undecided() => Standing::Unknown,
            standing => standing,
        }
    }

    /// The word a deck row wears: the first thing wrong with the card it
    /// shows, or `None` for a row with nothing against it.
    #[must_use]
    pub fn badge(&self, slot: usize) -> Option<Phrase> {
        let card = self.pool.get(slot)?.index;
        self.report.of(card).find_map(deckcheck::badge)
    }

    /// What is wrong with one card, for the card window.
    #[must_use]
    pub fn card_problems(&self, slot: usize, lang: Lang) -> Vec<String> {
        let Some(card) = self.pool.get(slot) else {
            return Vec::new();
        };
        self.report
            .of(card.index)
            .map(|problem| self.sentence(problem, lang))
            .collect()
    }

    /// The pool slot of the card with this registry index.
    fn slot_of_index(&self, index: u32) -> Option<usize> {
        self.slots.get(&index).copied()
    }

    /// A problem as the player reads it, cards by the names the pool gives
    /// them in the client's language.
    fn sentence(&self, problem: Problem, lang: Lang) -> String {
        let name = |card: u32| {
            self.slot_of_index(card)
                .and_then(|slot| self.pool.get(slot))
                .map_or_else(|| deckcheck::english_name(card), |card| card.name.clone())
        };
        deckcheck::problem(
            problem,
            deckcheck::Context {
                format: self.format(),
                list: self.list.as_ref().map_or("", |list| list.name.as_str()),
                lang,
            },
            &name,
        )
    }

    /// How the deck's size reads against what the format takes.
    fn count_line(&self, lang: Lang) -> String {
        let have = self.counts().main;
        let have = have.to_string();
        match formats::shape_of(self.format()) {
            Some(formats::Shape::Constructed) => Phrase::CheckCountAtLeast
                .fill(lang, &[&have, &formats::CONSTRUCTED_MIN.to_string()]),
            Some(formats::Shape::Highlander | formats::Shape::Commander) => {
                Phrase::CheckCountExactly.fill(lang, &[&have, &formats::SINGLETON_SIZE.to_string()])
            }
            None => Phrase::CheckCount.fill(lang, &[&have]),
        }
    }

    /// What the builder says about the deck.
    #[must_use]
    pub fn overview(&self, lang: Lang) -> Overview {
        let format = self.format();
        let named = self.list.as_ref().map_or_else(
            || deckcheck::format_name(format, lang).to_string(),
            |list| list.name.clone(),
        );
        let standing = self.standing();
        let free = format == FREEFORM && self.list.is_none();
        let headline = match standing {
            Standing::Legal if free => Phrase::CheckFree.text(lang).to_string(),
            Standing::Legal => Phrase::CheckLegal.fill(lang, &[&named]),
            Standing::Illegal => {
                Phrase::CheckIllegal.fill_count(lang, self.report.problems.len(), &[&named])
            }
            Standing::Unknown => Phrase::CheckUnknown.fill(lang, &[&named]),
        };
        let problems = self
            .report
            .problems
            .iter()
            .map(|problem| Line {
                text: self.sentence(*problem, lang),
                slot: problem.card().and_then(|card| self.slot_of_index(card)),
            })
            .collect();
        let mut notices = Vec::new();
        let unchecked = &self.report.unchecked;
        if !self.loaded() {
            notices.push(Phrase::CheckWaitsForPool.text(lang).to_string());
        }
        if unchecked.statuses {
            notices.push(Phrase::CheckNoStatuses.text(lang).to_string());
        }
        if unchecked.shape {
            notices.push(Phrase::CheckNoShape.text(lang).to_string());
        }
        if !unchecked.cards.is_empty() {
            notices.push(Phrase::CheckCardsUnknown.fill_count(lang, unchecked.cards.len(), &[]));
        }
        if !self.report.unscripted.is_empty() {
            notices.push(Phrase::CheckUnscripted.fill_count(
                lang,
                self.report.unscripted.len(),
                &[],
            ));
        }
        Overview {
            standing,
            headline,
            count: self.count_line(lang),
            problems,
            notices,
        }
    }
}
