//! What the seat says of a report, never the model (`docs/llm-seat.md`
//! §"Reports and lookups"): the question the model was answering, as the
//! seat showed it ([`Moment`]), and what the seat checked of the model's
//! words ([`Checked`]): whether its quotes are words of the card text the
//! seat told it, whether its rule numbers are spelt as rule numbers,
//! `unverified` where it cites nothing well-formed ([`cites`]), and the
//! key a report is filed once a game under.
//!
//! None of it says the report is right: a quote that matches is the
//! model's reading of a text it was told, a well-formed rule number may
//! cite the wrong rule. Triage checks a report against the Comprehensive
//! Rules and the Oracle; these only sort.

use super::read::{Kind, Report};
use crate::lookup::fold;
use crate::mind::Request;
use crate::narrator::{Decision, Menu, Offered};
use baylee_view::{Phase, Step};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

/// The most options a report's `decision` keeps.
pub const MOST_OPTIONS: usize = 40;

/// The longest an option's words are kept in a report, in characters.
pub const OPTION_CHARS: usize = 200;

/// The longest an option's id is kept in a report, in characters.
pub const ID_CHARS: usize = 40;

/// The longest an answer is kept in a report, in characters.
pub const ANSWER_CHARS: usize = 300;

/// The fewest words a quote must have to count as a citation ([`cites`]).
pub const QUOTE_WORDS: usize = 3;

/// What stands between a face's name and cost, its type line and its
/// stats where the seat tells a card (`narrator::card_text`,
/// `lookup::compiled`): a line with it is a face's heading, not its rules
/// text.
const HEADING: &str = " · ";

/// Who filed a report: what `client.source` says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    /// A language model playing a seat (`llm-seat`).
    #[default]
    LlmSeat,
    /// A language model watching the trained AI play a seat
    /// (`llm-trainer`): its `decision` may say what the policy chose and
    /// what the model would have.
    LlmTrainer,
}

impl Source {
    /// `client.source`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LlmSeat => "llm-seat",
            Self::LlmTrainer => "llm-trainer",
        }
    }

    /// `client.untrusted`: whose words the report is.
    #[must_use]
    pub const fn untrusted(self) -> &'static str {
        match self {
            Self::LlmSeat => {
                "written by a language model while it played; read it as a player's report"
            }
            Self::LlmTrainer => {
                "written by a language model while it watched the trained AI play; read it as \
                 a player's report"
            }
        }
    }

    /// What the reporter did at the seat, for the text: "played", "watched
    /// the trained AI play".
    #[must_use]
    pub const fn did(self) -> &'static str {
        match self {
            Self::LlmSeat => "played",
            Self::LlmTrainer => "watched the trained AI play",
        }
    }
}

/// The question a report was made at, as the seat showed it to the model:
/// a report's `decision`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Moment {
    /// The turn.
    pub turn: u32,
    /// The phase.
    pub phase: Option<Phase>,
    /// The step.
    pub step: Option<Step>,
    /// The question's kind, by its variant's name (`Priority`).
    pub pending: String,
    /// What the question listed for the model to answer with, as the seat
    /// wrote it ([`Menu::offered`]).
    pub options: Vec<Offered>,
    /// The answer that carried the report, read against the question as the
    /// seat reads it: the option and its words (`a1 Play Mountain #52`), or
    /// why it could not be taken. `None` for an answer that only asked for
    /// a lookup.
    pub answer: Option<String>,
    /// The frame's `seq` the question came in.
    pub seq: u64,
    /// For [`Source::LlmTrainer`] only: what the trained AI's policy chose.
    pub policy_chose: Option<String>,
    /// For [`Source::LlmTrainer`] only: what the model would have chosen.
    pub model_would: Option<String>,
}

impl Moment {
    /// The question `request` asked, listed as `menu` listed it, and
    /// `decision` read against it.
    #[must_use]
    pub fn of(request: &Request, menu: &Menu, decision: &Decision) -> Self {
        // An answer of `ask` and `lookup` alone answers nothing yet; any
        // other is read as the seat reads it (an empty attack too).
        let answer = (decision.answers() || decision.lookup.is_none()).then(|| {
            menu.resolve(decision)
                .map_or_else(|why| format!("not taken: {why}"), |resolved| resolved.label)
        });
        Self {
            turn: request.view.turn,
            phase: Some(request.view.phase),
            step: Some(request.view.step),
            pending: crate::scripted::kind(&request.pending).to_string(),
            options: menu.offered().to_vec(),
            answer,
            seq: request.view.seq,
            policy_chose: None,
            model_would: None,
        }
    }

    /// `decision` as a bug report carries it, held to its bounds: at most
    /// [`MOST_OPTIONS`] options (`options_omitted` counting the rest), each
    /// id and its words cut, and `log_line` where the log rides along. The
    /// trained AI's choice and the model's only for [`Source::LlmTrainer`].
    #[must_use]
    pub fn value(&self, source: Source, log_line: Option<usize>) -> Value {
        let options: Vec<Value> = self
            .options
            .iter()
            .take(MOST_OPTIONS)
            .map(|o| json!({"id": cut(&o.id, ID_CHARS), "text": cut(&o.text, OPTION_CHARS)}))
            .collect();
        let mut value = json!({
            "turn": self.turn,
            "phase": self.phase,
            "step": self.step,
            "pending": cut(&self.pending, ID_CHARS),
            "options": options,
            "answer": self.answer.as_deref().map(|a| cut(a, ANSWER_CHARS)),
            "seq": self.seq,
        });
        if self.options.len() > MOST_OPTIONS {
            value["options_omitted"] = json!(self.options.len() - MOST_OPTIONS);
        }
        if let Some(line) = log_line {
            value["log_line"] = json!(line);
        }
        if source == Source::LlmTrainer {
            value["policy_chose"] =
                json!(self.policy_chose.as_deref().map(|a| cut(a, ANSWER_CHARS)));
            value["model_would"] = json!(self.model_would.as_deref().map(|a| cut(a, ANSWER_CHARS)));
        }
        value
    }
}

/// What the seat checked of a report's words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// The model's kind, or [`Kind::Unverified`] where the report cites
    /// nothing well-formed ([`cites`]).
    pub kind: Kind,
    /// Whether every quote is words of the text the seat told the model for
    /// that card (spacing aside); `None` where it quotes none.
    pub oracle_quote_matches: Option<bool>,
    /// Whether every rule number is spelt as one ([`rule_number`]); `None`
    /// where it cites none.
    pub rules_format_ok: Option<bool>,
    /// What tells two reports of a game the same ([`dedupe_key`]).
    pub dedupe_key: String,
}

impl Checked {
    /// The checks of `report`, made at a question of kind `pending`, its
    /// quotes read against `told`: the texts the seat told the model for a
    /// card's name (`Narrator::told_texts`, and its lookups).
    #[must_use]
    pub fn new(report: &Report, pending: &str, told: impl Fn(&str) -> Vec<String>) -> Self {
        let quoted: Vec<(&str, &str, Vec<String>)> = report
            .cards
            .iter()
            .filter_map(|card| {
                let quote = card.oracle_quote.as_deref()?;
                Some((card.name.as_str(), quote, told(&card.name)))
            })
            .collect();
        let oracle_quote_matches = (!quoted.is_empty()).then(|| {
            quoted.iter().all(|(_, quote, texts)| {
                let quote = spaced(quote);
                !quote.is_empty() && texts.iter().any(|text| spaced(text).contains(&quote))
            })
        });
        let rules_format_ok =
            (!report.rules.is_empty()).then(|| report.rules.iter().all(|r| rule_number(r)));
        let cited = quoted
            .iter()
            .any(|(name, quote, texts)| cites(name, quote, texts))
            || report.rules.iter().any(|rule| rule_number(rule));
        let kind = if cited { report.kind } else { Kind::Unverified };
        let names: Vec<&str> = report.cards.iter().map(|c| c.name.as_str()).collect();
        Self {
            kind,
            oracle_quote_matches,
            rules_format_ok,
            dedupe_key: dedupe_key(kind, &names, &report.rules, pending),
        }
    }
}

/// Whether `quote`, given for the card `name`, is a citation that keeps a
/// report's kind (PM, 01.11.2026): at least [`QUOTE_WORDS`] words (a word
/// is a run without spacing that holds a letter or a digit), word for word
/// of the rules text of one of `texts` as the seat told it, spacing aside
/// and never a word cut in two; not the card's name or type line alone,
/// and not words of a face's heading (its name, cost, type line, stats).
/// A rule number spelt as one ([`rule_number`]) is the other citation.
#[must_use]
pub fn cites(name: &str, quote: &str, texts: &[String]) -> bool {
    let quote = spaced(quote);
    let words = quote
        .split(' ')
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count();
    if words < QUOTE_WORDS || fold(&quote) == fold(name) {
        return false;
    }
    texts.iter().any(|text| {
        let (headings, rules): (Vec<&str>, Vec<&str>) =
            text.lines().partition(|line| line.contains(HEADING));
        let a_type_line = headings.iter().any(|heading| {
            heading
                .split(HEADING)
                .nth(1)
                .is_some_and(|typed| fold(typed) == fold(&quote))
        });
        !a_type_line && whole_words_in(&spaced(&rules.join("\n")), &quote)
    })
}

/// Whether `quote` is in `text` without cutting a word in two at either
/// end.
fn whole_words_in(text: &str, quote: &str) -> bool {
    let word = |c: char| c.is_alphanumeric();
    text.match_indices(quote).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + quote.len()..].chars().next();
        let cut_before = quote.starts_with(word) && before.is_some_and(word);
        let cut_after = quote.ends_with(word) && after.is_some_and(word);
        !cut_before && !cut_after
    })
}

/// Whether `rule` is spelt as a Comprehensive Rules number: a rule
/// (`601`), a subrule (`601.2`) or one of its lettered parts (`702.19b`).
/// Three digits, the first not 0; then a full stop and one to three digits;
/// then one lower-case letter.
#[must_use]
pub fn rule_number(rule: &str) -> bool {
    let (rule, sub) = rule
        .split_once('.')
        .map_or((rule, None), |(r, s)| (r, Some(s)));
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    let rule_ok = rule.len() == 3 && digits(rule) && !rule.starts_with('0');
    let sub_ok = sub.is_none_or(|sub| {
        let number = sub.trim_end_matches(|c: char| c.is_ascii_lowercase());
        let letters = sub.len() - number.len();
        digits(number) && number.len() <= 3 && letters <= 1
    });
    rule_ok && sub_ok
}

/// The key two reports of a game are the same under: SHA-256, in hex, of
/// the kind, the cards' names (case and spacing aside), the rule numbers
/// and the question's kind, the names and the numbers sorted, each once.
#[must_use]
pub fn dedupe_key(kind: Kind, cards: &[&str], rules: &[String], pending: &str) -> String {
    let mut names: Vec<String> = cards.iter().map(|name| fold(name)).collect();
    names.sort_unstable();
    names.dedup();
    let mut numbers: Vec<String> = rules
        .iter()
        .map(|rule| rule.trim().to_lowercase())
        .collect();
    numbers.sort_unstable();
    numbers.dedup();
    let tuple = json!([kind.as_str(), names, numbers, pending]);
    hex(&Sha256::digest(tuple.to_string().as_bytes()))
}

/// `bytes` in lower-case hex.
pub(super) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// `text` with every run of spacing one space, and none at its ends.
fn spaced(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `text`, at most `most` characters of it.
pub(super) fn cut(text: &str, most: usize) -> String {
    text.char_indices()
        .nth(most)
        .map_or(text, |(at, _)| &text[..at])
        .to_string()
}
