//! What a person watching the bridge's terminal reads (`llm-seat.md` §11).
//!
//! One headline per woken decision, then what the mind chose, the sentence
//! it said, its reasoning where the provider shows it, and its tokens; the
//! steps of a plan it made, one short line each; and every answer the house
//! gave instead, with why. At the end, the counts and the rule of thumb:
//! over a tenth of the decisions answered by the house means the table's
//! clock is too fast for this mind.
//!
//! Read from the seat's transcript notes and the mind's own note on each
//! answer, which carry no secret: the seat core never holds a token, and
//! the API mind scrubs its key from everything it writes.

use crate::llm::Tally;
use crate::seat::{By, Stats};
use crate::transcript::{Event, Note};
use baylee_engine::choice::PlayerAction;
use baylee_view::Step;
use serde_json::Value;
use std::collections::BTreeMap;

/// The most characters of a model's reasoning the terminal prints.
const REASONING: usize = 600;

/// What the terminal has been told, for the lines still to come.
#[derive(Debug, Default)]
pub struct Show {
    /// Why the mind gave no answer to the question in hand.
    failure: Option<String>,
    /// House answers by why.
    house: BTreeMap<String, u32>,
    /// Steps a plan answered.
    plan_steps: u32,
}

impl Show {
    /// A show with nothing told yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The lines one note adds; none for the ones a watcher need not read
    /// (standing answers, steps of a payment being asked).
    pub fn note(&mut self, note: &Note) -> Vec<String> {
        match &note.event {
            Event::Asked {
                kind,
                step,
                their_turn,
                continuing,
                retry,
                budget_ms,
                ..
            } => {
                self.failure = None;
                if *continuing {
                    return Vec::new();
                }
                let whose = if *their_turn {
                    "their turn"
                } else {
                    "your turn"
                };
                let again = if *retry { " (asked again)" } else { "" };
                vec![format!(
                    "q{} · turn {} · {whose} · {} · {kind} · {} s{again}",
                    note.question,
                    note.turn,
                    step_words(*step),
                    budget_ms / 1000
                )]
            }
            Event::Answered {
                by,
                action,
                note: said,
                thinking: _,
                ..
            } => self.answered(*by, action, said.as_deref()),
            Event::Refused { by, reason, .. } => {
                let who = match by {
                    crate::RefusedBy::Referee => "the referee",
                    crate::RefusedBy::Table => "the table",
                };
                self.failure = Some(format!("refused by {who}"));
                vec![format!("  refused by {who}: {reason}")]
            }
            Event::MindFailed { error } => {
                self.failure = Some(error.clone());
                Vec::new()
            }
            Event::Expired => {
                self.failure = Some("budget: the time ran out".into());
                Vec::new()
            }
            Event::Late => vec!["  (an answer came after its question was gone)".into()],
            Event::MindDown => {
                vec!["the mind is down: the house plays until it answers again".into()]
            }
            Event::TableSaid { message } => vec![format!("table: {message}")],
            Event::Left { reason } => vec![format!("left the table: {reason}")],
            Event::Over { result } => vec![format!("game over: {result}")],
            Event::Standing { .. }
            | Event::Unanswerable
            | Event::Withdrawn
            | Event::Resent
            | Event::Closed => Vec::new(),
        }
    }

    fn answered(&mut self, by: By, action: &PlayerAction, said: Option<&str>) -> Vec<String> {
        match by {
            By::Mind => {
                let note: Value = said
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(Value::Null);
                if let Some(plan) = note.get("plan").and_then(Value::as_str) {
                    self.plan_steps += 1;
                    return vec![format!("  · {} ({plan})", describe(action))];
                }
                mind_lines(&note, action)
            }
            By::House | By::Least => {
                let why = self
                    .failure
                    .take()
                    .unwrap_or_else(|| "no answer from the mind".into());
                let short = reason_key(&why);
                *self.house.entry(short.clone()).or_insert(0) += 1;
                let who = if by == By::House {
                    "house answered"
                } else {
                    "least answer"
                };
                vec![format!("  {who} ({short}): {}", describe(action))]
            }
            By::Standing => Vec::new(),
        }
    }

    /// The closing lines: decisions, who answered them, the tokens and the
    /// money, and the rule of thumb.
    #[must_use]
    pub fn summary(&self, stats: &Stats, tally: Option<&Tally>) -> Vec<String> {
        let mut out = Vec::new();
        let decisions = stats.wakes + stats.continuations;
        let fell = stats.fallbacks.total();
        let rate = if decisions == 0 {
            0.0
        } else {
            f64::from(fell) * 100.0 / f64::from(decisions)
        };
        out.push(format!(
            "{} wakes, {} payment steps ({} of them a plan's); the mind answered {}, the house {} \
             ({rate:.1} %), the least answer {}",
            stats.wakes,
            stats.continuations,
            self.plan_steps,
            stats.answered.mind,
            stats.answered.house,
            stats.answered.least,
        ));
        if !self.house.is_empty() {
            let why: Vec<String> = self
                .house
                .iter()
                .map(|(why, n)| format!("{n}× {why}"))
                .collect();
            out.push(format!("house answers: {}", why.join(", ")));
        }
        if let Some(tally) = tally {
            let usage = tally.usage;
            let money = tally
                .usd
                .map_or_else(|| "price unknown".to_string(), |usd| format!("${usd:.2}"));
            let calls = tally.calls_cap.map_or_else(
                || format!("{} calls", tally.calls),
                |cap| format!("{} of {cap} calls", tally.calls),
            );
            out.push(format!(
                "tokens: {} in, {} written to the cache, {} read from it, {} out; {calls} \
                 ({} failed); {money}",
                usage.input, usage.cache_write, usage.cache_read, usage.output, tally.failed,
            ));
            if tally.spent {
                out.push("the game's budget ran out: the house finished the game".into());
            }
        }
        out.push(format!(
            "rule of thumb: over 10 % house answers means this clock is too fast for this mind \
             ({})",
            if rate > 10.0 { "it was" } else { "it was not" }
        ));
        out
    }
}

/// A mind's answer: what it chose, what it said, what it thought.
fn mind_lines(note: &Value, action: &PlayerAction) -> Vec<String> {
    let mut out = Vec::new();
    let chose = note
        .get("chose")
        .and_then(Value::as_str)
        .map_or_else(|| describe(action), str::to_string);
    out.push(format!("  → {chose}"));
    if let Some(say) = note.get("say").and_then(Value::as_str) {
        out.push(format!("    says: {say}"));
    }
    if let Some(reasoning) = note.get("reasoning").and_then(Value::as_str) {
        let flat: String = reasoning.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut shown: String = flat.chars().take(REASONING).collect();
        if flat.chars().count() > REASONING {
            shown.push('…');
        }
        out.push(format!("    thinks: {shown}"));
    }
    if let Some(tokens) = note.get("tokens") {
        let n = |key: &str| tokens.get(key).and_then(Value::as_u64).unwrap_or(0);
        let ms = note.get("ms").and_then(Value::as_u64).unwrap_or(0);
        out.push(format!(
            "    {:.1} s · {} in, {} cached, {} out",
            f64::from(u32::try_from(ms).unwrap_or(u32::MAX)) / 1000.0,
            n("input") + n("cache_write"),
            n("cache_read"),
            n("output"),
        ));
    }
    out
}

/// Why the house answered, in a few words to count by.
fn reason_key(why: &str) -> String {
    if why.contains("budget of") || why.contains("budget is spent") {
        "the game's budget is spent".into()
    } else if why.starts_with("budget") || why.contains("no time left") {
        "the time ran out".into()
    } else if why.contains("could not be read") {
        "an unreadable answer".into()
    } else if why.contains("refused to answer") {
        "the model refused".into()
    } else if why.starts_with("refused by") {
        "refused twice".into()
    } else if why.contains("unavailable") {
        let tail = why.split_once(": ").map_or(why, |(_, rest)| rest);
        format!("unavailable: {}", tail.chars().take(80).collect::<String>())
    } else {
        why.chars().take(80).collect()
    }
}

/// A step in a watcher's words.
const fn step_words(step: Step) -> &'static str {
    match step {
        Step::Untap => "untap",
        Step::Upkeep => "upkeep",
        Step::Draw => "draw",
        Step::Main => "main phase",
        Step::CombatBegin => "beginning of combat",
        Step::DeclareAttackers => "declare attackers",
        Step::DeclareBlockers => "declare blockers",
        Step::CombatDamageFirst => "first-strike damage",
        Step::CombatDamage => "combat damage",
        Step::CombatEnd => "end of combat",
        Step::End => "end step",
        Step::Cleanup => "cleanup",
    }
}

/// An action in a few words, with the `#` handles the narrator uses.
#[must_use]
pub fn describe(action: &PlayerAction) -> String {
    let tag = crate::narrator::tag;
    let ids = |ids: &[baylee_core::ids::ObjectId]| {
        if ids.is_empty() {
            "nothing".to_string()
        } else {
            ids.iter().map(|&o| tag(o)).collect::<Vec<_>>().join(", ")
        }
    };
    let player = |p: baylee_core::ids::PlayerId| format!("P{}", u16::from(p.get()) + 1);
    match action {
        PlayerAction::MulliganKeep => "keep".into(),
        PlayerAction::MulliganTake => "mulligan".into(),
        PlayerAction::PassPriority => "pass".into(),
        PlayerAction::PlayLand { card } => format!("play land {}", tag(*card)),
        PlayerAction::CastSpell { card } => format!("cast {}", tag(*card)),
        PlayerAction::ActivateManaAbility { source } => format!("tap {} for mana", tag(*source)),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => format!("activate {} (ability {ability_index})", tag(*source)),
        PlayerAction::DeclareAttackers { attackers } if attackers.is_empty() => {
            "attack with nothing".into()
        }
        PlayerAction::DeclareAttackers { attackers } => {
            let pairs: Vec<String> = attackers
                .iter()
                .map(|(c, d)| {
                    let at = match d {
                        baylee_core::ids::Defender::Player(p) => player(*p),
                        baylee_core::ids::Defender::Planeswalker(o) => tag(*o),
                    };
                    format!("{} → {at}", tag(*c))
                })
                .collect();
            format!("attack: {}", pairs.join(", "))
        }
        PlayerAction::DeclareBlockers { blockers } if blockers.is_empty() => "block nothing".into(),
        PlayerAction::DeclareBlockers { blockers } => {
            let pairs: Vec<String> = blockers
                .iter()
                .map(|(b, a)| format!("{} blocks {}", tag(*b), tag(*a)))
                .collect();
            pairs.join(", ")
        }
        PlayerAction::ChooseObjects { objects } => format!("choose {}", ids(objects)),
        PlayerAction::ChooseTargets { objects, players } => {
            let mut named: Vec<String> = objects.iter().map(|&o| tag(o)).collect();
            named.extend(players.iter().map(|&p| player(p)));
            if named.is_empty() {
                "no targets".into()
            } else {
                format!("target {}", named.join(", "))
            }
        }
        PlayerAction::Suspend { card } => format!("suspend {}", tag(*card)),
        PlayerAction::Arrange { piles } => {
            let piles: Vec<String> = piles.iter().map(|p| format!("[{}]", ids(p))).collect();
            format!("arrange {}", piles.join(" "))
        }
        PlayerAction::ChooseColor(color) => {
            format!("choose {}", crate::narrator::color_name(*color))
        }
        PlayerAction::ChooseMode(i) => format!("choose option {}", i + 1),
        PlayerAction::ChooseNumber(n) => format!("choose {n}"),
        PlayerAction::ChoosePlayer(p) => format!("choose {}", player(*p)),
        PlayerAction::YesNo(true) => "yes".into(),
        PlayerAction::YesNo(false) => "no".into(),
        PlayerAction::Concede => "concede".into(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;
    use serde_json::json;

    fn note(event: Event) -> Note {
        Note {
            question: 12,
            seq: 40,
            turn: 7,
            event,
        }
    }

    fn asked() -> Event {
        Event::Asked {
            kind: "Priority",
            why: None,
            step: Step::Main,
            their_turn: false,
            continuing: false,
            retry: false,
            budget_ms: 25_000,
            log_lines: 3,
        }
    }

    /// A wake, the model's answer with its reasoning, a plan step, and a
    /// house answer with why.
    #[test]
    fn a_game_reads_as_headlines_answers_and_house_answers() {
        let mut show = Show::new();
        let mut lines = show.note(&note(asked()));
        let said = json!({
            "chose": "a2 Cast Lightning Bolt #50 {R} (taps Mountain #23)",
            "say": "Bolt the angel.", "reasoning": "The angel\nis their only flyer.",
            "tokens": {"input": 1200, "output": 300, "cache_write": 0, "cache_read": 2000},
            "ms": 4200
        });
        lines.extend(show.note(&note(Event::Answered {
            by: By::Mind,
            action: PlayerAction::ActivateManaAbility {
                source: ObjectId::new(23, 0),
            },
            model_ms: Some(4200),
            note: Some(said.to_string()),
            thinking: None,
        })));
        lines.extend(show.note(&note(Event::Answered {
            by: By::Mind,
            action: PlayerAction::CastSpell {
                card: ObjectId::new(50, 0),
            },
            model_ms: Some(0),
            note: Some(json!({"plan": "a2 Cast Lightning Bolt"}).to_string()),
            thinking: None,
        })));
        lines.extend(show.note(&note(asked())));
        lines.extend(show.note(&note(Event::Expired)));
        lines.extend(show.note(&note(Event::Answered {
            by: By::House,
            action: PlayerAction::PassPriority,
            model_ms: None,
            note: None,
            thinking: None,
        })));
        assert_eq!(
            lines,
            [
                "q12 · turn 7 · your turn · main phase · Priority · 25 s",
                "  → a2 Cast Lightning Bolt #50 {R} (taps Mountain #23)",
                "    says: Bolt the angel.",
                "    thinks: The angel is their only flyer.",
                "    4.2 s · 1200 in, 2000 cached, 300 out",
                "  · cast #50 (a2 Cast Lightning Bolt)",
                "q12 · turn 7 · your turn · main phase · Priority · 25 s",
                "  house answered (the time ran out): pass",
            ]
        );
        let mut stats = Stats {
            wakes: 2,
            continuations: 1,
            ..Stats::default()
        };
        stats.answered.mind = 2;
        stats.answered.house = 1;
        stats.fallbacks.expired = 1;
        let tally = Tally {
            calls: 1,
            usage: crate::llm::Usage {
                input: 1200,
                output: 300,
                cache_write: 0,
                cache_read: 2000,
            },
            usd: Some(0.0059),
            ..Tally::default()
        };
        let summary = show.summary(&stats, Some(&tally));
        assert!(summary[0].contains("(33.3 %)"), "{summary:?}");
        assert_eq!(summary[1], "house answers: 1× the time ran out");
        assert!(summary[2].ends_with("$0.01"), "{summary:?}");
        assert!(summary[2].contains("; 1 calls (0 failed)"), "{summary:?}");
        let capped = Tally {
            calls_cap: Some(500),
            usd: None,
            ..tally
        };
        let summary = show.summary(&stats, Some(&capped));
        assert!(
            summary[2].ends_with("; 1 of 500 calls (0 failed); price unknown"),
            "{summary:?}"
        );
        assert!(summary[3].contains("(it was)"), "{summary:?}");
    }

    #[test]
    fn why_the_house_answered_is_counted_by_its_cause() {
        assert_eq!(
            reason_key("the mind declined: the game's budget of 1000 tokens is spent"),
            "the game's budget is spent"
        );
        assert_eq!(
            reason_key("the mind is unavailable: 529: overloaded_error: Overloaded"),
            "unavailable: 529: overloaded_error: Overloaded"
        );
        assert_eq!(
            reason_key("the mind declined: the model's answer could not be read: x"),
            "an unreadable answer"
        );
    }
}
