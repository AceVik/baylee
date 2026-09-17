//! Reads a reference **token** script into a [`TokenDef`] literal.
//!
//! A token script is a card script with almost everything taken out — a name,
//! a colour word, a type line and a power/toughness — so it is read here
//! rather than by [`scriptgen`](crate::scriptgen), which is about abilities.
//! The same rule holds as everywhere else in this crate: **one unread line
//! and the token is refused**, because a token the engine creates with the
//! wrong colour or a missing keyword is worse than a card that stays a stub.
//!
//! # Why a token needs generating at all
//!
//! `Effect::CreateToken` takes a `&'static TokenDef`, and the id the engine
//! stamps on the object it makes — the one a client keys token art off — is
//! that definition's place in a registry. So a card that makes a Soldier
//! cannot carry its own literal: it would have no id. Fourteen tokens are
//! written by hand in `baylee_cards::tokens` for the cards that were finished
//! by hand; the reference carries 852 token scripts and its cards name 819 of
//! them, and reading those is what lets the transcoder write a card that
//! creates one.
//!
//! # The constant's name is ours
//!
//! A generated token is named after what it *is* — `SOLDIER_1_1_WHITE`,
//! `BIRD_1_1_WHITE_FLYING` — and never after the reference's own file stem.
//! Two reasons, and the second is the load-bearing one. The reference corpus
//! is read as an automated lookup and never copied into this repo (`NOTICE`),
//! and a stem stored in a generated file would be a copy. And a name built
//! from the token's characteristics is a name the *registry* owns, so the
//! same token read from another source later is the same constant.
//!
//! The name is everything the reader distinguished: what the token is called,
//! the supertypes and card types beyond `Creature` it carries, its size, its
//! colours and its keyword bits. Each of those segments was paid for — eight
//! pairs of the reference's tokens collide without the type words alone, a
//! 1/1 white Soldier against a 1/1 white *enchantment* Soldier — and a
//! collision is not a cosmetic problem here: two permanents sharing a
//! constant share an id, and one of them reaches the table wearing the
//! other's picture. Over the reference's 852 token scripts the rule reads
//! 586 in full and gives them 586 distinct names, with no name claimed by
//! two different definitions.
//!
//! The evidence that it is the naming a person reaches for is that it is the
//! naming a person reached for: of the nine hand-written tokens this reader
//! can read at all, eight come out spelled exactly as `baylee_cards::tokens`
//! already spells them — `ALLY_1_1_WHITE`, `ANGEL_4_4_WHITE_FLYING`,
//! `SHAPESHIFTER_2_2_BLUE_CHANGELING`. The ninth is `CONSTRUCT_0_0`, an
//! *artifact* creature whose hand-written name leaves the artifact out; the
//! rule writes `CONSTRUCT_ARTIFACT_0_0`, because leaving it out is exactly
//! what lets two tokens share a name.

use crate::catalog::SubtypeCatalogs;
use baylee_core::types::{SupertypeSet, TypeSet};
use std::fmt::Write as _;

/// One token read in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBody {
    /// The `SCREAMING_SNAKE` constant this token is filed under.
    pub constant: String,
    /// A one-line doc comment for it ("1/1 white Soldier.").
    pub doc: String,
    /// The `TokenDef { … }` literal, with its `..TokenDef::DEFAULT` tail.
    pub literal: String,
    /// Subtype modules the literal names (`creature`, `artifact`, …), so the
    /// generated file imports exactly the ones it uses.
    pub modules: Vec<String>,
}

/// The two lines a token script carries that say nothing this reads.
///
/// `ManaCost` is always "no cost" and `Oracle` is the printed reminder text,
/// which this side has no use for — a token has no printing to hold a header
/// against. Every other head refuses the token, and each of them is a real
/// shape in the reference: `Text` is rules text ("this creature is all
/// colors"), `S`/`T`/`A`/`SVar`/`R`/`Loyalty` are abilities, and
/// `AlternateMode` is a second face.
const IGNORED: &[&str] = &["ManaCost", "Oracle"];

/// The card types a token may be made of, and the constant each one is.
///
/// A table rather than a `match` on [`TypeSet::from_word`], because that
/// function knows fifteen types and this writes seven: a `match` with a
/// catch-all arm would have filed a Dungeon as `TypeSet::EMPTY` and called it
/// read. Anything `from_word` knows and this does not is refused by name
/// below, so the two lists cannot drift into silence.
const TYPES: &[(&str, &str)] = &[
    ("Artifact", "TypeSet::ARTIFACT"),
    ("Creature", "TypeSet::CREATURE"),
    ("Enchantment", "TypeSet::ENCHANTMENT"),
    ("Land", "TypeSet::LAND"),
    ("Planeswalker", "TypeSet::PLANESWALKER"),
    ("Battle", "TypeSet::BATTLE"),
    ("Kindred", "TypeSet::KINDRED"),
];

/// The supertypes a token may carry. `World`, `Ongoing` and `Host` are the
/// three [`SupertypeSet`] knows and this does not.
const SUPERTYPES: &[(&str, &str)] = &[
    ("Basic", "SupertypeSet::BASIC"),
    ("Legendary", "SupertypeSet::LEGENDARY"),
    ("Snow", "SupertypeSet::SNOW"),
];

/// Reads a token script, or refuses it.
///
/// `None` is an honest refusal and the commonest outcome after success: 266
/// of the reference's 852 token scripts are refused, 184 of them for carrying
/// an ability and 62 for a size the card that makes the token computes. A
/// token whose ability this cannot write is one the engine would put on the
/// battlefield inert, which is worse than a card that stays a stub.
#[must_use]
pub fn read(text: &str, cats: &SubtypeCatalogs) -> Option<TokenBody> {
    let (mut name, mut colors, mut types, mut pt) = (None, None, None, None);
    let mut keywords: Vec<&'static str> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (head, rest) = line.split_once(':')?;
        match head {
            "Name" => name = Some(rest.trim().to_string()),
            "Colors" => colors = Some(rest.trim().to_string()),
            "Types" => types = Some(rest.trim().to_string()),
            "PT" => pt = Some(rest.trim().to_string()),
            "K" => keywords.push(crate::scriptgen::keyword_const_of(rest)?),
            _ if IGNORED.contains(&head) => {}
            _ => return None,
        }
    }

    // The script's line order is the script's, not the token's: the corpus
    // writes `Defender,Flying` beside `Flying,Vigilance` and `red,white`
    // beside `green,blue`. A name built from the order the lines happen to
    // arrive in is two names for one token — two ids, in a table that may
    // only ever grow. So both lists are put in a canonical order before
    // either the name or the literal is built: keywords alphabetically,
    // colours in WUBRG, which is the order Magic itself prints them.
    keywords.sort_unstable();
    keywords.dedup();

    // CR 111.2: a token has the characteristics the effect that created it
    // states, and the type line is the one field none of them can omit.
    let name = name?;
    let types = type_line(types.as_deref()?, cats)?;
    let (power, toughness) = match (pt.as_deref(), types.is_creature) {
        // `PT:*/*` is a token whose size is computed by the card that makes
        // it, which is a rule rather than a definition.
        (Some(pt), true) => {
            let (p, t) = pt.split_once('/')?;
            let p = p.trim().parse::<i16>().ok()?;
            let t = t.trim().parse::<i16>().ok()?;
            (Some(p), Some(t))
        }
        (None, false) => (None, None),
        // A creature with no size, or a size on something that is not a
        // creature: the two halves of the script disagree, and deciding
        // which of them is right is not this reader's business.
        _ => return None,
    };
    let colors = match colors.as_deref() {
        Some(raw) => Some(color_line(raw)?),
        None => None,
    };

    let mut literal = String::with_capacity(256);
    literal.push_str("TokenDef {\n");
    let _ = writeln!(literal, "    name: {:?},", token_name(&name));
    if let Some(colors) = &colors
        && !colors.words.is_empty()
    {
        let _ = writeln!(literal, "    colors: {},", colors.expr);
    }
    let _ = writeln!(literal, "    types: {},", types.set);
    if let Some(supertypes) = &types.supertypes {
        let _ = writeln!(literal, "    supertypes: {supertypes},");
    }
    if !types.subtypes.is_empty() {
        let _ = writeln!(literal, "    subtypes: &[{}],", types.subtypes.join(", "));
    }
    if let (Some(p), Some(t)) = (power, toughness) {
        let _ = writeln!(literal, "    power: Some({p}),");
        let _ = writeln!(literal, "    toughness: Some({t}),");
    }
    if !keywords.is_empty() {
        let _ = writeln!(literal, "    keywords: {},", union_of(&keywords));
    }
    literal.push_str("    ..TokenDef::DEFAULT\n}");

    Some(TokenBody {
        constant: constant_name(
            &name,
            &types.qualifiers,
            power,
            toughness,
            colors.as_ref(),
            &keywords,
        ),
        doc: doc_line(
            &name,
            &types.qualifiers,
            power,
            toughness,
            colors.as_ref(),
            &keywords,
        ),
        literal,
        modules: types.modules,
    })
}

/// A colour word list as a `ColorSet` expression plus the words themselves.
struct Colors {
    expr: String,
    words: Vec<&'static str>,
}

/// The five colours in WUBRG, which is the order a card prints them in and
/// the order [`baylee_core::color::ColorSet`] numbers them.
const WUBRG: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

fn color_line(raw: &str) -> Option<Colors> {
    let mut found = [false; 5];
    for word in raw.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        // "colorless" is the absence of colours and not a sixth one, so it
        // adds nothing — and a script naming it beside a real colour is one
        // this reader has misunderstood rather than one it can average out.
        if word.eq_ignore_ascii_case("colorless") {
            if raw.contains(',') {
                return None;
            }
            continue;
        }
        found[WUBRG.iter().position(|c| c.eq_ignore_ascii_case(word))?] = true;
    }
    let words: Vec<&'static str> = WUBRG
        .iter()
        .zip(found)
        .filter_map(|(c, on)| on.then_some(*c))
        .collect();
    let expr = format!(
        "ColorSet::from_slice(&[{}])",
        words
            .iter()
            .map(|w| format!("Color::{w}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Some(Colors { expr, words })
}

/// A token script's type line, read word by word — it prints no em dash, so
/// nothing separates the types from the subtypes but knowing what each word
/// is.
struct TypeLine {
    set: String,
    supertypes: Option<String>,
    subtypes: Vec<String>,
    modules: Vec<String>,
    is_creature: bool,
    /// The type-line words that qualify the token beyond what its name
    /// already says — every supertype, and every card type but `Creature` —
    /// in printed order, for the constant's name, and empty for a token of
    /// one card type and no supertypes.
    ///
    /// Without them eight pairs of the reference's tokens collide: a 1/1
    /// white Soldier and a 1/1 white *enchantment* Soldier are two different
    /// permanents and were one constant.
    qualifiers: Vec<String>,
}

fn type_line(line: &str, cats: &SubtypeCatalogs) -> Option<TypeLine> {
    let (mut types, mut supers, mut subtypes, mut modules) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut qualifiers = Vec::new();
    let mut is_creature = false;
    for word in line.split_whitespace() {
        if let Some((_, konst)) = TYPES.iter().find(|(w, _)| *w == word) {
            if *konst == "TypeSet::CREATURE" {
                is_creature = true;
            } else {
                qualifiers.push(word.to_ascii_uppercase());
            }
            types.push(*konst);
        } else if let Some((_, konst)) = SUPERTYPES.iter().find(|(w, _)| *w == word) {
            qualifiers.push(word.to_ascii_uppercase());
            supers.push(*konst);
        } else if TypeSet::from_word(word).is_some() || SupertypeSet::from_word(word).is_some() {
            // A card type this reader does not write. Refused by name rather
            // than left to the subtype lookup below, which would answer
            // `None` for the same word and hide which of the two questions
            // the token failed.
            return None;
        } else {
            // `subtypes::creature::SOLDIER` — the generated file imports the
            // modules, so the literal names the shorter path.
            let path = cats.const_path(word)?;
            let path = path.strip_prefix("subtypes::")?.to_string();
            let module = path.split("::").next()?.to_string();
            if !modules.contains(&module) {
                modules.push(module);
            }
            subtypes.push(path);
        }
    }
    if types.is_empty() {
        return None;
    }
    // A token of one card type and no supertypes says what it is by being
    // named at all — `TREASURE`, not `TREASURE_ARTIFACT`, and `SOLDIER_1_1`
    // rather than `SOLDIER_CREATURE_1_1`. The segment exists to separate a
    // plain creature from the same creature with a second type on it, and
    // there is nothing to separate here.
    if types.len() == 1 && supers.is_empty() {
        qualifiers.clear();
    }
    Some(TypeLine {
        set: union_of(&types),
        supertypes: (!supers.is_empty()).then(|| union_of(&supers)),
        subtypes,
        modules,
        is_creature,
        qualifiers,
    })
}

fn union_of(bits: &[&str]) -> String {
    let mut out = bits[0].to_string();
    for bit in &bits[1..] {
        let _ = write!(out, ".union({bit})");
    }
    out
}

/// The token's own name, with the reference's bookkeeping suffix removed.
///
/// The corpus files a Soldier under `Name:Soldier Token` because its own
/// interface needs to tell a token from the card that makes one; a token's
/// name in the game is "Soldier" (CR 111.4). 130 of the 853 are named after
/// a person or a thing and carry no suffix at all, and those keep every word.
fn token_name(name: &str) -> &str {
    name.strip_suffix(" Token").unwrap_or(name).trim()
}

/// `SOLDIER_1_1_WHITE` — what the token is, in the registry's own words.
fn constant_name(
    name: &str,
    qualifiers: &[String],
    power: Option<i16>,
    toughness: Option<i16>,
    colors: Option<&Colors>,
    keywords: &[&str],
) -> String {
    let mut out = String::new();
    for ch in token_name(name).chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let mut out = out.trim_end_matches('_').to_string();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        // A name that is not a Rust identifier — the reference has none, and
        // a constant called `2_2_GREEN` would not compile if it ever did.
        out.insert_str(0, "TOKEN_");
    }
    for word in qualifiers {
        let _ = write!(out, "_{word}");
    }
    if let (Some(p), Some(t)) = (power, toughness) {
        let _ = write!(out, "_{}_{}", pt_word(p), pt_word(t));
    }
    for word in colors.map(|c| c.words.as_slice()).unwrap_or_default() {
        let _ = write!(out, "_{}", word.to_ascii_uppercase());
    }
    for keyword in keywords {
        let _ = write!(out, "_{}", keyword.trim_start_matches("KeywordSet::"));
    }
    out
}

/// A power or toughness inside a constant's name, where a minus sign is not a
/// character a Rust identifier may carry.
fn pt_word(n: i16) -> String {
    if n < 0 {
        format!("MINUS{}", n.unsigned_abs())
    } else {
        n.to_string()
    }
}

/// "1/1 white Soldier with flying.", for the constant's doc comment.
fn doc_line(
    name: &str,
    qualifiers: &[String],
    power: Option<i16>,
    toughness: Option<i16>,
    colors: Option<&Colors>,
    keywords: &[&str],
) -> String {
    let mut out = String::new();
    if let (Some(p), Some(t)) = (power, toughness) {
        let _ = write!(out, "{p}/{t} ");
    }
    match colors.map(|c| c.words.as_slice()).unwrap_or_default() {
        [] => out.push_str("colorless "),
        words => {
            for word in words {
                let _ = write!(out, "{} ", word.to_ascii_lowercase());
            }
        }
    }
    for word in qualifiers {
        let _ = write!(out, "{} ", word.to_ascii_lowercase());
    }
    out.push_str(token_name(name));
    if !keywords.is_empty() {
        let words: Vec<String> = keywords
            .iter()
            .map(|k| {
                k.trim_start_matches("KeywordSet::")
                    .to_ascii_lowercase()
                    .replace('_', " ")
            })
            .collect();
        let _ = write!(out, " with {}", words.join(" and "));
    }
    out.push('.');
    out
}

/// The reference's token scripts, by the stem a card's `TokenScript$` names.
///
/// The sibling of [`crate::scriptgen::ScriptLookup`], and deliberately not
/// folded into it: a card script is found by the card's printed **name** and
/// a token script by a stem that is printed nowhere at all, so the two
/// indexes answer different questions over different directories.
///
/// The directory is derived from the cardsfolder rather than named on its
/// own, because both come out of one checkout of the reference — a second
/// path to set is a second thing to get wrong, and a token index pointed at
/// last month's copy would hand a card a definition the ledger never saw.
pub struct TokenLookup {
    root: std::path::PathBuf,
    stems: std::collections::BTreeSet<String>,
}

impl TokenLookup {
    /// Indexes every `*.txt` directly under `root`.
    ///
    /// Flat rather than recursive on purpose — the reference keeps its token
    /// scripts in one directory — but the count is returned so a caller can
    /// hold it to a floor rather than take an empty index for an answer.
    ///
    /// # Errors
    ///
    /// IO errors while reading the directory.
    pub fn new(root: std::path::PathBuf) -> Result<Self, crate::error::CodegenError> {
        let entries =
            std::fs::read_dir(&root).map_err(crate::error::CodegenError::io(root.as_path()))?;
        let mut stems = std::collections::BTreeSet::new();
        for entry in entries {
            let path = entry
                .map_err(crate::error::CodegenError::io(root.as_path()))?
                .path();
            if path.extension().is_some_and(|e| e == "txt")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                stems.insert(stem.to_string());
            }
        }
        Ok(Self { root, stems })
    }

    /// How many token scripts the index found.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stems.len()
    }

    /// Whether the index found none at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stems.is_empty()
    }

    /// The token a stem names, or `None` when there is no such script or
    /// [`read`] refused it.
    ///
    /// The two are one answer on purpose. A caller has the same thing to do
    /// either way — refuse the card that asked — and telling them apart here
    /// would invite a rule that treats "I could not read this token" as
    /// softer than "there is no such token", which is how a card ends up
    /// creating the wrong permanent.
    #[must_use]
    pub fn body(&self, stem: &str, cats: &SubtypeCatalogs) -> Option<TokenBody> {
        if !self.stems.contains(stem) {
            return None;
        }
        let text = std::fs::read_to_string(self.root.join(format!("{stem}.txt"))).ok()?;
        read(&text, cats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats() -> SubtypeCatalogs {
        let mut cats = SubtypeCatalogs {
            creature: vec!["Soldier".into(), "Bird".into(), "Shapeshifter".into()],
            artifact: vec!["Treasure".into(), "Equipment".into()],
            enchantment: vec![],
            land: vec![],
            planeswalker: vec![],
            spell: vec![],
        };
        cats.normalize();
        cats
    }

    /// The plainest token there is, and the shape 360 of the reference's 852
    /// have: a name, a colour, a type line and a size.
    #[test]
    fn a_plain_creature_token_reads_as_what_it_is() {
        let body = read(
            "Name:Soldier Token\nManaCost:no cost\nColors:white\n\
             Types:Creature Soldier\nPT:1/1\nOracle:",
            &cats(),
        )
        .expect("a 1/1 white Soldier is readable");
        assert_eq!(body.constant, "SOLDIER_1_1_WHITE");
        assert_eq!(body.doc, "1/1 white Soldier.");
        assert_eq!(
            body.literal,
            "TokenDef {\n    \
             name: \"Soldier\",\n    \
             colors: ColorSet::from_slice(&[Color::White]),\n    \
             types: TypeSet::CREATURE,\n    \
             subtypes: &[creature::SOLDIER],\n    \
             power: Some(1),\n    \
             toughness: Some(1),\n    \
             ..TokenDef::DEFAULT\n}"
        );
        assert_eq!(body.modules, ["creature"]);
    }

    /// Keyword bits are read, and they are part of the name — two tokens that
    /// differ only in a keyword are two tokens with two ids.
    #[test]
    fn a_keyword_is_read_and_is_part_of_the_name() {
        let body = read(
            "Name:Bird Token\nColors:white\nTypes:Creature Bird\nPT:1/1\nK:Flying\nOracle:",
            &cats(),
        )
        .expect("a 1/1 white Bird with flying is readable");
        assert_eq!(body.constant, "BIRD_1_1_WHITE_FLYING");
        assert_eq!(body.doc, "1/1 white Bird with flying.");
        assert!(body.literal.contains("keywords: KeywordSet::FLYING,"));
    }

    /// A colourless non-creature keeps neither a colour nor a size in its
    /// name, which is how the hand-written registry spells `TREASURE`.
    #[test]
    fn a_colourless_artifact_is_named_after_itself_alone() {
        let body = read(
            "Name:Treasure Token\nTypes:Artifact Treasure\nOracle:",
            &cats(),
        )
        .expect("an artifact with no size is readable");
        assert_eq!(body.constant, "TREASURE");
        assert_eq!(body.doc, "colorless Treasure.");
        assert!(!body.literal.contains("power"));
        assert!(
            !body.literal.contains("colors:"),
            "colourless is the default and is not restated: {}",
            body.literal
        );
        assert_eq!(body.modules, ["artifact"]);
    }

    /// Everything the type line says beyond "creature" is part of the name.
    /// Two of the reference's Soldiers are a plain creature and an
    /// enchantment creature, and they are two permanents — one constant for
    /// the pair would hand one of them the other's id, and so the other's
    /// picture.
    #[test]
    fn what_the_type_line_says_beyond_creature_is_part_of_the_name() {
        let plain = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1",
            &cats(),
        )
        .expect("a plain Soldier is readable");
        let enchanted = read(
            "Name:Soldier Token\nColors:white\nTypes:Enchantment Creature Soldier\nPT:1/1",
            &cats(),
        )
        .expect("an enchantment Soldier is readable");
        assert_eq!(plain.constant, "SOLDIER_1_1_WHITE");
        assert_eq!(enchanted.constant, "SOLDIER_ENCHANTMENT_1_1_WHITE");
        assert_eq!(enchanted.doc, "1/1 white enchantment Soldier.");
        assert!(
            enchanted
                .literal
                .contains("types: TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),"),
            "{}",
            enchanted.literal
        );

        let legendary = read(
            "Name:Soldier Token\nColors:white\nTypes:Legendary Creature Soldier\nPT:2/2",
            &cats(),
        )
        .expect("a legendary Soldier is readable");
        assert_eq!(legendary.constant, "SOLDIER_LEGENDARY_2_2_WHITE");
        assert!(
            legendary
                .literal
                .contains("supertypes: SupertypeSet::LEGENDARY,"),
            "{}",
            legendary.literal
        );
    }

    /// The script's line order is not the token's. The corpus writes
    /// `Defender,Flying` beside `Flying,Vigilance` and `red,white` beside
    /// `green,blue`, and a name that followed it would file one token under
    /// two constants — which in an append-only table is two ids the client
    /// keys two pictures off, forever.
    #[test]
    fn the_order_the_script_lists_things_in_is_not_part_of_the_token() {
        let one = read(
            "Name:Bird Token\nColors:blue,white\nTypes:Creature Bird\nPT:2/2\n\
             K:Vigilance\nK:Flying",
            &cats(),
        )
        .expect("readable either way round");
        let other = read(
            "Name:Bird Token\nColors:white,blue\nTypes:Creature Bird\nPT:2/2\n\
             K:Flying\nK:Vigilance",
            &cats(),
        )
        .expect("readable either way round");
        assert_eq!(one, other);
        assert_eq!(one.constant, "BIRD_2_2_WHITE_BLUE_FLYING_VIGILANCE");
        assert!(
            one.literal
                .contains("colors: ColorSet::from_slice(&[Color::White, Color::Blue]),"),
            "{}",
            one.literal
        );
    }

    /// One unread line and the token is refused. Each of these is a real
    /// shape in the reference, and each would have produced a token that is
    /// wrong rather than missing.
    #[test]
    fn anything_unread_refuses_the_token() {
        for script in [
            // Rules text, which this reader does not model at all.
            "Name:Horror Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\n\
             Text:This creature is all colors.",
            // An ability, in each of the four shapes the reference writes.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nA:AB$ Draw | Cost$ T",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nT:Mode$ Attacks",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nS:Mode$ Continuous",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nSVar:X:Count$Valid",
            // A second face.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nAlternateMode:Double",
            // A computed size.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:*/*",
            // A keyword that is data rather than a bit.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Equip:2",
            // A card type this reader does not write, and a subtype the pool
            // has never heard of.
            "Name:Dungeon Token\nTypes:Dungeon",
            "Name:Horror Token\nColors:white\nTypes:Creature Horror\nPT:1/1",
            // A colour word beside "colorless", which is a script this has
            // misread rather than one it can average out.
            "Name:Soldier Token\nColors:colorless,white\nTypes:Creature Soldier\nPT:1/1",
            // A creature with no size, and a size on something that is not a
            // creature: the two halves disagree and neither wins.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier",
            "Name:Treasure Token\nTypes:Artifact Treasure\nPT:1/1",
            // No name at all, which one script in the reference manages.
            "ManaCost:no cost\nTypes:Creature Soldier\nPT:1/1",
        ] {
            assert!(read(script, &cats()).is_none(), "{script}");
        }
    }
}
