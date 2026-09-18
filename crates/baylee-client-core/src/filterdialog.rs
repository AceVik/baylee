//! A filter string, taken apart into the controls a dialog draws — and put
//! back together again.
//!
//! The owner asked for both directions at once: a gear button inside the
//! search box opens a panel where the query can be set *"peniebelst fein"*,
//! and *"aber auch in die andere Richtung, der bestehende Filter String wird
//! in die UI Elemente zerlegt"*. Those are one problem, not two. A dialog
//! that decomposes a string and recomposes it differently is a dialog that
//! silently rewrites what the player typed the moment they open it — and the
//! first thing anyone does with a builder is open it to look.
//!
//! So the whole module is built around one property, and
//! `tests::a_form_puts_back_exactly_what_it_took_apart` asserts it over every
//! query in the language's own corpus — written as a name and not as a link,
//! because the module it names only exists in a test build:
//!
//! ```text
//! FilterForm::of(&query).query() == query
//! ```
//!
//! Exactly equal, for **every** query and not only the ones the controls
//! understand. That is what [`FilterPart`] buys: a top-level part the dialog
//! has no control for is not dropped, not normalised and not pushed to the
//! end — it keeps its place in the row of parts and is carried whole. A
//! dialog opened on `t:creature (c:r or c:g) mv:3` draws two controls with an
//! untouchable chip between them, and closing it without touching anything
//! gives back the string that was there.
//!
//! What this module is **not** is a second reading of the language. It moves
//! [`Term`]s around; it never looks inside one. Deciding what `c:rg` means is
//! [`crate::cardquery`]'s job and is done once.

use crate::cardquery::{Key, Op, Query, Surface, Term, Value};

/// One thing standing in the row of controls a dialog draws.
///
/// The two cases are not "understood" and "broken": they are *a term that has
/// an on/off reading* and *a shape that does not*. `t:creature` may be drawn
/// as a control because everything beside it must also hold, so ticking it
/// off means something. `(c:r or c:g)` has no such reading — `a or b` minus
/// `a` is `b`, a different question — so it is carried as it stands and the
/// dialog offers to delete it whole or nothing at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterPart {
    /// A term the dialog draws a control for, and the minus in front of it.
    Row {
        /// Whether the player wrote a `-` before it.
        negated: bool,
        /// The term itself, untouched.
        term: Term,
    },
    /// A branch no control stands for, carried verbatim and put back in place.
    Opaque(Query),
}

/// What kind of control a key wants.
///
/// A dialog reads this rather than matching on [`Key`] itself, so that adding
/// a key to the language does not mean finding every `match` in the renderer:
/// a new key with no control here is drawn as a plain text box, which is what
/// a player would have typed into anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// A text box: a name, a word, a type, a line of rules text.
    Text,
    /// Five colour pips, and the three rules a colon, an `=` and a `<=` mean.
    Colors,
    /// A comparison and a number.
    Number,
    /// A tri-state tick: on, off, or not asked about.
    Flag,
    /// A mana cost, typed in `{1}{W}` notation.
    Cost,
}

impl Control {
    /// The control a key is drawn with.
    #[must_use]
    pub const fn of(key: &Key) -> Self {
        match key {
            Key::Color | Key::Identity => Self::Colors,
            Key::ManaValue | Key::Power | Key::Toughness | Key::Loyalty => Self::Number,
            Key::Is => Self::Flag,
            Key::Mana => Self::Cost,
            Key::Loose | Key::Name | Key::ExactName | Key::Oracle | Key::Type | Key::Unknown(_) => {
                Self::Text
            }
        }
    }
}

/// A filter string as a dialog holds it.
///
/// A list and not a struct of named fields, which is the decision the whole
/// module rests on. Named fields would mean one `name`, one `mv`, one colour
/// rule — and a player may write `t:creature t:goblin`, or `mv>=2 mv<=4`,
/// both of which are ordinary searches. Collapsing those into a single field
/// would lose one of them on the way in and re-emit a different query on the
/// way out, which is exactly the silent rewrite this module exists to avoid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterForm {
    /// The top-level parts, in the order they were written.
    pub parts: Vec<FilterPart>,
}

impl FilterForm {
    /// Takes a query apart into what a dialog draws.
    #[must_use]
    pub fn of(query: &Query) -> Self {
        let parts = match query {
            // An empty box opens an empty dialog, not a dialog with one blank
            // row in it: a row stands for a term, and there is no term.
            Query::Anything => Vec::new(),
            Query::All(parts) => parts.iter().map(Self::part_of).collect(),
            other => vec![Self::part_of(other)],
        };
        Self { parts }
    }

    fn part_of(query: &Query) -> FilterPart {
        match query {
            Query::Term(term) => FilterPart::Row {
                negated: false,
                term: term.clone(),
            },
            Query::Not(inner) => match &**inner {
                Query::Term(term) => FilterPart::Row {
                    negated: true,
                    term: term.clone(),
                },
                _ => FilterPart::Opaque(query.clone()),
            },
            _ => FilterPart::Opaque(query.clone()),
        }
    }

    /// The query these controls stand for.
    ///
    /// Built through the same `All` the parser builds, so the result is in the
    /// language's one canonical shape and compares equal to the query it came
    /// from. A form with nothing in it is [`Query::Anything`], which is the
    /// empty box — not an `All` of nothing, which would render as a stray
    /// pair of brackets.
    #[must_use]
    pub fn query(&self) -> Query {
        let parts: Vec<Query> = self.parts.iter().map(FilterPart::query).collect();
        crate::cardquery::all_of(parts)
    }

    /// Where the first row for this key sits, if a row for it is drawn.
    ///
    /// A dialog binding a dedicated control — the five colour pips, the mana
    /// value spinner — asks this to find out whether it is showing an
    /// existing term or offering a new one.
    #[must_use]
    pub fn row_of(&self, key: &Key) -> Option<usize> {
        self.parts.iter().position(|part| match part {
            FilterPart::Row { term, .. } => &term.key == key,
            FilterPart::Opaque(_) => false,
        })
    }

    /// Writes a term for this key: over the first row that has one, or a new
    /// row at the end.
    ///
    /// The first and not every one, for the same reason `t:creature t:goblin`
    /// is two rows: a second term with the same key is a second condition the
    /// player wrote on purpose, and a control that overwrote both would be
    /// deleting one of them.
    pub fn set(&mut self, key: &Key, op: Op, value: Value, negated: bool) {
        let row = FilterPart::Row {
            negated,
            term: Term {
                key: key.clone(),
                op,
                value,
            },
        };
        match self.row_of(key) {
            Some(at) => self.parts[at] = row,
            None => self.parts.push(row),
        }
    }

    /// Takes the first row for this key out, if there is one.
    ///
    /// Returns whether anything was removed, so a caller can tell "the tick
    /// was already off" from "the tick has just been cleared".
    pub fn clear(&mut self, key: &Key) -> bool {
        match self.row_of(key) {
            Some(at) => {
                self.parts.remove(at);
                true
            }
            None => false,
        }
    }

    /// The parts this surface cannot answer, by index.
    ///
    /// Not a filter and not a refusal: the row is still drawn and the term is
    /// still carried, because a player may be editing a line they will paste
    /// somewhere else. What it is for is the warning beside the control —
    /// a term this surface cannot answer hides every row, which reads exactly
    /// like a search that found nothing, and this is the one place the dialog
    /// can say so before it happens.
    #[must_use]
    pub fn unanswerable(&self, surface: Surface) -> Vec<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, part)| match part {
                FilterPart::Row { term, .. } => !surface.answers(&term.key),
                // A branch is unanswerable only if nothing in it can be
                // answered, which is `Match::Unknown`'s own rule: one side of
                // an `or` is enough.
                FilterPart::Opaque(query) => {
                    !query.terms().iter().any(|term| surface.answers(&term.key))
                }
            })
            .map(|(at, _)| at)
            .collect()
    }
}

impl FilterPart {
    /// The query this one part stands for.
    #[must_use]
    pub fn query(&self) -> Query {
        match self {
            Self::Row { negated, term } => {
                let term = Query::Term(term.clone());
                if *negated {
                    Query::Not(Box::new(term))
                } else {
                    term
                }
            }
            Self::Opaque(query) => query.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
