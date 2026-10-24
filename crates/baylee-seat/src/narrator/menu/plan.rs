//! A plan: the steps a model writes once with an answer, which the seat
//! answers the questions that follow with, one step a question, without a
//! call (`docs/llm-protocol.md` §"Plans").
//!
//! A step is one short string in a grammar small enough to state in the
//! schema ([`GRAMMAR`]) and to read without guessing: a step outside it
//! refuses the whole answer, with the reason, before anything is sent
//! ([`Step::parse`]). A step's ids are read when its question comes,
//! against that question's own offer, as a model's answer is
//! ([`Menu::plan_decision`]): a step that does not answer the question
//! exactly stops the plan, and the model is asked ([`Stop`]).

use super::{Ask, Decision, Menu, normal, object};
use crate::mind::Request;
use crate::narrator::Table;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use std::fmt;

/// The steps a plan may hold, as the schema and the system prompt state
/// them.
pub const GRAMMAR: &str = "play #id | cast #id [-> targets] | activate #id[:n] [-> targets] | \
choose <card name or #id>[, …] | yes | no | color W/U/B/R/G | mode m<n> | n <number> | \
attack none | attack #id@P<n> … | pass";

/// One step of a plan: what the seat answers the next question it is asked
/// with. Ids are kept as written and read against each question's offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Play a land from the hand.
    Play(String),
    /// Cast a spell, by tapping what it needs, and name its targets ahead.
    Cast {
        /// The card.
        card: String,
        /// Targets for the question the cast asks.
        targets: Vec<String>,
    },
    /// Activate an ability; `ability` is its number as the menu prints it,
    /// needed where the object offers more than one.
    Activate {
        /// The object.
        source: String,
        /// Its ability's number, one-based.
        ability: Option<u32>,
        /// Targets for the question the activation asks.
        targets: Vec<String>,
    },
    /// Choose cards, targets or a permanent to keep: names or ids.
    Choose(Vec<String>),
    /// Yes to a yes/no question.
    Yes,
    /// No to a yes/no question.
    No,
    /// A colour, as its letter or its name.
    Color(String),
    /// A way to cast a spell: `m1`, `m2`, ….
    Mode(String),
    /// A number.
    Number(u32),
    /// An attack: `(attacker, defender)` pairs; none for "attack none".
    Attack(Vec<(String, String)>),
    /// Pass priority, once.
    Pass,
}

impl Step {
    /// Reads one step as the model wrote it.
    ///
    /// # Errors
    /// The step is not in the grammar, with the grammar.
    pub fn parse(written: &str) -> Result<Self, String> {
        let trimmed = written.trim();
        let (verb, rest) = trimmed
            .split_once(char::is_whitespace)
            .map_or((trimmed, ""), |(verb, rest)| (verb, rest.trim()));
        Self::read(&verb.to_lowercase(), rest)
            .map_err(|why| format!("plan step «{trimmed}» {why}; a step is one of: {GRAMMAR}"))
    }

    /// One step from its verb and the rest; `Err` says what is wrong.
    fn read(verb: &str, rest: &str) -> Result<Self, &'static str> {
        match verb {
            "play" => Ok(Self::Play(one_id(rest)?)),
            "cast" => {
                let (card, targets) = with_targets(rest)?;
                Ok(Self::Cast {
                    card: one_id(&card)?,
                    targets,
                })
            }
            "activate" => {
                let (head, targets) = with_targets(rest)?;
                let (source, ability) = match head.split_once(':') {
                    Some((source, n)) => {
                        let n: u32 = n
                            .trim()
                            .parse()
                            .ok()
                            .filter(|n| *n > 0)
                            .ok_or("names its ability by a number from 1")?;
                        (source.trim().to_string(), Some(n))
                    }
                    None => (head, None),
                };
                Ok(Self::Activate {
                    source: one_id(&source)?,
                    ability,
                    targets,
                })
            }
            "choose" => {
                let items: Vec<String> = rest
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
                if items.is_empty() {
                    return Err("names nothing to choose");
                }
                Ok(Self::Choose(items))
            }
            "yes" if rest.is_empty() => Ok(Self::Yes),
            "no" if rest.is_empty() => Ok(Self::No),
            "color" | "colour" => {
                let color = normal(rest);
                let colors = [
                    "w", "u", "b", "r", "g", "white", "blue", "black", "red", "green",
                ];
                if colors.contains(&color.as_str()) {
                    Ok(Self::Color(color))
                } else {
                    Err("names no colour: W, U, B, R or G")
                }
            }
            "mode" => {
                let mode = normal(rest);
                if mode.strip_prefix('m').is_some_and(digits) {
                    Ok(Self::Mode(mode))
                } else {
                    Err("names no option like m1")
                }
            }
            "n" | "number" => rest
                .parse()
                .map(Self::Number)
                .map_err(|_| "needs a whole number"),
            "attack" => attack(rest),
            "pass" if rest.is_empty() => Ok(Self::Pass),
            _ => Err("is not a step"),
        }
    }

    /// Whether the step answers a priority.
    #[must_use]
    pub const fn at_priority(&self) -> bool {
        matches!(
            self,
            Self::Play(_) | Self::Cast { .. } | Self::Activate { .. } | Self::Pass
        )
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let targets = |f: &mut fmt::Formatter<'_>, targets: &[String]| {
            if targets.is_empty() {
                Ok(())
            } else {
                write!(f, " -> {}", targets.join(" "))
            }
        };
        match self {
            Self::Play(card) => write!(f, "play {card}"),
            Self::Cast { card, targets: t } => {
                write!(f, "cast {card}")?;
                targets(f, t)
            }
            Self::Activate {
                source,
                ability,
                targets: t,
            } => {
                write!(f, "activate {source}")?;
                if let Some(n) = ability {
                    write!(f, ":{n}")?;
                }
                targets(f, t)
            }
            Self::Choose(items) => write!(f, "choose {}", items.join(", ")),
            Self::Yes => f.write_str("yes"),
            Self::No => f.write_str("no"),
            Self::Color(color) => write!(f, "color {}", color.to_uppercase()),
            Self::Mode(mode) => write!(f, "mode {mode}"),
            Self::Number(n) => write!(f, "n {n}"),
            Self::Attack(pairs) if pairs.is_empty() => f.write_str("attack none"),
            Self::Attack(pairs) => {
                let pairs: Vec<String> = pairs.iter().map(|(a, d)| format!("{a}@{d}")).collect();
                write!(f, "attack {}", pairs.join(" "))
            }
            Self::Pass => f.write_str("pass"),
        }
    }
}

/// `none`, or `attacker@defender` pairs.
fn attack(rest: &str) -> Result<Step, &'static str> {
    if normal(rest) == "none" {
        return Ok(Step::Attack(Vec::new()));
    }
    let pairs: Option<Vec<(String, String)>> = rest
        .split([' ', ','])
        .filter(|pair| !pair.trim().is_empty())
        .map(|pair| {
            let (attacker, at) = pair.split_once('@')?;
            (is_id(attacker) && (is_player(at) || is_id(at)))
                .then(|| (attacker.to_string(), at.to_string()))
        })
        .collect();
    match pairs {
        Some(pairs) if !pairs.is_empty() => Ok(Step::Attack(pairs)),
        _ => Err("needs attacker@defender pairs like #30@P2, or none"),
    }
}

/// One object id, as the whole of `rest`.
fn one_id(rest: &str) -> Result<String, &'static str> {
    if is_id(rest) {
        Ok(rest.to_string())
    } else {
        Err("needs one object id like #45")
    }
}

/// `rest` and the targets after its `->`.
fn with_targets(rest: &str) -> Result<(String, Vec<String>), &'static str> {
    let Some((head, targets)) = rest.split_once("->") else {
        return Ok((rest.trim().to_string(), Vec::new()));
    };
    let targets: Vec<String> = targets
        .split([' ', ','])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    if targets.is_empty() || targets.iter().any(|t| !is_id(t) && !is_player(t)) {
        return Err("names a target that is not an object id or a player id");
    }
    Ok((head.trim().to_string(), targets))
}

/// A non-empty run of digits.
fn digits(written: &str) -> bool {
    !written.is_empty() && written.chars().all(|c| c.is_ascii_digit())
}

/// `#45`, or `#45.2` with a generation.
fn is_id(written: &str) -> bool {
    written
        .strip_prefix('#')
        .is_some_and(|rest| digits(rest.split_once('.').map_or(rest, |(slot, _)| slot)))
}

/// `P2`, or `you`/`me`.
fn is_player(written: &str) -> bool {
    let written = normal(written);
    written == "you" || written == "me" || written.strip_prefix('p').is_some_and(digits)
}

/// How long the seat passes for the model after an answer, unless
/// something happens (`until`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Until {
    /// Until the seat's next turn.
    MyTurn,
    /// Until the seat's second main phase, this turn or its next.
    MyMain2,
    /// Until the seat's end step, this turn or its next.
    MyEnd,
    /// Until the end of the turn it was written in.
    EndOfTurn,
}

impl Until {
    /// Reads `until`, and the older `hold: "until_my_turn"`.
    ///
    /// # Errors
    /// A value outside the list, with the list.
    pub fn parse(written: &str) -> Result<Option<Self>, String> {
        match normal(written).as_str() {
            "my_turn" | "until_my_turn" => Ok(Some(Self::MyTurn)),
            "my_main2" => Ok(Some(Self::MyMain2)),
            "my_end" => Ok(Some(Self::MyEnd)),
            "end_of_turn" => Ok(Some(Self::EndOfTurn)),
            "none" | "" => Ok(None),
            other => Err(format!(
                "until «{other}» is none of my_turn, my_main2, my_end, end_of_turn, none"
            )),
        }
    }

    /// As the model writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MyTurn => "my_turn",
            Self::MyMain2 => "my_main2",
            Self::MyEnd => "my_end",
            Self::EndOfTurn => "end_of_turn",
        }
    }
}

/// What an opponent's spell or ability on the stack does while `until`
/// holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum React {
    /// Wakes the model.
    #[default]
    All,
    /// Wakes it only when something on the stack targets the seat, a
    /// permanent it controls, or its commander.
    TargetsMe,
    /// Wakes it for nothing: the hand has no answer at instant speed.
    None,
}

impl React {
    /// Reads `react`.
    ///
    /// # Errors
    /// A value outside the list, with the list.
    pub fn parse(written: &str) -> Result<Self, String> {
        match normal(written).as_str() {
            "all" | "" => Ok(Self::All),
            "targets_me" => Ok(Self::TargetsMe),
            "none" => Ok(Self::None),
            other => Err(format!("react «{other}» is none of all, targets_me, none")),
        }
    }

    /// As the model writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::TargetsMe => "targets_me",
            Self::None => "none",
        }
    }
}

/// Why a plan stopped and the model is asked: each reason one sentence,
/// in the words the report uses, so the model learns them from the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The question is not the kind the step answers.
    Kind {
        /// What the table asked, in words.
        asked: &'static str,
    },
    /// The step's ids or names are not in the question's offer.
    NotOffered(String),
    /// Something of another player's is on the stack.
    OpposingStack(String),
    /// A payment is owed.
    Owed,
    /// The turn the plan was written in ended.
    TurnOver,
    /// The table refused what a step sent.
    Refused(String),
    /// The taps of a cast or a payment were interrupted.
    TapsStopped,
    /// The targets named ahead are not legal now.
    TargetsNotLegal,
    /// The attack step passed without the attack the plan names.
    AttackPassed,
}

impl Stop {
    /// The sentence the report says.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            Self::Kind { asked } => format!("the table asked for {asked}"),
            Self::NotOffered(why) => format!("the question does not offer it ({why})"),
            Self::OpposingStack(what) => format!("{what} is on the stack"),
            Self::Owed => "you owe a payment".into(),
            Self::TurnOver => "the turn it was written in ended".into(),
            Self::Refused(why) => format!("the table refused it ({why})"),
            Self::TapsStopped => "the table asked something else while it tapped mana; any \
                                  mana it made is in your pool"
                .into(),
            Self::TargetsNotLegal => "the targets you named ahead are not legal now".into(),
            Self::AttackPassed => "the attack step passed without that attack".into(),
        }
    }
}

/// What the table asks, in the words a stopped plan's report uses.
#[must_use]
pub fn asked(pending: &Pending) -> &'static str {
    match pending {
        Pending::Priority { .. } => "priority",
        Pending::YesNo {
            prompt: YesNoPrompt::DrawOffer { .. },
            ..
        } => "a decision about an offered draw",
        Pending::YesNo { .. } => "a yes or a no",
        Pending::ChooseColor { .. } => "a colour",
        Pending::ChooseCastMode { .. } => "how to cast it",
        Pending::ChooseNumber { .. } => "a number",
        Pending::ChooseAttackers { .. } => "attackers",
        Pending::ChooseBlockers { .. } => "blockers",
        Pending::ChooseTargets { .. } => "targets",
        Pending::ChooseCards { .. }
        | Pending::LegendChoice { .. }
        | Pending::MulliganBottom { .. }
        | Pending::DiscardChoice { .. } => "cards to choose",
        _ => "something the plan has no step for",
    }
}

mod decision;

#[cfg(test)]
mod tests;
