//! Every `CR` citation in the tree, held against a local copy of the rules.
//!
//! The tree cites the Comprehensive Rules about 1770 times, and an audit of
//! all of them found **246 that named the wrong rule** — not typos, but
//! numbers that were right once and have since moved: sacrifice was 701.19a
//! and is 701.21a, discard was 701.7a and is 701.9a, amass was 701.44 and is
//! 701.47. Wizards renumbers a section whenever they insert a keyword action
//! into it, so a citation rots on a schedule nobody in this repository sets.
//! Fixing all 246 by hand was one afternoon; it made none of it unrepeatable.
//!
//! This is the repeatable half, and it is a **report** rather than a gate for
//! a reason that is not about taste: the rules text is Wizards' and is
//! deliberately not vendored (`docs/legal.md` §2), so CI has no copy and a
//! test cannot read one. What CI can check about a citation is nothing at
//! all. So this reads a copy the developer already has, and says so plainly
//! when there is none.
//!
//! # What it can actually check
//!
//! Two things, both mechanical, and the second is the one worth having.
//!
//! **The number exists.** A citation naming a rule the text does not define
//! is wrong whatever it meant to say. This is cheap and catches a section
//! that was deleted outright.
//!
//! **The number is under the word.** The rules text names its own vocabulary:
//! every `701.N` heading is a keyword action (`Sacrifice`, `Discard`,
//! `Destroy`, `Tap and Untap`) and every `702.N` heading a keyword ability
//! (`Flying`, `Disturb`, `Menace`). Where a citing line uses one of those
//! words *and* cites a number in that same section, the number has to sit
//! under that word's heading. That is exactly the shape the drift takes —
//! the prose stayed right and the number slid — and it is checkable without
//! understanding either sentence.
//!
//! Four rules keep it quiet on the citations that are correct, and each
//! of them was added because it fired on one that was not wrong:
//!
//! - **The citing line only.** A `///` block three lines up mentions half of
//!   Magic; widening the window to it turned 4 findings into 6, all false.
//! - **Every rule-shaped number on a citing line counts as a citation**, so
//!   `(CR 702.18a against 702.11b)` is two of them and not one. The second
//!   number carries no `CR`, and reading only the first made a paired
//!   sentence look like a single citation naming the other half's word.
//! - **A line citing two rules of one section is not checked.** The names
//!   and the numbers pair up there in an order this cannot see.
//! - **A word the cited rule's own text already uses is incidental.**
//!   Regenerate's rule says "tap the permanent", so a line that describes
//!   regeneration by what it does is not citing `Tap`.
//!
//! Measured against this repository at the commit that corrected those 246:
//! 17 name findings and 3 unknown numbers before the sweep, and afterwards
//! one — a fault-log entry that had gone stale *about* a citation, which is
//! a finding this could not have been designed to produce and did anyway.
//!
//! # Which files
//!
//! `git ls-files`, filtered by extension — never a glob. The audit's own file
//! list was `crates/**/*.rs` plus `docs/*.md` plus `*.md`, and it silently
//! missed `xtask/src/main.rs`, `.claude/skills/mtg-rules/SKILL.md` and every
//! `docs` subdirectory: 28 citations that 271 agents never saw, three of them
//! wrong. That is the same argument `card_files` makes in `CLAUDE.md` and the
//! same one the rules gate makes by naming one exclusion instead of eight
//! crates. A tracked-file walk cannot lose a directory the way a glob can.
//!
//! And it reports its population against a floor, because "0 findings" out of
//! a worklist of nought is the failure mode a checker is most likely to have.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable that names a rules copy, for a checkout sitting
/// somewhere this cannot guess.
const RULES_ENV: &str = "BAYLEE_COMP_RULES";

/// What a tracked file has to end in to be read.
const EXTENSIONS: [&str; 4] = ["rs", "md", "sh", "toml"];

/// Below this many citations, the walk found a different tree than the one it
/// is about, and a clean report would be a lie. Measured at about 1780.
const CITATION_FLOOR: usize = 1000;

/// Below this many citing files, likewise. Measured at 245.
const FILE_FLOOR: usize = 100;

/// One citation: where it is, what it names, and the whole line it sits on.
///
/// The line is kept in full and shortened only when it is printed. It was
/// stored truncated once, and the word check then read 140 characters of a
/// line whose numbers were read from all of it — a heading word further
/// along was invisible, which is a checker quietly checking less than it
/// says it does.
struct Citation {
    file: String,
    line: usize,
    rule: String,
    said: String,
}

/// How much of a citing line a finding prints.
const SHOWN: usize = 140;

impl Citation {
    fn shown(&self) -> String {
        if self.said.chars().count() <= SHOWN {
            return self.said.clone();
        }
        self.said.chars().take(SHOWN).collect::<String>() + " \u{2026}"
    }
}

/// The rules text, read as three indexes rather than as prose.
struct Rules {
    /// Every number the text defines, `701.21a` and `701.21` alike.
    numbers: BTreeSet<String>,
    /// A rule's own first sentence, keyed by the full number — `701.21a`
    /// and `701.21` are different sentences and `--all` promised the one
    /// that was cited.
    first: BTreeMap<String, String>,
    /// A section's whole text, lowercased — what "the rule already uses this
    /// word" is asked of.
    body: BTreeMap<String, String>,
    /// `701.21` → `["sacrifice"]`, `701.26` → `["tap", "untap"]`.
    headings: BTreeMap<String, Vec<String>>,
    /// An inflected heading word → the sections that claim it.
    claims: BTreeMap<String, BTreeSet<String>>,
}

pub fn run(root: &Path, given: &Path, all: bool) -> anyhow::Result<()> {
    let Some(path) = rules_file(root, given) else {
        println!(
            "cr-check: no Comprehensive Rules text to read, so nothing was checked.\n\
             It is Wizards' text and this repository does not carry it \
             (`docs/legal.md` \u{a7}2). Point at a copy in one of three ways:\n  \
             --rules <path>            relative to the repository, or absolute\n  \
             {RULES_ENV}=<path>  the same, for a checkout somewhere else\n  \
             ../MagicCompRules*.txt    anywhere within four levels of the repository's parent"
        );
        return Ok(());
    };
    let rules = Rules::read(&path)?;
    let citations = collect(root)?;

    let files: BTreeSet<&str> = citations.iter().map(|c| c.file.as_str()).collect();
    if citations.len() < CITATION_FLOOR || files.len() < FILE_FLOOR {
        anyhow::bail!(
            "cr-check read a different tree than it can: {} citations in {} files \
             (floors {CITATION_FLOOR} and {FILE_FLOOR}). A report over a population \
             it never reached says nothing about the tree and still exits green.",
            citations.len(),
            files.len()
        );
    }

    let mut findings = Vec::new();
    let mut per_line: BTreeMap<(&str, usize), Vec<&Citation>> = BTreeMap::new();
    for cite in &citations {
        per_line
            .entry((cite.file.as_str(), cite.line))
            .or_default()
            .push(cite);
    }
    for line in per_line.values() {
        let named = rules.named_on(&line[0].said);
        for cite in line {
            if !rules.numbers.contains(&cite.rule) {
                findings.push(format!(
                    "  UNKNOWN  {}:{} cites CR {}, which the rules do not define\n      {}",
                    cite.file,
                    cite.line,
                    cite.rule,
                    cite.shown()
                ));
                continue;
            }
            if let Some(word) = rules.wrong_word(cite, line, &named) {
                findings.push(format!(
                    "  WORD     {}:{} cites CR {}, but the line says {word}\n      {}",
                    cite.file,
                    cite.line,
                    cite.rule,
                    cite.shown()
                ));
            }
        }
    }

    if all {
        for cite in &citations {
            println!("{}:{} [CR {}]", cite.file, cite.line, cite.rule);
            println!("    says: {}", cite.shown());
            println!(
                "    rule: {}",
                rules
                    .first
                    .get(&cite.rule)
                    .or_else(|| rules.first.get(&section_of(&cite.rule)))
                    .map_or("\u{2014}", String::as_str)
            );
        }
    }

    for finding in &findings {
        println!("{finding}");
    }
    println!(
        "cr-check: {} citations in {} files, {} different rules, against {}",
        citations.len(),
        files.len(),
        citations
            .iter()
            .map(|c| c.rule.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        path.display()
    );
    if findings.is_empty() {
        println!(
            "cr-check: every citation names a rule that exists and matches the word beside it"
        );
        return Ok(());
    }
    anyhow::bail!("{} citation(s) name the wrong rule", findings.len());
}

/// `--rules` if it exists, then the environment, then a bounded search of the
/// repository's neighbours — the same three questions `scripts_root` asks,
/// for the same reason: there is no path that is right for everyone.
fn rules_file(root: &Path, given: &Path) -> Option<PathBuf> {
    let asked = root.join(given);
    if asked.is_file() {
        return Some(asked);
    }
    if let Some(named) = std::env::var_os(RULES_ENV) {
        let named = root.join(named);
        if named.is_file() {
            return Some(named);
        }
    }
    find_rules(&root.join(".."), 4)
}

/// The first file named `MagicCompRules*.txt` within `depth` levels of `at`.
///
/// Bounded and breadth-first for the reason `find_cardsfolder` is: an
/// unbounded walk from the parent of a repository is a walk of somebody's
/// whole home directory.
fn find_rules(at: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(at).ok()?.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            dirs.push(path);
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with("MagicCompRules") && name.to_ascii_lowercase().ends_with(".txt") {
            return Some(path);
        }
    }
    dirs.sort();
    dirs.into_iter().find_map(|dir| find_rules(&dir, depth - 1))
}

impl Rules {
    fn read(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read(path)?;
        // Wizards' download has shipped both a BOM and CRLF line endings.
        let text = String::from_utf8_lossy(raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&raw))
            .replace('\r', "");
        let rules = Self::index(&text);
        anyhow::ensure!(
            rules.numbers.len() > 2000 && rules.headings.len() > 200,
            "{} does not read as a Comprehensive Rules text: {} rules, {} keyword headings",
            path.display(),
            rules.numbers.len(),
            rules.headings.len()
        );
        Ok(rules)
    }

    /// The three indexes, with no floor under them — the half a fixture can
    /// exercise without being the size of the real thing.
    fn index(text: &str) -> Self {
        let mut rules = Self {
            numbers: BTreeSet::new(),
            first: BTreeMap::new(),
            body: BTreeMap::new(),
            headings: BTreeMap::new(),
            claims: BTreeMap::new(),
        };
        for line in text.lines() {
            let Some((number, rest)) = numbered(line) else {
                continue;
            };
            let section = section_of(&number);
            rules.numbers.insert(number.clone());
            rules.first.entry(number.clone()).or_insert_with(|| {
                rest.split_once(". ")
                    .map_or(rest, |(head, _)| head)
                    .to_string()
            });
            let body = rules.body.entry(section.clone()).or_default();
            body.push(' ');
            body.push_str(&rest.to_lowercase());
            if number == section
                && (section.starts_with("701.") || section.starts_with("702."))
                && let Some(words) = heading_words(rest)
            {
                {
                    for word in &words {
                        for form in inflect(word) {
                            rules
                                .claims
                                .entry(form)
                                .or_default()
                                .insert(section.clone());
                        }
                    }
                    rules.headings.insert(section, words);
                }
            }
        }
        rules
    }

    /// The keyword sections whose own word appears on this line.
    fn named_on(&self, said: &str) -> BTreeSet<String> {
        let lower = said.to_lowercase();
        let words: BTreeSet<&str> = lower
            .split(|c: char| !c.is_ascii_alphabetic() && c != '\'' && c != '-')
            .filter(|w| !w.is_empty())
            .collect();
        let mut named = BTreeSet::new();
        for (form, sections) in &self.claims {
            let hit = if form.contains(' ') {
                lower.contains(form.as_str())
            } else {
                words.contains(form.as_str())
            };
            if hit {
                named.extend(sections.iter().cloned());
            }
        }
        named
    }

    /// The heading this line names, when the citation does not sit under it.
    fn wrong_word(
        &self,
        cite: &Citation,
        line: &[&Citation],
        named: &BTreeSet<String>,
    ) -> Option<String> {
        let family = cite.rule.get(..3)?;
        if family != "701" && family != "702" {
            return None;
        }
        // A line citing two rules of one section pairs its words with its
        // numbers in an order this cannot see.
        if line.iter().filter(|c| c.rule.starts_with(family)).count() > 1 {
            return None;
        }
        let section = section_of(&cite.rule);
        let here: Vec<&String> = named.iter().filter(|s| s.starts_with(family)).collect();
        if here.is_empty() || here.iter().any(|s| **s == section) {
            return None;
        }
        let own = self.body.get(&section).map_or("", String::as_str);
        let wrong: Vec<String> = here
            .into_iter()
            .filter(|s| {
                !self.headings[*s]
                    .iter()
                    .flat_map(|w| inflect(w))
                    .any(|form| contains_word(own, &form))
            })
            .map(|s| format!("{s} {}", self.headings[s].join("/")))
            .collect();
        (!wrong.is_empty()).then(|| wrong.join(", "))
    }
}

/// `701.21a Text…` → `("701.21a", "Text…")`, and nothing for a prose line.
fn numbered(line: &str) -> Option<(String, &str)> {
    let bytes = line.as_bytes();
    if bytes.len() < 6 || !bytes[..3].iter().all(u8::is_ascii_digit) || bytes[3] != b'.' {
        return None;
    }
    let mut end = 4;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 4 {
        return None;
    }
    if end < bytes.len() && bytes[end].is_ascii_lowercase() {
        end += 1;
    }
    let number = line[..end].to_string();
    let rest = line[end..].trim_start_matches('.').trim_start();
    (!rest.is_empty()).then_some((number, rest))
}

/// `701.21a` → `701.21`.
fn section_of(rule: &str) -> String {
    rule.trim_end_matches(|c: char| c.is_ascii_lowercase())
        .to_string()
}

/// A keyword heading's words, lowercased, or nothing for a prose paragraph.
///
/// `Tap and Untap` is two words this splits, because a line about untapping
/// names a rule whose heading begins with the other verb.
fn heading_words(title: &str) -> Option<Vec<String>> {
    if title.len() > 40
        || !title
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == ' ' || c == '\'' || c == '-')
    {
        return None;
    }
    let words: Vec<String> = title
        .to_lowercase()
        .split(" and ")
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();
    (!words.is_empty() && title.split_whitespace().count() <= 3).then_some(words)
}

/// The forms a heading takes in a sentence about it.
///
/// The heading is inflected rather than the sentence stemmed, which is not a
/// detail: stemming the line turns every "player" into a mention of `Play`
/// and fires on every 701 citation near the commonest word in Magic.
fn inflect(word: &str) -> Vec<String> {
    let mut forms = vec![word.to_string()];
    if word.ends_with('s') || word.contains(' ') {
        return forms;
    }
    forms.push(format!("{word}s"));
    forms.push(format!("{word}es"));
    forms.push(format!("{word}ed"));
    if let Some(stem) = word.strip_suffix('e') {
        forms.push(format!("{stem}ed"));
        forms.push(format!("{stem}ing"));
    } else {
        forms.push(format!("{word}ing"));
    }
    // English doubles a final consonant after a single vowel — "tapped",
    // "tapping" — and the rules text says both of those far more often than
    // it says "tap". Without this the `Tap and Untap` heading is invisible in
    // every sentence that actually describes tapping something.
    if doubles(word) {
        forms.push(format!(
            "{word}{}ed",
            word.chars().next_back().unwrap_or('_')
        ));
        forms.push(format!(
            "{word}{}ing",
            word.chars().next_back().unwrap_or('_')
        ));
    }
    if let Some(stem) = word.strip_suffix('y')
        && stem.chars().next_back().is_some_and(|c| !is_vowel(c))
    {
        forms.push(format!("{stem}ies"));
        forms.push(format!("{stem}ied"));
    }
    forms
}

/// Whether a verb doubles its last letter before `-ed`: a single vowel
/// between two consonants, with the last one not `w`, `x` or `y`.
fn doubles(word: &str) -> bool {
    let letters: Vec<char> = word.chars().collect();
    let [.., third, second, last] = letters[..] else {
        return false;
    };
    !is_vowel(last)
        && !matches!(last, 'w' | 'x' | 'y')
        && is_vowel(second)
        && !is_vowel(third)
        && letters.len() <= 6
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Whether `haystack` uses `word` as a word rather than inside another.
fn contains_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .any(|w| w == word)
}

/// Every citation in every tracked file this can read.
///
/// A line that cites the rules once has every rule-shaped number on it read
/// as a citation, `CR` or no `CR`: `(CR 702.18a against 702.11b)` is two.
fn collect(root: &Path) -> anyhow::Result<Vec<Citation>> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()?;
    anyhow::ensure!(
        out.status.success(),
        "git ls-files failed in {}",
        root.display()
    );
    let listing = String::from_utf8_lossy(&out.stdout);
    let mut citations = Vec::new();
    for name in listing.split('\0').filter(|n| !n.is_empty()) {
        let path = Path::new(name);
        if !path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if !cites_a_rule(line) {
                continue;
            }
            for rule in rule_shaped(line) {
                citations.push(Citation {
                    file: name.to_string(),
                    line: n + 1,
                    rule,
                    said: line.trim().to_string(),
                });
            }
        }
    }
    Ok(citations)
}

/// Whether a line says `CR ` followed by a rule-shaped number.
fn cites_a_rule(line: &str) -> bool {
    line.match_indices("CR ").any(|(at, _)| {
        let rest = &line[at + 3..];
        let digits = rest.as_bytes();
        digits.len() > 4 && digits[..3].iter().all(u8::is_ascii_digit) && digits[3] == b'.'
    })
}

/// Every `NNN.N` or `NNN.Na` on a line, as whole tokens.
fn rule_shaped(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        let boundary = i == 0 || !bytes[i - 1].is_ascii_digit() && bytes[i - 1] != b'.';
        if boundary && bytes[i..i + 3].iter().all(u8::is_ascii_digit) && bytes[i + 3] == b'.' {
            let mut end = i + 4;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > i + 4 {
                if end < bytes.len() && bytes[end].is_ascii_lowercase() {
                    end += 1;
                }
                let after_is_letter = end < bytes.len() && bytes[end].is_ascii_alphanumeric();
                if !after_is_letter {
                    found.push(line[i..end].to_string());
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{Rules, contains_word, heading_words, inflect, rule_shaped};

    /// The lines a fixture pretends to have read say `rule 701.9a` where a
    /// real comment would say `CR 701.9a`, and deliberately: this file is a
    /// tracked file like any other, so a fixture citing in the citing
    /// spelling would make the checker fail on its own test data. It is the
    /// rule the module doc states, applied to itself.
    ///
    /// A rules text in the Comprehensive Rules' *shape* with none of its
    /// words: Wizards' text is not this repository's to carry, and a fixture
    /// that quoted it would put it here (`docs/legal.md` \u{a7}2).
    const FIXTURE: &str = "\
100.1. A made-up opening paragraph about nothing at all.
701.9. Discard
701.9a A player who is told to yield a card from their grip puts it in the pile.
701.21. Sacrifice
701.21a A player who is told to give up a permanent moves it to the pile.
701.19. Regenerate
701.19a To bring a thing back is to tap it, send it home from the fight and forget the marks on it.
701.26. Tap and Untap
701.26a To turn a thing sideways is one action and to turn it upright is the other.
702.11. Hexproof
702.11a A thing with this word cannot be chosen by what an opponent plays.
702.18. Shroud
702.18a A thing with this word cannot be chosen by anything at all.
";

    fn indexed() -> Rules {
        Rules::index(FIXTURE)
    }

    #[test]
    fn a_heading_joined_by_and_claims_both_of_its_verbs() {
        let rules = indexed();
        assert_eq!(rules.headings["701.26"], ["tap", "untap"]);
        assert!(rules.claims["untapped"].contains("701.26"));
        assert!(rules.claims["tapping"].contains("701.26"));
    }

    #[test]
    fn a_prose_paragraph_is_not_a_heading() {
        assert!(heading_words("A made-up opening paragraph about nothing at all.").is_none());
        assert_eq!(
            heading_words("Sacrifice"),
            Some(vec!["sacrifice".to_string()])
        );
    }

    #[test]
    fn the_word_player_is_not_the_keyword_action_play() {
        let forms = inflect("play");
        assert!(forms.contains(&"playing".to_string()));
        assert!(!forms.contains(&"player".to_string()));
        assert!(!contains_word("a player draws", "play"));
    }

    #[test]
    fn a_line_that_names_a_word_and_cites_another_rules_number_is_a_finding() {
        let rules = indexed();
        let said = "a sacrificed permanent goes to the pile (rule 701.9a)";
        let named = rules.named_on(said);
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "701.9a".into(),
            said: said.into(),
        };
        let found = rules.wrong_word(&cite, &[&cite], &named);
        assert_eq!(found.as_deref(), Some("701.21 sacrifice"));
    }

    #[test]
    fn a_line_that_names_the_rule_it_cites_is_not_a_finding() {
        let rules = indexed();
        let said = "a sacrificed permanent goes to the pile (rule 701.21a)";
        let named = rules.named_on(said);
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "701.21a".into(),
            said: said.into(),
        };
        assert!(rules.wrong_word(&cite, &[&cite], &named).is_none());
    }

    /// Naming two keywords and citing one of them is the shape half the
    /// engine's comments have, and every one of them would be a finding
    /// without the any-of rule.
    #[test]
    fn naming_two_keywords_and_citing_one_of_them_passes() {
        let rules = indexed();
        let said = "hexproof and shroud both keep a chooser out (rule 702.11a)";
        let named = rules.named_on(said);
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "702.11a".into(),
            said: said.into(),
        };
        assert!(rules.wrong_word(&cite, &[&cite], &named).is_none());
    }

    /// Regenerate's own rule says "tap it", so a line describing what
    /// regenerating does is not citing `Tap and Untap` — and the doubled
    /// form is the one such a line actually uses.
    #[test]
    fn a_word_the_cited_rule_itself_uses_is_incidental() {
        let rules = indexed();
        let said = "it is tapped and sent home (rule 701.19a)";
        let named = rules.named_on(said);
        assert!(named.contains("701.26"), "the doubled form has to be seen");
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "701.19a".into(),
            said: said.into(),
        };
        assert!(rules.wrong_word(&cite, &[&cite], &named).is_none());
    }

    /// The same line, citing a rule whose text does *not* use the word.
    #[test]
    fn a_word_the_cited_rule_never_uses_is_a_finding() {
        let rules = indexed();
        let said = "it is tapped and sent home (rule 701.9a)";
        let named = rules.named_on(said);
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "701.9a".into(),
            said: said.into(),
        };
        assert_eq!(
            rules.wrong_word(&cite, &[&cite], &named).as_deref(),
            Some("701.26 tap/untap")
        );
    }

    #[test]
    fn a_line_citing_two_rules_of_one_section_is_left_alone() {
        let rules = indexed();
        let said = "shroud is wider than hexproof (CR 702.18a against 702.11a)";
        let named = rules.named_on(said);
        let first = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "702.18a".into(),
            said: said.into(),
        };
        let second = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "702.11a".into(),
            said: said.into(),
        };
        let line = [&first, &second];
        assert!(rules.wrong_word(&first, &line, &named).is_none());
        assert!(rules.wrong_word(&second, &line, &named).is_none());
    }

    #[test]
    fn a_citation_outside_the_keyword_sections_is_not_word_checked() {
        let rules = indexed();
        let said = "a sacrificed creature was still blocking (rule 509.1a)";
        let named = rules.named_on(said);
        let cite = super::Citation {
            file: "made/up.rs".into(),
            line: 1,
            rule: "509.1a".into(),
            said: said.into(),
        };
        assert!(rules.wrong_word(&cite, &[&cite], &named).is_none());
    }

    #[test]
    fn both_halves_of_a_paired_citation_are_read() {
        assert_eq!(
            rule_shaped("(CR 702.18a against 702.11b) and the film"),
            ["702.18a", "702.11b"]
        );
        assert_eq!(rule_shaped("CR 701.26"), ["701.26"]);
        assert_eq!(
            rule_shaped("a version string 1.2.3 is not a rule"),
            [] as [String; 0]
        );
    }

    #[test]
    fn a_text_that_is_not_the_rules_is_refused_rather_than_read_as_empty() {
        let dir = std::env::temp_dir().join("baylee-cr-check-fixture");
        std::fs::create_dir_all(&dir).expect("a temp directory");
        let path = dir.join("not-the-rules.txt");
        std::fs::write(&path, "701.1. Something\n").expect("it writes");
        assert!(Rules::read(&path).is_err());
    }
}
