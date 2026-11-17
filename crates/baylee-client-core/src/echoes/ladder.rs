//! What changes from one level of a Tower item to the next, read from the two
//! levels' own words.
//!
//! The item overlay's level ladder (held Shift) marks what the eye must find:
//! a number that changed, and a line the level before did not have. Nothing
//! here knows a balance number. It compares the sentences the client already
//! says for each level, so it holds for every family and both languages
//! without a second copy of the catalogue.
//!
//! A line is cut into tokens: a run of `{…}` symbols (a cost, read as one
//! number), a run of ASCII digits, a word, a run of whitespace. Lines are
//! paired three ways, in order: the same text; the same text once every
//! number token is masked; and, for what is left, the most alike pair by the
//! masked tokens they share (at least half), because a sentence can change
//! around its number (`1 card` / `2 cards`, a once-per-turn tail dropped at
//! the top level). Inside a pair the tokens of the newer line that are not in
//! the longest run both share are marked. A line left unpaired is new.

use std::ops::Range;

/// How one line of a level stands to the level before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Word for word the line the level before says.
    Same,
    /// The level before says this line with something different: the
    /// [`Line::marks`] are what (none when only something was taken away).
    Changed,
    /// The level before has no line like it.
    Added,
}

/// One line of a level, against the level before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// How it stands.
    pub change: Change,
    /// Byte ranges of the line that differ from the line it is paired with,
    /// on token boundaries (never inside a `{…}` symbol), in order and
    /// disjoint. Empty unless [`Change::Changed`].
    pub marks: Vec<Range<usize>>,
}

/// A whole level against the level before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// One entry per line of the newer level, in its order.
    pub lines: Vec<Line>,
    /// Indices of the older level's lines that nothing in the newer one
    /// continues.
    pub removed: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Number,
    Word,
    Space,
}

#[derive(Clone, Debug)]
struct Token<'a> {
    kind: Kind,
    text: &'a str,
    at: usize,
}

impl Token<'_> {
    /// What the token says once a number is masked.
    fn masked(&self) -> &str {
        if self.kind == Kind::Number {
            "#"
        } else {
            self.text
        }
    }
}

fn tokens(line: &str) -> Vec<Token<'_>> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < line.len() {
        let start = at;
        let kind = if bytes[at] == b'{' && symbols_end(line, at) > at {
            at = symbols_end(line, at);
            Kind::Number
        } else if bytes[at].is_ascii_digit() {
            while at < line.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            Kind::Number
        } else if line[at..].starts_with(char::is_whitespace) {
            while let Some(c) = line[at..].chars().next().filter(|c| c.is_whitespace()) {
                at += c.len_utf8();
            }
            Kind::Space
        } else {
            while let Some(c) = line[at..].chars().next() {
                if c.is_whitespace()
                    || c.is_ascii_digit()
                    || (c == '{' && symbols_end(line, at) > at)
                {
                    break;
                }
                at += c.len_utf8();
            }
            Kind::Word
        };
        out.push(Token {
            kind,
            text: &line[start..at],
            at: start,
        });
    }
    out
}

/// The end of the run of closed `{…}` symbols starting at `at` (`at` itself
/// when there is none).
fn symbols_end(line: &str, mut at: usize) -> usize {
    while line[at..].starts_with('{') {
        match line[at..].find('}') {
            Some(close) if !line[at + 1..at + close].contains('{') => at += close + 1,
            _ => break,
        }
    }
    at
}

/// The longest common subsequence of `older` and `newer` under `same`, as a
/// flag per token of `newer`: whether it belongs to it.
fn common<T>(older: &[T], newer: &[T], same: impl Fn(&T, &T) -> bool) -> Vec<bool> {
    let (rows, cols) = (older.len(), newer.len());
    // `table[row][col]`: the subsequence's length from `older[row..]` and
    // `newer[col..]`.
    let mut table = vec![vec![0u16; cols + 1]; rows + 1];
    for row in (0..rows).rev() {
        for col in (0..cols).rev() {
            table[row][col] = if same(&older[row], &newer[col]) {
                table[row + 1][col + 1] + 1
            } else {
                table[row + 1][col].max(table[row][col + 1])
            };
        }
    }
    let mut kept = vec![false; cols];
    let (mut row, mut col) = (0, 0);
    while row < rows && col < cols {
        if same(&older[row], &newer[col]) {
            kept[col] = true;
            row += 1;
            col += 1;
        } else if table[row + 1][col] >= table[row][col + 1] {
            row += 1;
        } else {
            col += 1;
        }
    }
    kept
}

/// How alike two lines read once numbers are masked: shared tokens over all
/// tokens (Dice), spaces left out.
fn likeness(a: &[Token<'_>], b: &[Token<'_>]) -> f32 {
    let a: Vec<&str> = a
        .iter()
        .filter(|t| t.kind != Kind::Space)
        .map(Token::masked)
        .collect();
    let b: Vec<&str> = b
        .iter()
        .filter(|t| t.kind != Kind::Space)
        .map(Token::masked)
        .collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let shared = common(&a, &b, |x, y| x == y)
        .iter()
        .filter(|kept| **kept)
        .count();
    #[allow(clippy::cast_precision_loss)] // lines are a few dozen tokens
    let likeness = 2.0 * shared as f32 / (a.len() + b.len()) as f32;
    likeness
}

/// The ranges of `current` that `previous` does not say.
fn marks(previous: &[Token<'_>], current: &[Token<'_>]) -> Vec<Range<usize>> {
    let kept = common(previous, current, |a, b| a.text == b.text);
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut last = None::<usize>;
    for (index, token) in current.iter().enumerate() {
        if kept[index] || token.kind == Kind::Space {
            continue;
        }
        let end = token.at + token.text.len();
        // Two marked tokens with only whitespace between read as one change.
        let joins = last.is_some_and(|before| {
            current[before + 1..index]
                .iter()
                .all(|t| t.kind == Kind::Space)
        });
        match out.last_mut() {
            Some(range) if joins => range.end = end,
            _ => out.push(token.at..end),
        }
        last = Some(index);
    }
    out
}

/// Compares a level's lines with the level before's.
#[must_use]
pub fn step(previous: &[&str], current: &[&str]) -> Step {
    let before: Vec<Vec<Token<'_>>> = previous.iter().map(|l| tokens(l)).collect();
    let after: Vec<Vec<Token<'_>>> = current.iter().map(|l| tokens(l)).collect();
    let mut partner: Vec<Option<usize>> = vec![None; current.len()];
    let mut taken = vec![false; previous.len()];
    let masked = |line: &[Token<'_>]| line.iter().map(Token::masked).collect::<String>();
    let passes: [&dyn Fn(usize, usize) -> bool; 2] =
        [&|i, j| previous[i] == current[j], &|i, j| {
            masked(&before[i]) == masked(&after[j])
        }];
    for pass in passes {
        for (j, slot) in partner.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            if let Some(i) = (0..previous.len()).find(|&i| !taken[i] && pass(i, j)) {
                taken[i] = true;
                *slot = Some(i);
            }
        }
    }
    for (j, slot) in partner.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let best = (0..previous.len())
            .filter(|&i| !taken[i])
            .map(|i| (i, likeness(&before[i], &after[j])))
            .filter(|&(_, alike)| alike >= 0.5)
            .fold(None::<(usize, f32)>, |best, (i, alike)| match best {
                Some((_, top)) if top >= alike => best,
                _ => Some((i, alike)),
            });
        if let Some((i, _)) = best {
            taken[i] = true;
            *slot = Some(i);
        }
    }
    let lines = partner
        .iter()
        .enumerate()
        .map(|(j, slot)| match slot {
            None => Line {
                change: Change::Added,
                marks: Vec::new(),
            },
            Some(i) if previous[*i] == current[j] => Line {
                change: Change::Same,
                marks: Vec::new(),
            },
            Some(i) => Line {
                change: Change::Changed,
                marks: marks(&before[*i], &after[j]),
            },
        })
        .collect();
    let removed = (0..previous.len()).filter(|&i| !taken[i]).collect();
    Step { lines, removed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked<'a>(line: &'a str, step: &Line) -> Vec<&'a str> {
        step.marks.iter().map(|r| &line[r.clone()]).collect()
    }

    #[test]
    fn an_identical_line_is_the_same() {
        let line = "Creatures you control have vigilance.";
        let step = step(&[line], &[line]);
        assert_eq!(step.lines[0].change, Change::Same);
        assert!(step.lines[0].marks.is_empty());
        assert!(step.removed.is_empty());
    }

    #[test]
    fn a_changed_number_is_marked_and_nothing_else() {
        let now = "Equipped: +6 starting life.";
        let step = step(&["Equipped: +4 starting life."], &[now]);
        assert_eq!(step.lines[0].change, Change::Changed);
        assert_eq!(marked(now, &step.lines[0]), ["6"]);
    }

    #[test]
    fn of_two_numbers_only_the_one_that_changed_is_marked() {
        let now = "Create 2 Plant tokens with 1 charge counter each.";
        let step = step(
            &["Create 1 Plant tokens with 1 charge counter each."],
            &[now],
        );
        assert_eq!(marked(now, &step.lines[0]), ["2"]);
        let now = "Create 2 Plant tokens with 3 charge counter each.";
        let step = super::step(
            &["Create 2 Plant tokens with 1 charge counter each."],
            &[now],
        );
        assert_eq!(marked(now, &step.lines[0]), ["3"]);
    }

    #[test]
    fn a_cost_is_one_number_and_is_never_cut_inside_its_braces() {
        let now = "{2}, Remove a charge counter: Add {C}{C}{C}.";
        let step = step(&["{3}, Remove a charge counter: Add {C}{C}."], &[now]);
        assert_eq!(step.lines[0].change, Change::Changed);
        assert_eq!(marked(now, &step.lines[0]), ["{2}", "{C}{C}{C}"]);
    }

    #[test]
    fn a_line_the_level_before_lacks_is_added() {
        let new = "Creatures you control have lifelink.";
        let step = step(
            &["Creatures you control get +0/+1."],
            &["Creatures you control get +0/+2.", new],
        );
        assert_eq!(step.lines[0].change, Change::Changed);
        assert_eq!(step.lines[1].change, Change::Added);
        assert!(step.lines[1].marks.is_empty());
        assert!(step.removed.is_empty());
    }

    #[test]
    fn a_line_the_newer_level_drops_is_removed() {
        let step = step(
            &["At the beginning of your upkeep, scry 1.", "Draw a card."],
            &["Draw a card."],
        );
        assert_eq!(step.lines[0].change, Change::Same);
        assert_eq!(step.removed, [0]);
    }

    #[test]
    fn a_sentence_that_changes_around_its_number_is_still_the_same_line() {
        // A once-per-turn tail dropped at the top level: changed, not new.
        let step = step(
            &["Remove a charge counter: Deal 3 damage. Activate only once each turn."],
            &["Remove a charge counter: Deal 4 damage."],
        );
        assert_eq!(step.lines[0].change, Change::Changed);
        assert!(step.removed.is_empty());
        // A plural that follows its number marks the number and the noun.
        let now = "At the beginning of your upkeep, draw 2 cards.";
        let step = super::step(&["At the beginning of your upkeep, draw 1 card."], &[now]);
        assert_eq!(marked(now, &step.lines[0]), ["2 cards."]);
    }

    #[test]
    fn german_lines_compare_the_same_way() {
        let now = "Ausgerüstet: +6 Start-LeP.";
        let step = step(
            &[
                "Ausgerüstet: +4 Start-LeP.",
                "Zu Beginn deines Versorgungssegments: Hellsicht 1.",
            ],
            &[
                "Zu Beginn deines Versorgungssegments: Hellsicht 1.",
                now,
                "Kreaturen unter deiner Kontrolle haben Wachsamkeit.",
            ],
        );
        assert_eq!(step.lines[0].change, Change::Same);
        assert_eq!(step.lines[1].change, Change::Changed);
        assert_eq!(marked(now, &step.lines[1]), ["6"]);
        assert_eq!(step.lines[2].change, Change::Added);
        assert!(step.removed.is_empty());
    }

    #[test]
    fn an_unlike_line_is_not_forced_into_a_pair() {
        let step = step(&["Draw a card."], &["Creatures you control have hexproof."]);
        assert_eq!(step.lines[0].change, Change::Added);
        assert_eq!(step.removed, [0]);
    }

    #[test]
    fn marks_fall_on_char_boundaries_in_any_text() {
        let now = "Hellsicht 2 für Äpfel {G}{G}.";
        let step = step(&["Hellsicht 1 für Äpfel {G}."], &[now]);
        for range in &step.lines[0].marks {
            assert!(now.is_char_boundary(range.start) && now.is_char_boundary(range.end));
        }
        assert_eq!(marked(now, &step.lines[0]), ["2", "{G}{G}"]);
    }
}
