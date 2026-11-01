//! A report as a model's answer carries it (`docs/llm-protocol.md`
//! §"Reports and lookups"): its kind, severity and confidence, the cards it
//! names with the words of their text it quotes, the rule numbers it cites,
//! what it saw and what it expected.
//!
//! Read leniently and held to bounds, never refused for its shape: a kind
//! the reader does not know is `other`, a severity or a confidence it does
//! not know is not said, a field too long is cut, the cards and the rule
//! numbers past [`MOST_CARDS`] and [`MOST_RULES`] are dropped. The shape of
//! before 01.11.2026 is still read: `what` as `observed`, `evidence` kept as
//! it was, `references` as they are.

use super::checks::cut;
use super::{FIELD_CHARS, MOST_REFERENCES, REFERENCE_CHARS};
use serde_json::Value;

/// The most cards a report keeps.
pub const MOST_CARDS: usize = 8;

/// The longest a card's name is kept, in characters.
pub const NAME_CHARS: usize = 100;

/// The longest a quote of a card's text is kept, in characters.
pub const QUOTE_CHARS: usize = 300;

/// The most rule numbers a report keeps.
pub const MOST_RULES: usize = 8;

/// The longest a rule number is kept, in characters: far more than any
/// is long, so one that is cut is no rule number.
pub const RULE_CHARS: usize = 16;

/// What kind of error a report says the table made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// The table broke a rule.
    RulesError,
    /// A card did not do what its text says.
    CardTextMismatch,
    /// A legal choice was not offered.
    MissingOption,
    /// A choice the rules do not allow was offered.
    IllegalOption,
    /// A result is wrong: damage, life, a zone, a count.
    WrongOutcome,
    /// The text the model was shown is wrong.
    DisplayText,
    /// Anything else, and a kind the reader does not know.
    #[default]
    Other,
    /// Set by the seat, never read from a model: the report quotes no
    /// card's text and cites no rule number ([`super::Checked`]).
    Unverified,
}

impl Kind {
    /// The kinds a model may give, in the order the schema lists them.
    pub const MODEL: [Self; 7] = [
        Self::RulesError,
        Self::CardTextMismatch,
        Self::MissingOption,
        Self::IllegalOption,
        Self::WrongOutcome,
        Self::DisplayText,
        Self::Other,
    ];

    /// Its name, as the schema and the bug report spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RulesError => "rules_error",
            Self::CardTextMismatch => "card_text_mismatch",
            Self::MissingOption => "missing_option",
            Self::IllegalOption => "illegal_option",
            Self::WrongOutcome => "wrong_outcome",
            Self::DisplayText => "display_text",
            Self::Other => "other",
            Self::Unverified => "unverified",
        }
    }

    /// A model's kind: one of [`Self::MODEL`] by name, else `other`.
    fn read(value: Option<&Value>) -> Self {
        let written = value.and_then(Value::as_str).map(enum_word);
        Self::MODEL
            .into_iter()
            .find(|kind| written.as_deref() == Some(kind.as_str()))
            .unwrap_or(Self::Other)
    }
}

/// How much an error a report says the table made changes the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// It changes who wins, or could.
    GameChanging,
    /// It changes the game a little.
    Minor,
    /// It changes nothing but what is shown.
    Cosmetic,
}

impl Severity {
    /// The severities, in the order the schema lists them.
    pub const ALL: [Self; 3] = [Self::GameChanging, Self::Minor, Self::Cosmetic];

    /// Its name, as the schema and the bug report spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GameChanging => "game_changing",
            Self::Minor => "minor",
            Self::Cosmetic => "cosmetic",
        }
    }
}

/// How sure the model says it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// It may well be wrong.
    Low,
    /// Likely.
    Medium,
    /// Sure.
    High,
}

impl Confidence {
    /// The confidences, in the order the schema lists them.
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    /// Its name, as the schema and the bug report spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// A card a report names, and the words of its text it quotes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Card {
    /// Its name, as the model wrote it, brackets taken off: at most
    /// [`NAME_CHARS`].
    pub name: String,
    /// The words of its Oracle text the model quotes, quotation marks
    /// around them taken off: at most [`QUOTE_CHARS`].
    pub oracle_quote: Option<String>,
}

/// A rules or engine error a model noticed, as its answer said it: the
/// model's claim, untrusted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// What kind of error it says it is.
    pub kind: Kind,
    /// How much it says the error changes the game, where it says so.
    pub severity: Option<Severity>,
    /// How sure it says it is, where it says so.
    pub confidence: Option<Confidence>,
    /// The cards it names, at most [`MOST_CARDS`].
    pub cards: Vec<Card>,
    /// The Comprehensive Rules numbers it cites (`702.19b`), as written
    /// less a leading `CR`, at most [`MOST_RULES`].
    pub rules: Vec<String>,
    /// What happened.
    pub observed: String,
    /// What the rules or the card's text say should happen.
    pub expected: Option<String>,
    /// What an answer of the old shape said it relied on.
    pub evidence: Option<String>,
    /// Scryfall addresses (the opt-in lookup), and whatever an answer of
    /// the old shape referred to.
    pub references: Vec<String>,
}

impl Report {
    /// The `report` of an answer: an object, or a plain string as its
    /// `observed`. `None` when it says nothing (no `observed` nor `what`, a
    /// blank one, a strict schema's `null`).
    #[must_use]
    pub fn read(value: &Value) -> Option<Self> {
        let report = match value {
            Value::String(_) => Self {
                observed: text(Some(value), FIELD_CHARS)?,
                ..Self::default()
            },
            Value::Object(fields) => Self {
                kind: Kind::read(fields.get("kind")),
                severity: one_of(fields.get("severity"), Severity::ALL, Severity::as_str),
                confidence: one_of(
                    fields.get("confidence"),
                    Confidence::ALL,
                    Confidence::as_str,
                ),
                cards: listed(fields.get("cards"))
                    .filter_map(card)
                    .take(MOST_CARDS)
                    .collect(),
                rules: rules(fields.get("rules")),
                observed: text(fields.get("observed"), FIELD_CHARS)
                    .or_else(|| text(fields.get("what"), FIELD_CHARS))?,
                expected: text(fields.get("expected"), FIELD_CHARS),
                evidence: text(fields.get("evidence"), FIELD_CHARS),
                references: listed(fields.get("references"))
                    .filter_map(|item| text(Some(item), REFERENCE_CHARS))
                    .take(MOST_REFERENCES)
                    .collect(),
            },
            _ => return None,
        };
        Some(report)
    }

    /// Everything it says, cards in brackets, for finding the cards it
    /// names.
    pub(super) fn words(&self) -> String {
        let mut words = self.observed.clone();
        for part in [&self.expected, &self.evidence].into_iter().flatten() {
            words.push('\n');
            words.push_str(part);
        }
        for card in &self.cards {
            words.push_str("\n[");
            words.push_str(&card.name);
            words.push(']');
        }
        for reference in &self.references {
            words.push('\n');
            words.push_str(reference);
        }
        words
    }
}

/// The items of a list, or a lone item where a list was asked for.
fn listed(value: Option<&Value>) -> impl Iterator<Item = &Value> {
    let items: &[Value] = match value {
        Some(Value::Array(items)) => items,
        Some(Value::Null) | None => &[],
        Some(one) => std::slice::from_ref(one),
    };
    items.iter()
}

/// A string, trimmed and held to `most` characters, its control characters
/// but a line break and a tab left out; `None` for a blank one.
fn text(value: Option<&Value>, most: usize) -> Option<String> {
    let text: String = value
        .and_then(Value::as_str)?
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| held(text, most))
}

/// A card: `{"name", "oracle_quote"}`, or a plain string as its name.
fn card(value: &Value) -> Option<Card> {
    let (name, quote) = match value {
        Value::Object(fields) => (fields.get("name"), fields.get("oracle_quote")),
        other => (Some(other), None),
    };
    let name = text(name, NAME_CHARS + 2)?;
    let name = name
        .strip_prefix('[')
        .and_then(|bare| bare.strip_suffix(']'))
        .unwrap_or(&name)
        .trim();
    let quote = text(quote, QUOTE_CHARS + 2).map(|quote| {
        let bare = quote.trim_matches(|c| matches!(c, '"' | '“' | '”')).trim();
        cut(bare, QUOTE_CHARS)
    });
    (!name.is_empty()).then(|| Card {
        name: cut(name, NAME_CHARS),
        oracle_quote: quote.filter(|quote| !quote.is_empty()),
    })
}

/// The rule numbers of `value`: each trimmed, a leading `CR` and a trailing
/// full stop taken off, lower case, each once, at most [`MOST_RULES`]. A
/// number in JSON is read as written.
fn rules(value: Option<&Value>) -> Vec<String> {
    let mut rules: Vec<String> = Vec::new();
    for item in listed(value) {
        let written = match item {
            Value::Number(n) => n.to_string(),
            other => match other.as_str() {
                Some(text) => text.to_string(),
                None => continue,
            },
        };
        let trimmed = written.trim();
        let bare = trimmed
            .get(..2)
            .filter(|head| head.eq_ignore_ascii_case("cr"))
            .map_or(trimmed, |_| &trimmed[2..])
            .trim()
            .trim_end_matches('.')
            .to_lowercase();
        let rule = cut(&bare, RULE_CHARS);
        if !rule.is_empty() && !rules.contains(&rule) {
            rules.push(rule);
        }
        if rules.len() == MOST_RULES {
            break;
        }
    }
    rules
}

/// One of `all` by its name (`as_str`), case, spaces and hyphens aside.
fn one_of<T: Copy>(
    value: Option<&Value>,
    all: [T; 3],
    name: impl Fn(T) -> &'static str,
) -> Option<T> {
    let written = enum_word(value?.as_str()?);
    all.into_iter().find(|each| name(*each) == written)
}

/// An enum's value as written, spelt as the schema spells them: lower
/// case, words joined by `_`.
fn enum_word(written: &str) -> String {
    written
        .trim()
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// `text`, at most `most` characters of it, an ellipsis saying it was cut.
/// A name and a quote are [`cut`] without one: what is left of a quote is
/// still words of the quote.
fn held(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}
