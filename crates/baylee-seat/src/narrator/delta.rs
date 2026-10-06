//! A later wake of a turn tells the board as what changed since the last
//! whole board the conversation was told (`docs/llm-protocol.md` §"Delta
//! wakes").
//!
//! The difference is always from one whole board, never from another
//! difference, so the model reads two boards at most and never a chain of
//! them; it is told whole again when too much changed, when the other side
//! has something on the stack, at a new turn, in a new conversation, and
//! when the model asked for it. A section of the board ("Your
//! battlefield", "Stack", a seat's line) is told as its changed lines and
//! the objects no longer in it when every line it lost names an object;
//! otherwise, and whenever its heading changed, it is told whole. The question's own lines name every object an answer may use,
//! so no id has to be recovered from an older board.

use super::{Table, tag};
use baylee_core::ids::ObjectId;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Past this share of the board's lines changed (four in ten), the board
/// is told whole: a short difference is cheaper than the board, and a long
/// one is the board told twice over.
const WHOLE_PAST: (usize, usize) = (4, 10);

/// A whole board the conversation was told.
#[derive(Clone, Debug)]
pub(super) struct Told {
    /// The question it was told with.
    question: u64,
    /// The turn it was told in.
    pub(super) turn: u32,
    /// Its lines, without the empty ones.
    lines: Vec<String>,
    /// The objects it named on the battlefield, on the stack and in the
    /// seat's hand, named as it named them.
    names: Vec<(ObjectId, String)>,
}

impl Told {
    /// `state`, the board as told with `question` in `table`.
    pub(super) fn new(question: u64, table: &Table<'_>, state: &str) -> Self {
        let view = table.view;
        let names = view
            .battlefield
            .iter()
            .chain(&view.stack)
            .map(|object| object.id)
            .chain(view.hand.iter().map(|card| card.id))
            .map(|id| (id, table.named(id)))
            .collect();
        Self {
            question,
            turn: view.turn,
            lines: lines(state),
            names,
        }
    }
}

/// The non-empty lines of `state`.
fn lines(state: &str) -> Vec<String> {
    state
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// One part of the board: a line of its own ("You (P1): 17 life …"), or a
/// heading and the indented lines under it ("Your battlefield:").
struct Section<'a> {
    /// What names it across two boards: its heading up to the colon,
    /// without a trailing count ("Your graveyard").
    key: &'a str,
    heading: &'a str,
    lines: Vec<&'a str>,
}

/// The sections of `lines`, or `None` when two share a key and could not
/// be told apart.
fn sections(lines: &[String]) -> Option<Vec<Section<'_>>> {
    let mut out: Vec<Section<'_>> = Vec::new();
    for line in lines {
        if line.starts_with(' ') {
            out.last_mut()?.lines.push(line);
            continue;
        }
        let head = line.split_once(':').map_or(line.as_str(), |(head, _)| head);
        let key = match head.rfind(" (") {
            Some(at) if head.ends_with(')') => &head[..at],
            _ => head,
        };
        if out.iter().any(|section| section.key == key) {
            return None;
        }
        out.push(Section {
            key,
            heading: line,
            lines: Vec::new(),
        });
    }
    Some(out)
}

/// The ids a line names: "#45", each whole.
fn tags(line: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    let bytes = line.as_bytes();
    let mut at = 0;
    while let Some(found) = line[at..].find('#') {
        let start = at + found;
        let mut end = start + 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > start + 1 {
            out.insert(&line[start..end]);
        }
        at = end;
    }
    out
}

/// `state` told as what changed since `told`, or `None` when it is to be
/// told whole: too much changed, or the board could not be split into
/// sections that match.
pub(super) fn difference(told: &Told, state: &str) -> Option<String> {
    let now = lines(state);
    let before: BTreeSet<&str> = told.lines.iter().map(String::as_str).collect();
    let after: BTreeSet<&str> = now.iter().map(String::as_str).collect();
    let changed = after
        .difference(&before)
        .count()
        .max(before.difference(&after).count());
    if changed * WHOLE_PAST.1 > after.len() * WHOLE_PAST.0 {
        return None;
    }
    let old = sections(&told.lines)?;
    let new = sections(&now)?;
    let mut out = String::new();
    for section in &new {
        let Some(was) = old.iter().find(|was| was.key == section.key) else {
            whole(section, &mut out);
            continue;
        };
        if was.heading != section.heading {
            whole(section, &mut out);
            continue;
        }
        let appeared: Vec<&str> = section
            .lines
            .iter()
            .filter(|line| !was.lines.contains(line))
            .copied()
            .collect();
        let vanished: Vec<&str> = was
            .lines
            .iter()
            .filter(|line| !section.lines.contains(line))
            .copied()
            .collect();
        if appeared.is_empty() && vanished.is_empty() {
            continue;
        }
        // A lost line that names nothing cannot be said as an object
        // that left: the section is told whole.
        if vanished.iter().any(|line| tags(line).is_empty()) {
            whole(section, &mut out);
            continue;
        }
        let still: BTreeSet<&str> = appeared.iter().flat_map(|line| tags(line)).collect();
        let left: Vec<String> = vanished
            .iter()
            .flat_map(|line| tags(line))
            .filter(|t| !still.contains(t))
            .collect::<BTreeSet<&str>>()
            .into_iter()
            .map(|t| {
                told.names
                    .iter()
                    .find(|(id, _)| tag(*id) == t)
                    .map_or_else(|| t.to_string(), |(_, name)| name.clone())
            })
            .collect();
        let heading = section.heading.trim_end_matches(':');
        let _ = writeln!(out, "{heading}, changed lines (the rest as before):");
        for line in appeared {
            let _ = writeln!(out, "{line}");
        }
        if !left.is_empty() {
            let _ = writeln!(out, "  no longer here: {}", left.join(", "));
        }
    }
    for was in &old {
        if was.heading.contains(':') && !new.iter().any(|section| section.key == was.key) {
            let _ = writeln!(out, "{}: none now.", was.key);
        }
    }
    let head = if out.is_empty() {
        format!("Board as at q{}, unchanged.\n", told.question)
    } else {
        format!("Board as at q{}, except:\n", told.question)
    };
    Some(head + &out)
}

/// A section told whole.
fn whole(section: &Section<'_>, out: &mut String) {
    let _ = writeln!(out, "{}", section.heading);
    for line in &section.lines {
        let _ = writeln!(out, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board of twelve lines, `changed` of its creatures' lines changed.
    fn board(changed: usize) -> String {
        let mut out =
            String::from("You (P1): 20 life\nP2 (opponent): 20 life\nYour battlefield:\n");
        for n in 0..9 {
            let state = if n < changed { "tapped" } else { "untapped" };
            let _ = writeln!(out, "  #{} Llanowar Elves · {state}", 30 + n);
        }
        out
    }

    #[test]
    fn five_of_twelve_lines_changed_is_told_whole_and_four_is_not() {
        let told = Told {
            question: 12,
            turn: 7,
            lines: lines(&board(0)),
            names: Vec::new(),
        };
        assert_eq!(told.lines.len(), 12);
        let four = difference(&told, &board(4)).expect("four lines in twelve is a difference");
        assert!(
            four.starts_with("Board as at q12, except:\nYour battlefield, changed lines"),
            "{four}"
        );
        assert_eq!(four.lines().count(), 2 + 4, "{four}");
        assert_eq!(difference(&told, &board(5)), None);
    }

    #[test]
    fn an_id_is_read_whole() {
        let named: Vec<&str> = tags("lands: Forest #21 (tapped), #210, {2}#3x, # 4")
            .into_iter()
            .collect();
        assert_eq!(named, ["#21", "#210", "#3"]);
    }
}
