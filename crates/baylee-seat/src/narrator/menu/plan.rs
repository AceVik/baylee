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
use baylee_engine::choice::{Pending, PlayerAction};
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
    /// The seat lost life since the plan was written.
    LifeFell {
        /// Its life then.
        from: i32,
        /// Its life now.
        to: i32,
    },
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
            Self::LifeFell { from, to } => {
                format!("your life fell from {from} to {to} since you wrote it")
            }
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

impl Menu {
    /// The answer `step` gives this question, read against its offer as a
    /// model's own answer is: the kind of question first, then every id
    /// and name. `Err` is why the plan stops here.
    ///
    /// # Errors
    /// The step does not answer this question exactly.
    pub fn plan_decision(&self, step: &Step, request: &Request) -> Result<Decision, Stop> {
        let table = Table::new(&request.view, &request.context);
        let kind = || Stop::Kind {
            asked: asked(&request.pending),
        };
        let decision = |pick: Vec<String>| Decision {
            ask: Some(format!("q{}", self.question)),
            pick,
            ..Decision::default()
        };
        match (step, &request.pending) {
            (Step::Play(card), Pending::Priority { .. }) => {
                let id = self.option_for(card, |act| match act {
                    PlayerAction::PlayLand { card } => Some(*card),
                    _ => None,
                })?;
                Ok(decision(vec![id]))
            }
            (Step::Cast { card, targets }, Pending::Priority { .. }) => {
                let id = self.option_for(card, |act| match act {
                    PlayerAction::CastSpell { card } => Some(*card),
                    _ => None,
                })?;
                let mut decision = decision(vec![id]);
                decision.then_targets.clone_from(targets);
                Ok(decision)
            }
            (
                Step::Activate {
                    source,
                    ability,
                    targets,
                },
                Pending::Priority { .. },
            ) => {
                let id = self.ability_option(source, *ability)?;
                let mut decision = decision(vec![id]);
                decision.then_targets.clone_from(targets);
                Ok(decision)
            }
            (Step::Pass, Pending::Priority { .. }) => self
                .options
                .iter()
                .find(|c| c.alias.as_deref() == Some("pass"))
                .map(|c| decision(vec![c.id.clone()]))
                .ok_or_else(|| Stop::NotOffered("passing is not offered".into())),
            (Step::Yes, Pending::YesNo { .. }) => Ok(decision(vec!["yes".into()])),
            (Step::No, Pending::YesNo { .. }) => Ok(decision(vec!["no".into()])),
            (Step::Color(color), Pending::ChooseColor { .. })
            | (Step::Mode(color), Pending::ChooseCastMode { .. }) => {
                Ok(decision(vec![color.clone()]))
            }
            (Step::Number(n), Pending::ChooseNumber { .. }) => Ok(Decision {
                number: Some(n.to_string()),
                ..decision(Vec::new())
            }),
            (Step::Attack(pairs), Pending::ChooseAttackers { .. }) => Ok(Decision {
                attacks: pairs.clone(),
                ..decision(Vec::new())
            }),
            (
                Step::Choose(items),
                Pending::ChooseCards { .. }
                | Pending::ChooseTargets { .. }
                | Pending::LegendChoice { .. }
                | Pending::MulliganBottom { .. }
                | Pending::DiscardChoice { .. },
            ) => Ok(decision(self.chosen_by_name(items, &table)?)),
            _ => Err(kind()),
        }
    }

    /// The option whose action `of` reads as acting on the object `written`
    /// names.
    fn option_for(
        &self,
        written: &str,
        of: impl Fn(&PlayerAction) -> Option<ObjectId>,
    ) -> Result<String, Stop> {
        let offered: Vec<(ObjectId, &str)> = self
            .options
            .iter()
            .filter_map(|c| {
                let action = match &c.act {
                    super::Act::Now(action) | super::Act::Taps { then: action, .. } => action,
                };
                of(action).map(|id| (id, c.id.as_str()))
            })
            .collect();
        let ids: Vec<ObjectId> = offered.iter().map(|(id, _)| *id).collect();
        let id = object(written, &ids).map_err(Stop::NotOffered)?;
        Ok(offered
            .iter()
            .find(|(o, _)| *o == id)
            .map(|(_, option)| (*option).to_string())
            .expect("found among them"))
    }

    /// The option that activates `source`'s ability: its only one offered,
    /// or the one numbered `ability`.
    fn ability_option(&self, source: &str, ability: Option<u32>) -> Result<String, Stop> {
        let offered: Vec<(ObjectId, u32, &str)> = self
            .options
            .iter()
            .filter_map(|c| match &c.act {
                super::Act::Now(PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                }) => Some((*source, *ability_index, c.id.as_str())),
                _ => None,
            })
            .collect();
        let sources: Vec<ObjectId> = offered.iter().map(|(id, ..)| *id).collect();
        let id = object(source, &sources).map_err(Stop::NotOffered)?;
        let mine: Vec<&(ObjectId, u32, &str)> = offered
            .iter()
            .filter(|(o, index, _)| *o == id && ability.is_none_or(|n| index + 1 == n))
            .collect();
        match mine.as_slice() {
            [(.., option)] => Ok((*option).to_string()),
            [] => Err(Stop::NotOffered(format!(
                "{source} offers no ability numbered {}",
                ability.unwrap_or(0)
            ))),
            _ => Err(Stop::NotOffered(format!(
                "{source} offers more than one ability; name it as activate {source}:<n>"
            ))),
        }
    }

    /// Ids for `items`: an id among the offer as it is; a name for the
    /// options of that name, which must all be one card (two Swamps a
    /// library search shows are one card; Swamp and Snow-Covered Swamp are
    /// not), each name taking the next of them.
    fn chosen_by_name(&self, items: &[String], table: &Table<'_>) -> Result<Vec<String>, Stop> {
        let offered: &[ObjectId] = match &self.ask {
            Ask::Objects { from, .. } => from,
            Ask::Targets { objects, .. } => objects,
            _ => &[],
        };
        let mut chosen: Vec<ObjectId> = Vec::new();
        let mut out = Vec::new();
        for item in items {
            if is_id(item) || is_player(item) {
                out.push(item.clone());
                continue;
            }
            let wanted = normal(item);
            let named: Vec<ObjectId> = offered
                .iter()
                .copied()
                .filter(|&id| table.name(id).is_some_and(|name| normal(&name) == wanted))
                .collect();
            let mut cards: Vec<CardIndex> =
                named.iter().filter_map(|&id| card_of(table, id)).collect();
            cards.sort_unstable();
            cards.dedup();
            if cards.len() > 1 {
                return Err(Stop::NotOffered(format!(
                    "«{item}» names more than one card"
                )));
            }
            let next = named
                .iter()
                .find(|id| !chosen.contains(id))
                .copied()
                .ok_or_else(|| Stop::NotOffered(format!("no {item} is among the choices")))?;
            chosen.push(next);
            out.push(crate::narrator::tag(next));
        }
        Ok(out)
    }
}

/// The card behind an object the view shows.
fn card_of(table: &Table<'_>, id: ObjectId) -> Option<CardIndex> {
    let view = table.view;
    view.object(id)
        .and_then(|o| o.card)
        .or_else(|| view.hand.iter().find(|c| c.id == id).map(|c| c.card))
        .map(|c| c.index)
}

#[cfg(test)]
mod tests;
