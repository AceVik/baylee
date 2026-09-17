//! The token ledger: which id every token there is was assigned, and the
//! definitions of the ones a reader wrote.
//!
//! A token's id is its index in `baylee_cards::tokens::ALL` — that is what
//! the engine stamps on the object it creates and what a client keys token
//! art off. So the table is **append only**: an insertion in the middle
//! renumbers every token after it, and hands one of them another's picture.
//!
//! Two lists would not have been enough. The hand-written tokens grow (a card
//! finished by hand brings its own), and so do the generated ones, so a
//! single table with the hand-written half first would renumber the generated
//! half the first time somebody wrote a token by hand. The table therefore
//! records the **order ids were assigned in**, whichever half an entry came
//! from, and a run may only append to it. That is the same bargain
//! `baylee_cards_index::ROWS` makes for cards, and for the same reason: an
//! assignment that is only ever appended to is one a saved game can name.
//!
//! # Why the table is generated at all
//!
//! A hand-maintained list beside a generated one is two truths, and the
//! compiler checks neither against the other. Here the generated file *is*
//! the ledger — entry, order and definition in one place — so the build is
//! what says whether it is consistent, and `codegen --check` is what says
//! whether it is current.
//!
//! A hand-written token keeps its definition in `baylee_cards::tokens`, where
//! a person can read the comment explaining which printing its art came from;
//! the ledger holds only its name and the id that name is at. A generated one
//! has nowhere else to live, so the ledger carries its definition too — and
//! keeps carrying it, because an entry that has been assigned an id is an
//! entry whose body must still compile after the reference it was read from
//! has moved on.

use crate::tokengen::TokenBody;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// One row of the ledger, in assignment order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The constant this token is filed under.
    pub constant: String,
    /// Where its definition lives.
    pub body: Body,
}

/// Where an entry's `TokenDef` is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// In `baylee_cards::tokens`, written by a person; the ledger names it.
    HandWritten,
    /// In the ledger itself, written by [`crate::tokengen`].
    Generated {
        /// The doc comment the constant carries.
        doc: String,
        /// The `TokenDef { … }` literal.
        literal: String,
        /// Subtype modules the literal names.
        modules: Vec<String>,
    },
}

/// The ledger could not be extended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Two different definitions claim one constant — which would be two
    /// tokens at one id, so the run stops rather than picking one.
    Collision(String),
    /// An entry that already has an id has no definition any more: the
    /// reference no longer produces it. Nothing may take its id, and nothing
    /// can compile without its body, so this is a decision for a person.
    Orphan(String),
    /// The file did not parse as a ledger. Rewriting it from scratch would
    /// reassign every id, so the run stops instead.
    Unreadable(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collision(name) => write!(
                f,
                "two different tokens both want to be `{name}`; one of them would \
                 reach the table wearing the other's picture"
            ),
            Self::Orphan(name) => write!(
                f,
                "`{name}` holds a token id but no longer has a definition; its id \
                 cannot be reused, so decide what it is by hand"
            ),
            Self::Unreadable(why) => write!(f, "the token ledger did not parse: {why}"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// Reads a ledger back, in assignment order.
///
/// The read is what makes the table append-only: a run learns the ids it has
/// already given out from the file it is about to rewrite, exactly as
/// `xtask ledger` does for the card index. It is deliberately strict —
/// anything it does not recognise is [`LedgerError::Unreadable`] rather than
/// a row quietly dropped, because a dropped row is an id handed out twice.
///
/// What it reads is **rustfmt's** output and not this module's, because the
/// file goes out through the same door every other generated file does. So
/// every line is matched trimmed: the one thing that must never happen is a
/// reader that answers an empty table because the formatter moved a brace.
///
/// # Errors
///
/// [`LedgerError::Unreadable`] for anything this does not recognise, which
/// includes a definition no row names — rewriting the table from scratch
/// would reassign every id, so a file that is not understood stops the run.
pub fn parse(text: &str) -> Result<Vec<Entry>, LedgerError> {
    let mut defs: Vec<(String, String, String)> = Vec::new();
    let mut lines = text.lines().peekable();
    let mut doc = String::new();
    let mut order: Option<Vec<String>> = None;
    while let Some(line) = lines.next() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("/// ") {
            doc = rest.to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("pub static ")
            && let Some(name) = rest.strip_suffix(": TokenDef = TokenDef {")
        {
            let mut literal = "TokenDef {\n".to_string();
            loop {
                let body = lines
                    .next()
                    .ok_or_else(|| LedgerError::Unreadable(format!("`{name}` is unterminated")))?;
                if body.trim() == "};" {
                    literal.push('}');
                    break;
                }
                literal.push_str(body);
                literal.push('\n');
            }
            defs.push((name.to_string(), std::mem::take(&mut doc), literal));
            continue;
        }
        // `GENERATED` is rendered from the rows and never read back — it is a
        // view over `ALL`, and reading it would be reading the same fact twice.
        if line == "pub static GENERATED: &[&TokenDef] = &[" {
            while lines.next().is_some_and(|row| row.trim() != "];") {}
            continue;
        }
        if line == "pub static ALL: &[&TokenDef] = &[];" {
            order = Some(Vec::new());
            continue;
        }
        if line == "pub static ALL: &[&TokenDef] = &[" {
            let mut names = Vec::new();
            loop {
                let row = lines
                    .next()
                    .ok_or_else(|| LedgerError::Unreadable("`ALL` is unterminated".into()))?;
                if row.trim() == "];" {
                    break;
                }
                let row = row.trim().trim_end_matches(',');
                let name = row
                    .strip_prefix('&')
                    .ok_or_else(|| LedgerError::Unreadable(format!("`ALL` holds `{row}`")))?;
                names.push(name.trim_start_matches("tokens::").to_string());
            }
            order = Some(names);
        }
    }

    let order = order.ok_or_else(|| LedgerError::Unreadable("no `ALL` table".into()))?;
    let mut bodies: Vec<_> = defs;
    let entries = order
        .into_iter()
        .map(|constant| {
            let body = match bodies.iter().position(|(n, _, _)| *n == constant) {
                Some(i) => {
                    let (_, doc, literal) = bodies.remove(i);
                    Body::Generated {
                        modules: modules_of(&literal),
                        doc,
                        literal,
                    }
                }
                None => Body::HandWritten,
            };
            Entry { constant, body }
        })
        .collect();
    if let Some((name, _, _)) = bodies.first() {
        // A definition the table does not name has no id, so it is not in the
        // ledger at all — and silently keeping it would let a later run give
        // it one out of order.
        return Err(LedgerError::Unreadable(format!(
            "`{name}` is defined but is not in `ALL`"
        )));
    }
    Ok(entries)
}

/// The subtype modules a literal names, recovered from the literal itself.
///
/// Read back rather than stored, because a stored copy is a second truth: the
/// import list and the literal it serves would drift, and the first sign of it
/// would be a build failure in a generated file nobody edited.
fn modules_of(literal: &str) -> Vec<String> {
    let mut out = Vec::new();
    for known in [
        "artifact",
        "creature",
        "enchantment",
        "land",
        "planeswalker",
    ] {
        if literal.contains(&format!("{known}::")) {
            out.push(known.to_string());
        }
    }
    out
}

/// Extends a ledger with whatever is new, and never with anything else.
///
/// `hand` is every token written by hand, in the order `baylee_cards::tokens`
/// declares them; `generated` is every token a reader wrote this run. Both
/// are matched against the table by **name**, which is why the name has to be
/// a function of the token rather than of the script it came from.
///
/// # Errors
///
/// [`LedgerError::Collision`] when two different definitions claim one
/// constant, which would be two tokens at one id, and
/// [`LedgerError::Orphan`] when a row's definition has been withdrawn —
/// nothing may reuse its number, so that is a decision for a person.
pub fn assign(
    existing: Vec<Entry>,
    hand: &[String],
    generated: &[TokenBody],
) -> Result<Vec<Entry>, LedgerError> {
    let mut out = existing;
    // Grows as rows are appended, and that is the whole of why it is a `mut`
    // set rather than a snapshot. Two readable scripts can describe the same
    // token — the name is a function of the token, not of the script — and a
    // snapshot says "new" to both, so the pair is filed twice and the second
    // copy takes an id nothing will ever create. Nothing in the reference
    // does that today (852 scripts, 586 readable, 586 distinct names,
    // measured), which is exactly the kind of fact that stops being true
    // without anybody touching this file.
    let mut known: BTreeSet<String> = out.iter().map(|e| e.constant.clone()).collect();

    // A token that is already in the table keeps its id and its body. What it
    // does *not* keep is a body that has since been withdrawn: a reader that
    // no longer produces an entry leaves it here with nothing to compile, and
    // the run says so instead of writing a file that does not build.
    for entry in &out {
        if matches!(entry.body, Body::HandWritten) && !hand.contains(&entry.constant) {
            return Err(LedgerError::Orphan(entry.constant.clone()));
        }
    }

    for name in hand {
        if known.insert(name.clone()) {
            out.push(Entry {
                constant: name.clone(),
                body: Body::HandWritten,
            });
        }
    }
    for body in generated {
        // A generated token whose name a hand-written one already holds is
        // that hand-written token: the name is a complete description of what
        // the reader read, so the two are the same permanent, and the
        // hand-written definition is the one with a picture chosen for it.
        if known.contains(&body.constant) {
            if let Some(entry) = out.iter().find(|e| e.constant == body.constant)
                && let Body::Generated { literal, .. } = &entry.body
                && !same_definition(literal, &body.literal)
            {
                return Err(LedgerError::Collision(body.constant.clone()));
            }
            continue;
        }
        if let Some(other) = generated
            .iter()
            .find(|o| o.constant == body.constant && !same_definition(&o.literal, &body.literal))
        {
            return Err(LedgerError::Collision(other.constant.clone()));
        }
        known.insert(body.constant.clone());
        out.push(Entry {
            constant: body.constant.clone(),
            body: Body::Generated {
                doc: body.doc.clone(),
                literal: body.literal.clone(),
                modules: body.modules.clone(),
            },
        });
    }
    Ok(out)
}

/// Whether two `TokenDef` literals say the same thing.
///
/// Not `==`, for the reason [`parse`] gives about reading rustfmt's output:
/// one side of this comparison came back out of the formatted file and the
/// other was just built, so a definition long enough to be wrapped disagrees
/// with itself. That is not a theory — the first token to carry an ability
/// was `ELDRAZI_SPAWN_0_1`, whose `abilities: &[mana_ability!(…)]` rustfmt
/// breaks over four lines, and the second `codegen` run refused it as two
/// tokens at one name.
///
/// Whitespace is dropped rather than collapsed, and that is safe **only**
/// because this answers a yes/no question: `name: "Eldrazi Spawn"` becomes
/// `name:"EldraziSpawn"` on both sides alike, so two definitions agree
/// exactly when they agreed before. Nothing here may be shown to anybody.
fn same_definition(a: &str, b: &str) -> bool {
    let bare = |s: &str| -> String { s.chars().filter(|c| !c.is_whitespace()).collect() };
    bare(a) == bare(b)
}

/// Renders the ledger as `crates/baylee-cards/src/generated_tokens.rs`.
///
/// [`parse`] reads exactly what this writes, and `codegen --check` is what
/// holds the pair to it: a round trip that lost a row would hand the next run
/// an id it had already given out.
#[must_use]
pub fn render(entries: &[Entry]) -> String {
    let mut modules: BTreeSet<&str> = BTreeSet::new();
    let mut any_generated = false;
    let mut any_ability = false;
    for entry in entries {
        if let Body::Generated {
            modules: m,
            literal,
            ..
        } = &entry.body
        {
            any_generated = true;
            any_ability |= literal.contains("abilities:");
            modules.extend(m.iter().map(String::as_str));
        }
    }

    let mut out = String::with_capacity(4096);
    out.push_str(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         //\n\
         // The token ledger: every token there is, in the order ids were assigned\n\
         // to them. A token's id is its index in this table, and that number is\n\
         // what the engine stamps on the object it creates and what a client keys\n\
         // token art off — so the table is **append only**. An insertion in the\n\
         // middle renumbers every token after it and hands one of them another's\n\
         // picture.\n\
         //\n\
         // A hand-written token is named here and defined in `crate::tokens`,\n\
         // beside the comment saying which printing lent it its art. A generated\n\
         // one is defined here, because there is nowhere else for it to live.\n\
         //\n\
         // Both halves are **reachable from here**, the hand-written ones by\n\
         // re-export. A card that creates a token names one module and not the\n\
         // half of the ledger the token happens to be written in — which is the\n\
         // same thing `baylee_core::generated::index` does for a `CardIndex`\n\
         // constant, and for the same reason: where a definition sits is the\n\
         // generator's business and changes, and the name is what a card spends.\n\
         //\n\
         // Source: `baylee_cards::tokens` and the card-script reference's token\n\
         // scripts (read as an automated lookup, never copied).\n\
         #![allow(missing_docs, clippy::all, clippy::pedantic)]\n\n",
    );
    out.push_str("use crate::tokens;\n");
    out.push_str("use baylee_cards_dsl::TokenDef;\n");
    // Named one by one rather than as a glob: a glob would also pull in
    // whatever else `tokens` grows, and the point of the door is that what
    // comes through it is exactly the ledger.
    let hand: Vec<&str> = entries
        .iter()
        .filter(|e| matches!(e.body, Body::HandWritten))
        .map(|e| e.constant.as_str())
        .collect();
    if !hand.is_empty() {
        let _ = writeln!(out, "pub use crate::tokens::{{{}}};", hand.join(", "));
    }
    if any_generated {
        out.push_str("use baylee_cards_dsl::KeywordSet;\n");
        // An ability is written with the card DSL's macros, so a token that
        // carries one needs what a card file opens with. Added beside the
        // narrow imports rather than instead of them: an explicit `use`
        // shadows a glob, so the two agree, and a ledger with no ability in
        // it keeps exactly the imports it uses — which is what
        // `a_ledger_of_hand_written_tokens_imports_nothing_it_does_not_use`
        // is about.
        if any_ability {
            out.push_str("use baylee_cards_dsl::prelude::*;\n");
        }
        out.push_str("use baylee_core::color::{Color, ColorSet};\n");
        if !modules.is_empty() {
            let list: Vec<&str> = modules.into_iter().collect();
            let _ = writeln!(
                out,
                "use baylee_core::generated::subtypes::{{{}}};",
                list.join(", ")
            );
        }
        out.push_str("use baylee_core::types::{SupertypeSet, TypeSet};\n");
    }
    out.push('\n');

    for entry in entries {
        if let Body::Generated { doc, literal, .. } = &entry.body {
            let _ = writeln!(out, "/// {doc}");
            let _ = writeln!(out, "pub static {}: TokenDef = {literal};", entry.constant);
            out.push('\n');
        }
    }

    out.push_str("/// Every token there is, in the order ids were assigned.\n");
    out.push_str("pub static ALL: &[&TokenDef] = &[\n");
    for entry in entries {
        match entry.body {
            Body::HandWritten => {
                let _ = writeln!(out, "    &tokens::{},", entry.constant);
            }
            Body::Generated { .. } => {
                let _ = writeln!(out, "    &{},", entry.constant);
            }
        }
    }
    out.push_str("];\n\n");

    // A view, not a second source of ids: the ledger is `ALL` and nothing
    // else. What this answers is the one question `ALL` cannot — which half
    // of the table an entry came from — and two tests need it, because a
    // token a person wrote must name a picture and one a reader wrote has
    // none to name until somebody chooses it.
    out.push_str(
        "/// The entries a reader wrote, as a subset of [`ALL`] — never a second\n\
         /// table of ids.\n",
    );
    out.push_str("pub static GENERATED: &[&TokenDef] = &[\n");
    for entry in entries {
        if matches!(entry.body, Body::Generated { .. }) {
            let _ = writeln!(out, "    &{},", entry.constant);
        }
    }
    out.push_str("];\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generated(constant: &str, literal: &str) -> TokenBody {
        TokenBody {
            constant: constant.to_string(),
            doc: "a token.".to_string(),
            literal: literal.to_string(),
            modules: vec!["creature".to_string()],
        }
    }

    /// The round trip is what makes the table append-only: a run learns the
    /// ids it has already given out by reading the file it is about to
    /// rewrite, so a row lost between the two would be an id handed out
    /// twice.
    #[test]
    fn what_the_ledger_writes_is_what_it_reads_back() {
        let entries = vec![
            Entry {
                constant: "ALLY_1_1_WHITE".into(),
                body: Body::HandWritten,
            },
            Entry {
                constant: "SOLDIER_1_1_WHITE".into(),
                body: Body::Generated {
                    doc: "1/1 white Soldier.".into(),
                    literal: "TokenDef {\n    name: \"Soldier\",\n    \
                              subtypes: &[creature::SOLDIER],\n    \
                              ..TokenDef::DEFAULT\n}"
                        .into(),
                    modules: vec!["creature".into()],
                },
            },
        ];
        let rendered = render(&entries);
        assert_eq!(parse(&rendered).expect("readable"), entries);
        assert_eq!(render(&parse(&rendered).expect("readable")), rendered);
    }

    /// A ledger with nothing generated in it needs none of the imports a
    /// definition would, and an unused import is a warning in a file nobody
    /// may edit.
    #[test]
    fn a_ledger_of_hand_written_tokens_imports_nothing_it_does_not_use() {
        let entries = vec![Entry {
            constant: "TREASURE".into(),
            body: Body::HandWritten,
        }];
        let rendered = render(&entries);
        assert!(!rendered.contains("ColorSet"), "{rendered}");
        assert!(!rendered.contains("subtypes::"), "{rendered}");
        assert!(rendered.contains("    &tokens::TREASURE,\n"), "{rendered}");
        assert!(
            rendered.contains("pub static GENERATED: &[&TokenDef] = &[\n];\n"),
            "a ledger with nothing generated in it has an empty view: {rendered}"
        );
        assert_eq!(parse(&rendered).expect("readable"), entries);
    }

    /// Assignment appends and does nothing else. A token already in the
    /// table keeps the id it has, whatever order this run happened to read
    /// the two halves in.
    #[test]
    fn assignment_only_ever_appends() {
        let existing = vec![
            Entry {
                constant: "TREASURE".into(),
                body: Body::HandWritten,
            },
            Entry {
                constant: "SOLDIER_1_1_WHITE".into(),
                body: Body::Generated {
                    doc: "1/1 white Soldier.".into(),
                    literal: "TokenDef { a }".into(),
                    modules: vec![],
                },
            },
        ];
        let hand = vec!["TREASURE".to_string(), "CLUE".to_string()];
        let fresh = [
            generated("SOLDIER_1_1_WHITE", "TokenDef { a }"),
            generated("BIRD_1_1_WHITE_FLYING", "TokenDef { b }"),
        ];
        let out = assign(existing, &hand, &fresh).expect("appends");
        let names: Vec<&str> = out.iter().map(|e| e.constant.as_str()).collect();
        assert_eq!(
            names,
            [
                "TREASURE",
                "SOLDIER_1_1_WHITE",
                "CLUE",
                "BIRD_1_1_WHITE_FLYING"
            ]
        );
    }

    /// Two scripts describing the **same** token are one row, not two.
    ///
    /// The other half of the rule above, and the one that does not announce
    /// itself: two definitions that *disagree* stop the run, two that agree
    /// are the same permanent read twice. A ledger that files both hands the
    /// second copy an id nothing will ever create, and every id after it
    /// moves — so this is the cheap half of an append-only table going wrong.
    #[test]
    fn one_token_read_from_two_scripts_is_one_row() {
        let twice = [
            generated("ZOMBIE_2_2_BLACK", "TokenDef { z }"),
            generated("ZOMBIE_2_2_BLACK", "TokenDef { z }"),
        ];
        let out = assign(Vec::new(), &[], &twice).expect("agreeing twins are one token");
        let names: Vec<&str> = out.iter().map(|e| e.constant.as_str()).collect();
        assert_eq!(names, ["ZOMBIE_2_2_BLACK"]);
        // And the hand-written half is subject to the same arithmetic: a name
        // declared twice in `tokens.rs` is one token there too.
        let hand = ["FOOD".to_string(), "FOOD".to_string()];
        let out = assign(Vec::new(), &hand, &[]).expect("one name is one row");
        assert_eq!(out.len(), 1, "a name declared twice took two ids");
    }

    /// Two definitions at one name would be two tokens at one id, and the
    /// second of them reaches the table wearing the first's picture. The run
    /// stops rather than picking one.
    #[test]
    fn two_tokens_at_one_name_stop_the_run() {
        let existing = vec![Entry {
            constant: "SOLDIER_1_1_WHITE".into(),
            body: Body::Generated {
                doc: "1/1 white Soldier.".into(),
                literal: "TokenDef { a }".into(),
                modules: vec![],
            },
        }];
        let clash = [generated("SOLDIER_1_1_WHITE", "TokenDef { b }")];
        assert_eq!(
            assign(existing, &[], &clash),
            Err(LedgerError::Collision("SOLDIER_1_1_WHITE".into()))
        );

        let pair = [
            generated("BIRD_1_1_WHITE", "TokenDef { a }"),
            generated("BIRD_1_1_WHITE", "TokenDef { b }"),
        ];
        assert_eq!(
            assign(Vec::new(), &[], &pair),
            Err(LedgerError::Collision("BIRD_1_1_WHITE".into()))
        );
    }

    /// And the same definition wrapped by the formatter is **not** two
    /// tokens. One side of the comparison came back out of the written file
    /// and the other was just built, so a literal long enough for rustfmt to
    /// break disagreed with itself — which is what the first token to carry
    /// an ability did on the second `codegen` run of the day it was written.
    ///
    /// Asserted beside the collision it must not become, because a
    /// comparison loose enough to forgive a line break is one that could
    /// forgive a difference, and only the pair says it does not.
    #[test]
    fn the_formatter_breaking_a_line_is_not_a_second_token() {
        let written = "TokenDef {\n    name: \"Eldrazi Spawn\",\n    \
                       abilities: &[mana_ability!(\n        cost!(SacrificeSelf),\n        \
                       &[Effect::mana(ManaColor::Colorless, 1)]\n    )],\n}";
        let built = "TokenDef {\n    name: \"Eldrazi Spawn\",\n    \
                     abilities: &[mana_ability!(cost!(SacrificeSelf), \
                     &[Effect::mana(ManaColor::Colorless, 1)])],\n}";
        assert_ne!(written, built, "the two spellings really do differ");
        let existing = vec![Entry {
            constant: "ELDRAZI_SPAWN_0_1".into(),
            body: Body::Generated {
                doc: "0/1 colorless Eldrazi Spawn.".into(),
                literal: written.into(),
                modules: vec!["creature".into()],
            },
        }];
        let again = [generated("ELDRAZI_SPAWN_0_1", built)];
        let out = assign(existing, &[], &again).expect("one token, two spellings");
        assert_eq!(out.len(), 1);
    }

    /// A hand-written token that has been deleted takes its id with it, and
    /// nothing may reuse the number. Saying so is the whole point of a
    /// ledger; writing a file that names a constant which no longer exists
    /// would only move the failure to the compiler.
    #[test]
    fn a_hand_written_token_that_has_gone_is_a_decision_for_a_person() {
        let existing = vec![Entry {
            constant: "TREASURE".into(),
            body: Body::HandWritten,
        }];
        assert_eq!(
            assign(existing, &[], &[]),
            Err(LedgerError::Orphan("TREASURE".into()))
        );
    }

    /// A definition with no row has no id, and keeping it would let a later
    /// run give it one out of order — so the file is refused rather than
    /// read as if the row were simply missing.
    #[test]
    fn a_definition_the_table_does_not_name_refuses_the_file() {
        let text = "pub static ALL: &[&TokenDef] = &[\n];\n";
        assert_eq!(parse(text), Ok(Vec::new()));

        let stray = "/// a token.\n\
                     pub static SOLDIER_1_1_WHITE: TokenDef = TokenDef {\n\
                     };\n\
                     pub static ALL: &[&TokenDef] = &[\n];\n";
        assert!(matches!(parse(stray), Err(LedgerError::Unreadable(_))));

        assert!(matches!(
            parse("// nothing"),
            Err(LedgerError::Unreadable(_))
        ));
    }
}
