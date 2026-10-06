//! What an AI seat's mind said beside an answer (`v1::AiLog`), as lines
//! for the client's AI log panel.
//!
//! A debug engine forwards a seat's reasoning to every other seat, and a
//! release one to none (`docs/protocol.md` §"An AI seat's reasoning"), so
//! nothing here decides who may read it; this only turns the two strings the
//! bridge sends into lines. Both are text a model wrote: they are shown as
//! data, cut to a length a panel can hold, and never parsed for anything but
//! the note's few known fields.

use crate::i18n::{Lang, Phrase};

/// The most characters of reasoning one entry shows. A model can think for
/// pages; the panel is a glance at why, and the full transcript stays with
/// the bridge.
pub const THINKING_CHARS: usize = 1_200;

/// The most characters of a note that is not the JSON the bridge writes.
pub const NOTE_CHARS: usize = 100;

/// What one line of an entry is, which is what the panel colours it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Whose mind this is: the entry's first line.
    Head,
    /// What the model thought before it answered.
    Thinking,
    /// The answer it chose.
    Chose,
    /// The sentence it said to the table.
    Say,
    /// What the answer cost: tokens and time.
    Cost,
}

/// One line of an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// What the line is.
    pub kind: Kind,
    /// The text, in the player's language where it is ours.
    pub text: String,
}

/// The lines for what `who` (the seat's display name) said.
///
/// `note` is the bridge's JSON note (`chose`, `say`, `reasoning`, `tokens`,
/// `ms`); a note that is not JSON is shown as it is, cut short. `thinking`
/// is the model's own reasoning, and wins over the note's `reasoning` when
/// both are there, because it is the longer account of the same thing.
#[must_use]
pub fn lines(lang: Lang, who: &str, note: &str, thinking: &str) -> Vec<Line> {
    let line = |kind, text: String| Line { kind, text };
    let mut out = vec![line(Kind::Head, Phrase::AiSaidHead.fill(lang, &[who]))];
    let parsed = serde_json::from_str::<serde_json::Value>(note)
        .ok()
        .filter(serde_json::Value::is_object);
    let field = |name: &str| {
        parsed
            .as_ref()
            .and_then(|v| v.get(name))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let thought = Some(thinking.trim())
        .filter(|s| !s.is_empty())
        .or_else(|| field("reasoning"));
    if let Some(thought) = thought {
        out.push(line(
            Kind::Thinking,
            Phrase::AiSaidThinking.fill(lang, &[&cut(thought, THINKING_CHARS)]),
        ));
    }
    match parsed.as_ref() {
        Some(note) => {
            if let Some(chose) = field("chose") {
                out.push(line(
                    Kind::Chose,
                    Phrase::AiSaidChose.fill(lang, &[&cut(chose, NOTE_CHARS)]),
                ));
            }
            if let Some(say) = field("say") {
                out.push(line(Kind::Say, format!("“{}”", cut(say, NOTE_CHARS))));
            }
            let tokens = note.get("tokens").map_or(0, tokens_of);
            let ms = note.get("ms").and_then(serde_json::Value::as_u64);
            if tokens > 0 || ms.is_some() {
                let secs = format!("{:.1}", ms.unwrap_or(0) as f64 / 1000.0);
                out.push(line(
                    Kind::Cost,
                    Phrase::AiSaidCost.fill(lang, &[&tokens.to_string(), &secs]),
                ));
            }
        }
        None if !note.trim().is_empty() => {
            out.push(line(
                Kind::Chose,
                Phrase::AiSaidChose.fill(lang, &[&cut(note.trim(), NOTE_CHARS)]),
            ));
        }
        None => {}
    }
    out
}

/// Tokens an answer used: a plain count, or the bridge's
/// `{input, output, cache_write, cache_read}`, of which what was read and
/// written fresh is counted.
fn tokens_of(value: &serde_json::Value) -> u64 {
    value.as_u64().unwrap_or_else(|| {
        ["input", "output", "cache_write"]
            .iter()
            .filter_map(|k| value.get(k).and_then(serde_json::Value::as_u64))
            .sum()
    })
}

/// `text` cut to at most `chars` characters, on a character boundary, with
/// an ellipsis when anything was cut.
fn cut(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lines: &[Line]) -> Vec<Kind> {
        lines.iter().map(|l| l.kind).collect()
    }

    /// The bridge's note, as `show.rs` writes it: the answer, the sentence,
    /// the reasoning and what it cost, with tokens as an object.
    #[test]
    fn a_bridge_note_reads_as_head_thinking_choice_saying_and_cost() {
        let note = r#"{"chose":"a2 Cast Lightning Bolt #50","say":"Bolt the angel.",
            "reasoning":"The angel is their only flyer.",
            "tokens":{"input":1200,"output":300,"cache_write":0,"cache_read":2000},"ms":4200}"#;
        let lines = lines(Lang::En, "Ally", note, "");
        assert_eq!(
            kinds(&lines),
            [
                Kind::Head,
                Kind::Thinking,
                Kind::Chose,
                Kind::Say,
                Kind::Cost
            ]
        );
        assert!(lines[0].text.contains("Ally"), "{:?}", lines[0]);
        assert!(lines[1].text.contains("only flyer"), "{:?}", lines[1]);
        assert!(lines[2].text.contains("Lightning Bolt"), "{:?}", lines[2]);
        assert!(lines[4].text.contains("1500"), "{:?}", lines[4]);
        assert!(lines[4].text.contains("4.2"), "{:?}", lines[4]);
    }

    /// The model's own thinking wins over the note's shorter reasoning.
    #[test]
    fn thinking_wins_over_the_notes_reasoning() {
        let note = r#"{"chose":"pass","reasoning":"short"}"#;
        let lines = lines(Lang::En, "Ally", note, "the long account");
        assert!(lines[1].text.contains("the long account"), "{:?}", lines[1]);
        assert!(!lines[1].text.contains("short"), "{:?}", lines[1]);
    }

    /// A note that is not JSON is cut on a character, not a byte: a model's
    /// German is full of characters two bytes wide, and slicing those by
    /// byte panicked the client the AI log was in.
    #[test]
    fn a_long_note_in_umlauts_is_cut_without_panicking() {
        let note = "ä".repeat(NOTE_CHARS + 50);
        let lines = lines(Lang::De, "Partner", &note, "");
        let chose = &lines[1].text;
        assert!(chose.ends_with('…'), "{chose}");
        assert_eq!(chose.matches('ä').count(), NOTE_CHARS, "{chose}");
    }

    /// Nothing said is a head alone, not an empty line.
    #[test]
    fn nothing_said_is_the_head_alone() {
        assert_eq!(kinds(&lines(Lang::En, "Ally", "", "  ")), [Kind::Head]);
    }
}
