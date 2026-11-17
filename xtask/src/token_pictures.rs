//! `xtask token-pictures`: choose a printed token card for each generated
//! token, by exact match only (`docs/client.md` §"Where a token's picture
//! comes from").
//!
//! A token a reader wrote out of the card-script reference has no printing;
//! its `scryfall_id` stays empty in `generated_tokens.rs`, which only codegen
//! writes. This command holds every such token against the printed token
//! cards MTGJSON lists beside each product's deck (the same
//! `AllDeckFiles.tar.gz` `decks-import` reads, never downloaded here) and
//! writes the ones that match into
//! `crates/cards/src/tokens/printing_rows.rs`.
//!
//! A match is the name, the card types, supertypes and subtypes, the colours,
//! power and toughness, and a rules box that says nothing but the token's
//! own keywords (reminder text aside). A token with abilities of its own is
//! never matched: nothing in the registry says them in words to compare, and
//! a wrong picture is worse than the text face. Among printings that all
//! match, the newest product's wins. Every token left without one is listed
//! with the reason.
//!
//! The local card catalog (`baylee-catalog`) is not a source: it keeps no
//! colours, so it cannot tell a green Saproling from a black one.

use crate::{BTreeMap, Path, write_verbatim};
use anyhow::{Context, bail};
use baylee_cards::dsl::{KeywordSet, TokenDef};
use baylee_cards::generated_tokens::GENERATED;
#[cfg(test)]
use baylee_cards::tokens::ALL;
use baylee_cards::tokens::token_id;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde_json::Value;

/// The table this command writes, from the repository root.
pub(crate) const ROWS_FILE: &str = "crates/cards/src/tokens/printing_rows.rs";

/// One printed token card, as the source describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Printed {
    pub(crate) name: String,
    pub(crate) layout: String,
    /// The whole type line, "Token Creature — Saproling".
    pub(crate) type_line: String,
    /// Colour letters, `W U B R G`.
    pub(crate) colors: Vec<String>,
    pub(crate) power: Option<String>,
    pub(crate) toughness: Option<String>,
    pub(crate) loyalty: Option<String>,
    pub(crate) defense: Option<String>,
    pub(crate) text: String,
    pub(crate) scryfall_id: String,
    pub(crate) set: String,
    pub(crate) number: String,
    /// The release date of the product the printing came in, ISO.
    pub(crate) released: String,
}

/// A chosen printing for one token.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub(crate) token: u16,
    pub(crate) name: &'static str,
    pub(crate) printed: Printed,
}

/// What became of one generated token.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Matched(Box<Choice>),
    Unmatched {
        token: u16,
        name: &'static str,
        why: String,
    },
}

/// Every printed token card in an MTGJSON deck archive, one per Scryfall id.
fn printed_in_archive(archive: &Path) -> anyhow::Result<Vec<Printed>> {
    let mut by_id: BTreeMap<String, Printed> = BTreeMap::new();
    for (file, bytes) in crate::precons::read_archive(archive)? {
        let json: Value =
            serde_json::from_slice(&bytes).with_context(|| format!("{file}: not JSON"))?;
        let released = json["data"]["releaseDate"].as_str().unwrap_or_default();
        let Some(tokens) = json["data"]["tokens"].as_array() else {
            continue;
        };
        for token in tokens {
            let Some(printed) = printed_from_mtgjson(token, released) else {
                continue;
            };
            match by_id.get(&printed.scryfall_id) {
                Some(kept) if kept.released >= printed.released => {}
                _ => {
                    by_id.insert(printed.scryfall_id.clone(), printed);
                }
            }
        }
    }
    Ok(by_id.into_values().collect())
}

/// One MTGJSON token record, if it carries a Scryfall id.
pub(crate) fn printed_from_mtgjson(token: &Value, released: &str) -> Option<Printed> {
    let text = |key: &str| token[key].as_str().map(str::to_string);
    let scryfall_id = token["identifiers"]["scryfallId"].as_str()?.to_string();
    Some(Printed {
        name: text("name")?,
        layout: text("layout").unwrap_or_default(),
        type_line: text("type").unwrap_or_default(),
        colors: token["colors"]
            .as_array()
            .map_or_default(|c| {
                c.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            }),
        power: text("power"),
        toughness: text("toughness"),
        loyalty: text("loyalty"),
        defense: text("defense"),
        text: text("text").unwrap_or_default(),
        scryfall_id,
        set: text("setCode").unwrap_or_default().to_ascii_lowercase(),
        number: text("number").unwrap_or_default(),
        released: released.to_string(),
    })
}

/// A colour letter as the colour it names.
fn color_of(letter: &str) -> Option<Color> {
    Some(match letter {
        "W" => Color::White,
        "U" => Color::Blue,
        "B" => Color::Black,
        "R" => Color::Red,
        "G" => Color::Green,
        _ => return None,
    })
}

/// A printed number as the registry stores one; `*` and `X` are no number.
fn number_of(printed: Option<&str>) -> Result<Option<i16>, ()> {
    match printed {
        None => Ok(None),
        Some(n) => n.parse().map(Some).map_err(|_| ()),
    }
}

/// Why `printed` is not `token`, or `Ok` when every characteristic agrees.
pub(crate) fn compare(token: &TokenDef, printed: &Printed) -> Result<(), String> {
    if printed.layout != "token" {
        return Err(format!("layout `{}`, not a plain token", printed.layout));
    }
    if printed.name != token.name {
        return Err(format!("named {}", printed.name));
    }
    let Some(line) = printed.type_line.strip_prefix("Token ") else {
        return Err(format!(
            "type line `{}` is not a token's",
            printed.type_line
        ));
    };
    let (front, back) = line.split_once(" \u{2014} ").unwrap_or((line, ""));
    let mut types = TypeSet::EMPTY;
    let mut supertypes = SupertypeSet::EMPTY;
    for word in front.split_whitespace() {
        if let Some(t) = TypeSet::from_word(word) {
            types = types.union(t);
        } else if let Some(s) = SupertypeSet::from_word(word) {
            supertypes = supertypes.union(s);
        } else {
            return Err(format!("type word `{word}` is unknown here"));
        }
    }
    let mut subtype_set = SubtypeSet::EMPTY;
    for word in back.split_whitespace() {
        let id = subtypes::by_name(word).ok_or_else(|| format!("subtype `{word}` is unknown"))?;
        subtype_set.insert(id);
    }
    if types != token.types || supertypes != token.supertypes {
        return Err(format!("type line `{}`", printed.type_line));
    }
    if subtype_set != SubtypeSet::from_slice(token.subtypes) {
        return Err(format!("subtypes `{back}`"));
    }
    let mut colors = Vec::new();
    for letter in &printed.colors {
        colors.push(color_of(letter).ok_or_else(|| format!("colour `{letter}`"))?);
    }
    if ColorSet::from_slice(&colors) != token.colors {
        return Err(format!("colours {}", printed.colors.join("")));
    }
    let body = (
        number_of(printed.power.as_deref()),
        number_of(printed.toughness.as_deref()),
    );
    if body != (Ok(token.power), Ok(token.toughness)) {
        return Err(format!(
            "body {}/{}",
            printed.power.as_deref().unwrap_or("-"),
            printed.toughness.as_deref().unwrap_or("-")
        ));
    }
    let counters = (
        printed.loyalty.as_deref().map(str::parse::<u16>),
        printed.defense.as_deref().map(str::parse::<u16>),
    );
    if counters != (token.loyalty.map(Ok), token.defense.map(Ok)) {
        return Err("loyalty or defense".to_string());
    }
    let said = keywords_said(&printed.text)?;
    if said != token.keywords {
        return Err(format!(
            "rules text `{}`",
            printed.text.replace('\n', " / ")
        ));
    }
    Ok(())
}

/// The keywords a rules box says, if it says nothing else.
///
/// Reminder text aside, every line is a list of keywords joined by commas or
/// one of the sentences [`KeywordSet::from_printed`] knows; any other words
/// are an ability the comparison cannot hold against the registry.
fn keywords_said(text: &str) -> Result<KeywordSet, String> {
    let mut said = KeywordSet::EMPTY;
    for line in crate::strip_reminders(text).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(k) = KeywordSet::from_printed(line) {
            said = said.union(k);
            continue;
        }
        for word in line.split(", ") {
            let k = KeywordSet::from_printed(word)
                .ok_or_else(|| format!("rules text says more than keywords: `{line}`"))?;
            said = said.union(k);
        }
    }
    Ok(said)
}

/// Every generated token's outcome against `printed`, in ledger order.
pub(crate) fn choose(printed: &[Printed]) -> Vec<Outcome> {
    let mut by_name: BTreeMap<&str, Vec<&Printed>> = BTreeMap::new();
    for p in printed {
        by_name.entry(p.name.as_str()).or_default().push(p);
    }
    let mut generated: Vec<(u16, &'static TokenDef)> =
        GENERATED.iter().map(|t| (token_id(t), *t)).collect();
    generated.sort_by_key(|(id, _)| *id);
    generated
        .into_iter()
        .map(|(id, token)| outcome(id, token, by_name.get(token.name).map(Vec::as_slice)))
        .collect()
}

fn outcome(id: u16, token: &'static TokenDef, named: Option<&[&Printed]>) -> Outcome {
    let unmatched = |why: String| Outcome::Unmatched {
        token: id,
        name: token.name,
        why,
    };
    if !token.abilities.is_empty() {
        return unmatched(
            "has abilities of its own; the registry has no words to compare them with".to_string(),
        );
    }
    let Some(named) = named else {
        return unmatched("no printed token of that name in the source".to_string());
    };
    let mut fits: Vec<&Printed> = Vec::new();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for p in named {
        match compare(token, p) {
            Ok(()) => fits.push(p),
            Err(why) => *reasons.entry(why).or_default() += 1,
        }
    }
    // Newest product first; then a fixed order, so a rerun writes the same.
    fits.sort_by(|a, b| {
        b.released
            .cmp(&a.released)
            .then_with(|| a.set.cmp(&b.set))
            .then_with(|| a.number.cmp(&b.number))
            .then_with(|| a.scryfall_id.cmp(&b.scryfall_id))
    });
    match fits.first() {
        Some(best) => Outcome::Matched(Box::new(Choice {
            token: id,
            name: token.name,
            printed: (*best).clone(),
        })),
        None => unmatched(format!(
            "{} printed {}, none matches: {}",
            named.len(),
            token.name,
            reasons
                .iter()
                .map(|(why, n)| format!("{why} ×{n}"))
                .collect::<Vec<_>>()
                .join("; ")
        )),
    }
}

/// The table's source text for the chosen rows.
pub(crate) fn render(choices: &[&Choice], source: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "// Written by `cargo xtask token-pictures --write` — change it through the\n\
         // tool, never by hand (`docs/client.md` §\"Where a token's picture comes\n\
         // from\").\n//\n\
         // Source: the token printings MTGJSON's `AllDeckFiles.tar.gz` lists beside\n\
         // each product's deck (MIT; `NOTICE`), read as an automated lookup from\n\
         // the local cache. Only the Scryfall identifier, set code and collector\n\
         // number are kept: no image, rules text or other field.\n",
    );
    let _ = writeln!(out, "// MTGJSON build: {source}\n");
    out.push_str("&[\n");
    for choice in choices {
        let p = &choice.printed;
        let _ = writeln!(
            out,
            "    TokenPrinting {{ token: {}, name: {:?}, scryfall_id: {:?}, set: {:?}, number: {:?} }},",
            choice.token, choice.name, p.scryfall_id, p.set, p.number
        );
    }
    out.push_str("]\n");
    out
}

/// The archive's MTGJSON build, from the first file that names one.
fn source_build(archive: &Path) -> anyhow::Result<String> {
    for (_, bytes) in crate::precons::read_archive(archive)? {
        if let Ok(json) = serde_json::from_slice::<Value>(&bytes)
            && let Some(version) = json["meta"]["version"].as_str()
        {
            return Ok(version.to_string());
        }
    }
    Ok("?".to_string())
}

/// `xtask token-pictures`: report, and with `write` rewrite the table.
///
/// # Errors
/// When the archive is missing or unreadable, or the table cannot be written.
pub(crate) fn run(root: &Path, archive: Option<&Path>, write: bool) -> anyhow::Result<()> {
    let path = archive.map_or_else(
        || root.join(crate::precons::ARCHIVE_CACHE),
        Path::to_path_buf,
    );
    if !path.exists() {
        bail!(
            "{} is missing: run `cargo xtask decks-import` once to cache it, or pass --archive",
            path.display()
        );
    }
    let printed = printed_in_archive(&path)?;
    let outcomes = choose(&printed);
    let choices: Vec<&Choice> = outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::Matched(c) => Some(&**c),
            Outcome::Unmatched { .. } => None,
        })
        .collect();
    for outcome in &outcomes {
        match outcome {
            Outcome::Matched(c) => println!(
                "matched   {:>4} {:<28} {} {}/{} ({})",
                c.token,
                c.name,
                c.printed.scryfall_id,
                c.printed.set,
                c.printed.number,
                c.printed.released
            ),
            Outcome::Unmatched { token, name, why } => {
                println!("unmatched {token:>4} {name:<28} {why}");
            }
        }
    }
    println!(
        "token-pictures: {} printed token cards read; {} of {} generated tokens matched, {} left to their text face",
        printed.len(),
        choices.len(),
        outcomes.len(),
        outcomes.len() - choices.len()
    );
    let content = render(&choices, &source_build(&path)?);
    let mut changed = Vec::new();
    write_verbatim(!write, &root.join(ROWS_FILE), &content, &mut changed)?;
    if !write && !changed.is_empty() {
        println!("{ROWS_FILE} would change; rerun with --write");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printed(
        name: &str,
        type_line: &str,
        colors: &[&str],
        pt: (&str, &str),
        text: &str,
    ) -> Printed {
        Printed {
            name: name.into(),
            layout: "token".into(),
            type_line: type_line.into(),
            colors: colors.iter().map(|c| (*c).to_string()).collect(),
            power: Some(pt.0.into()),
            toughness: Some(pt.1.into()),
            loyalty: None,
            defense: None,
            text: text.into(),
            scryfall_id: "4fb8f4f1-6372-41ea-802c-4b952a78a08b".into(),
            set: "tddh".into(),
            number: "2".into(),
            released: "2011-09-02".into(),
        }
    }

    fn generated(name: &str, flying: bool) -> &'static TokenDef {
        GENERATED
            .iter()
            .copied()
            .find(|t| {
                t.name == name
                    && t.abilities.is_empty()
                    && t.keywords.contains(KeywordSet::FLYING) == flying
            })
            .expect("the ledger holds it")
    }

    /// The Bat the ledger holds is the black 1/1 flier; the printed card that
    /// says so, reminder text and all, is its picture.
    #[test]
    fn a_token_matches_the_card_that_prints_every_characteristic() {
        let bat = generated("Bat", true);
        let card = printed(
            "Bat",
            "Token Creature \u{2014} Bat",
            &["B"],
            ("1", "1"),
            "Flying (This creature can't be blocked except by creatures with flying or reach.)",
        );
        assert_eq!(compare(bat, &card), Ok(()));
    }

    /// Each characteristic on its own keeps a picture off the token: a wrong
    /// picture is worse than the text face.
    #[test]
    fn one_differing_characteristic_is_no_match() {
        let bat = generated("Bat", true);
        let good = printed(
            "Bat",
            "Token Creature \u{2014} Bat",
            &["B"],
            ("1", "1"),
            "Flying",
        );
        let mut wrong = Vec::new();
        let mut card = good.clone();
        card.colors = vec!["R".into()];
        wrong.push(card);
        let mut card = good.clone();
        card.power = Some("2".into());
        wrong.push(card);
        let mut card = good.clone();
        card.text = "Flying, haste".into();
        wrong.push(card);
        let mut card = good.clone();
        card.text = "Flying\nWhen this token dies, draw a card.".into();
        wrong.push(card);
        let mut card = good.clone();
        card.type_line = "Token Artifact Creature \u{2014} Bat".into();
        wrong.push(card);
        let mut card = good.clone();
        card.type_line = "Token Creature \u{2014} Bat Spirit".into();
        wrong.push(card);
        let mut card = good.clone();
        card.layout = "double_faced_token".into();
        wrong.push(card);
        let mut card = good;
        card.power = Some("*".into());
        wrong.push(card);
        for card in wrong {
            assert!(compare(bat, &card).is_err(), "{card:?}");
        }
    }

    /// Of several printings that all match, the newest product's is chosen;
    /// and a token with abilities is never matched, whatever is printed.
    #[test]
    fn the_newest_match_wins_and_abilities_are_never_guessed() {
        let mut old = printed(
            "Bat",
            "Token Creature \u{2014} Bat",
            &["B"],
            ("1", "1"),
            "Flying",
        );
        let mut new = old.clone();
        new.released = "2024-01-01".into();
        new.scryfall_id = "00000000-0000-7000-8000-000000000001".into();
        old.released = "2010-01-01".into();
        let outcomes = choose(&[old, new.clone()]);
        let bat = token_id(generated("Bat", true));
        let chosen = outcomes.iter().find_map(|o| match o {
            Outcome::Matched(c) if c.token == bat => Some(c),
            _ => None,
        });
        assert_eq!(chosen.map(|c| &c.printed), Some(&new));
        assert!(outcomes.iter().all(|o| match o {
            Outcome::Matched(c) => ALL[usize::from(c.token)].abilities.is_empty(),
            Outcome::Unmatched { .. } => true,
        }));
    }

    /// The written table is what the client compiles: one row per choice.
    #[test]
    fn the_table_names_each_row_once() {
        let card = printed(
            "Bat",
            "Token Creature \u{2014} Bat",
            &["B"],
            ("1", "1"),
            "Flying",
        );
        let choice = Choice {
            token: 7,
            name: "Bat",
            printed: card,
        };
        let text = render(&[&choice], "5.x");
        assert!(text.contains(
            "TokenPrinting { token: 7, name: \"Bat\", scryfall_id: \"4fb8f4f1-6372-41ea-802c-4b952a78a08b\", set: \"tddh\", number: \"2\" },"
        ));
        assert!(text.trim_end().ends_with(']'));
    }
}
