//! Reads a reference **token** script into a [`TokenDef`] literal.
//!
//! A token script is a card script with almost everything taken out — a name,
//! a colour word, a type line, a power/toughness, and sometimes an ability.
//! Everything but the ability is read here; the ability is read by
//! [`scriptgen`](crate::scriptgen), because `TokenDef::abilities` is the same
//! `AbilityDef` slice a card face carries and the engine reads it the same
//! way, so a second transcoder would be a second reading of one language.
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
//! other's picture.
//!
//! The one thing the name leaves out is the **abilities**, and that is a
//! hole rather than a decision. The reference prints `b_1_1_skeleton` beside
//! `b_1_1_skeleton_regenerate`, so two different definitions come out under
//! one name — `cargo run -p xtask -- transcode-report` counts them and says
//! which, because a number in a comment here would be a number nobody could
//! ask for and would be wrong after the next rule. Naming a token after its
//! abilities as well is *a* way out; what is done instead is that
//! [`crate::tokenledger::assign`] refuses the collision, so the day this
//! pool reaches for both halves of such a pair it stops the run and hands a
//! person the example rather than the id.
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

impl TokenBody {
    fn front_literal(&self) -> &str {
        self.literal
            .split_once("\n    back_face:")
            .map_or(self.literal.as_str(), |(front, _)| front)
    }

    /// The fixed printed `(power, toughness)` emitted on the front face of this
    /// `TokenDef`, if any.
    #[must_use]
    pub fn printed_pt(&self) -> (Option<i16>, Option<i16>) {
        let front = self.front_literal();
        let parse_line = |prefix: &str| -> Option<i16> {
            front.lines().find_map(|line| {
                line.strip_prefix(prefix)?
                    .strip_suffix("),")?
                    .parse::<i16>()
                    .ok()
            })
        };
        (
            parse_line("    power: Some("),
            parse_line("    toughness: Some("),
        )
    }

    /// Whether this token's front face is a creature with a `*` power and/or
    /// `*` toughness that has no `Modifier::CharacteristicPT` CDA (and therefore
    /// must be created via `Effect::CreateTokenSized` supplying the missing
    /// side(s), CR 111.3).
    #[must_use]
    pub fn needs_creation_pt(&self) -> (bool, bool) {
        let front = self.front_literal();
        let is_creature = front
            .lines()
            .any(|line| line.starts_with("    types: ") && line.contains("TypeSet::CREATURE"));
        if !is_creature || front.contains("Modifier::CharacteristicPT") {
            return (false, false);
        }
        let (power, toughness) = self.printed_pt();
        (power.is_none(), toughness.is_none())
    }

    /// Whether this token's front face defines its P/T via a CDA
    /// (`Modifier::CharacteristicPT`).
    #[must_use]
    pub fn has_characteristic_pt(&self) -> bool {
        self.front_literal().contains("Modifier::CharacteristicPT")
    }
}

/// The line head a token script carries that says nothing this reads.
///
/// `Oracle` is the printed reminder text, which this side has no use for — a
/// token has no printing to hold a header against. `ManaCost` is validated
/// separately so only `"no cost"` passes (a token back face with a real mana
/// cost is refused because [`TokenDef`] carries no mana cost). Every other
/// unhandled head refuses the token.
const IGNORED: &[&str] = &["Oracle"];

/// The line heads [`scriptgen`](crate::scriptgen) reads, not this module.
///
/// A token's abilities are a card's abilities — `TokenDef::abilities` is the
/// same `AbilityDef` slice a `FaceDef` carries, and the engine reads it the
/// same way — so there is one transcoder for both and this side merely gets
/// out of its way. `K` is **not** on the list: a keyword is a bit on the
/// token rather than an ability, and the transcoder would read it a second
/// time into a card's `keywords`, which a token has its own field for.
///
/// `S` and `R` are on it because the engine really does read a *token's*
/// static and replacement abilities: both of `progress.rs`'s collectors walk
/// the battlefield and ask `obj.abilities(&lookup)`, which is the accessor
/// that falls through to `TokenDef::abilities` — the same route a Treasure's
/// activated ability and the trigger scan already take. Had either read the
/// card instead, a token whose `S:` transcoded in full would have been an
/// honest-looking `Implemented` with an anthem it never applied.
const TRANSCODED: &[&str] = &["A", "T", "S", "R", "SVar"];

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

struct RawToken {
    name: String,
    colors: Option<String>,
    types: String,
    pt: Option<String>,
    loyalty: Option<u16>,
    defense: Option<u16>,
    double_faced: bool,
    keywords: Vec<&'static str>,
    ability_keywords: Vec<String>,
    text_all_colors: bool,
    has_rule_lines: bool,
}

fn split_double_faced(text: &str) -> Option<(String, Option<String>)> {
    let mut front = Vec::new();
    let mut back: Option<Vec<&str>> = None;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed == "ALTERNATE" {
            if back.is_some() {
                return None;
            }
            back = Some(Vec::new());
            continue;
        }
        if let Some(back_lines) = &mut back {
            back_lines.push(trimmed);
        } else {
            front.push(trimmed);
        }
    }
    Some((front.join("\n"), back.map(|lines| lines.join("\n"))))
}

fn parse_token_lines(text: &str, allow_double_faced: bool) -> Option<RawToken> {
    let (mut name, mut colors, mut types, mut pt) = (None, None, None, None);
    let mut loyalty = None;
    let mut defense = None;
    let mut double_faced = false;
    let mut keywords: Vec<&'static str> = Vec::new();
    let mut ability_keywords: Vec<String> = Vec::new();
    let mut text_all_colors = false;
    let mut has_rule_lines = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (head, rest) = line.split_once(':')?;
        match head {
            "Name" => name = Some(rest.trim().to_string()),
            "ManaCost" => {
                if !rest.trim().eq_ignore_ascii_case("no cost") {
                    return None;
                }
            }
            "Colors" => colors = Some(rest.trim().to_string()),
            "Types" => types = Some(rest.trim().to_string()),
            "PT" => pt = Some(rest.trim().to_string()),
            "Loyalty" => {
                if loyalty.is_some() {
                    return None;
                }
                loyalty = Some(rest.trim().parse::<u16>().ok()?);
            }
            "Defense" => {
                if defense.is_some() {
                    return None;
                }
                defense = Some(rest.trim().parse::<u16>().ok()?);
            }
            "AlternateMode" => {
                if !allow_double_faced || double_faced || rest.trim() != "DoubleFaced" {
                    return None;
                }
                double_faced = true;
            }
            "Text" => match rest.trim() {
                "This creature is all colors." | "CARDNAME is all colors." => {
                    text_all_colors = true;
                }
                _ => return None,
            },
            "K" => {
                if let Some(bit) = single_instance_trigger_keyword(rest.trim()) {
                    // CR 702.39b/91b/105b/121b/134b/149b: multiple instances
                    // trigger separately; refuse duplicate lines rather than
                    // silently collapsing them onto one bit.
                    if keywords.contains(&bit) {
                        return None;
                    }
                    keywords.push(bit);
                } else if let Some(bit) = crate::scriptgen::keyword_const_of(rest) {
                    keywords.push(bit);
                } else {
                    match rest.trim() {
                        "CARDNAME can't attack." => keywords.push("KeywordSet::CANT_ATTACK"),
                        "CARDNAME can't block." => keywords.push("KeywordSet::CANT_BLOCK"),
                        "CARDNAME can't attack or block." => {
                            keywords.push("KeywordSet::CANT_ATTACK");
                            keywords.push("KeywordSet::CANT_BLOCK");
                        }
                        other if is_token_ability_keyword(other) => {
                            ability_keywords.push(other.to_string());
                        }
                        _ => return None,
                    }
                }
            }
            "A" | "T" | "S" | "R" => has_rule_lines = true,
            _ if IGNORED.contains(&head) || TRANSCODED.contains(&head) => {}
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
    Some(RawToken {
        name: name?,
        colors,
        types: types?,
        pt,
        loyalty,
        defense,
        double_faced,
        keywords,
        ability_keywords,
        text_all_colors,
        has_rule_lines,
    })
}

fn single_instance_trigger_keyword(kw: &str) -> Option<&'static str> {
    match kw {
        "Provoke" => Some("KeywordSet::PROVOKE"),
        "Battle Cry" | "Battle cry" => Some("KeywordSet::BATTLE_CRY"),
        "Dethrone" => Some("KeywordSet::DETHRONE"),
        "Melee" => Some("KeywordSet::MELEE"),
        "Mentor" => Some("KeywordSet::MENTOR"),
        "Training" => Some("KeywordSet::TRAINING"),
        "Myriad" => Some("KeywordSet::MYRIAD"),
        "Extort" => Some("KeywordSet::EXTORT"),
        "Enlist" => Some("KeywordSet::ENLIST"),
        "Soulbond" => Some("KeywordSet::SOULBOND"),
        "Exploit" => Some("KeywordSet::EXPLOIT"),
        "Evolve" => Some("KeywordSet::EVOLVE"),
        "Riot" => Some("KeywordSet::RIOT"),
        "Unleash" => Some("KeywordSet::UNLEASH"),
        _ => None,
    }
}

struct LiteralFields<'a> {
    name: &'a str,
    colors: Option<&'a Colors>,
    types: &'a TypeLine,
    power: Option<i16>,
    toughness: Option<i16>,
    loyalty: Option<u16>,
    defense: Option<u16>,
    keywords: &'a [&'a str],
    abilities: &'a [String],
    back_face: Option<&'a str>,
}

struct TokenStats {
    power: Option<i16>,
    toughness: Option<i16>,
    star_power: bool,
    star_toughness: bool,
    toughness_plus: Option<u8>,
    loyalty: Option<u16>,
}

struct TokenPt {
    power: Option<i16>,
    toughness: Option<i16>,
    star_power: bool,
    star_toughness: bool,
    toughness_plus: Option<u8>,
}

fn render_token_literal(fields: &LiteralFields<'_>) -> String {
    let mut literal = String::with_capacity(256);
    literal.push_str("TokenDef {\n");
    let _ = writeln!(literal, "    name: {:?},", token_name(fields.name));
    if let Some(colors) = fields.colors
        && !colors.words.is_empty()
    {
        let _ = writeln!(literal, "    colors: {},", colors.expr);
    }
    let _ = writeln!(literal, "    types: {},", fields.types.set);
    if let Some(supertypes) = &fields.types.supertypes {
        let _ = writeln!(literal, "    supertypes: {supertypes},");
    }
    if !fields.types.subtypes.is_empty() {
        let _ = writeln!(
            literal,
            "    subtypes: &[{}],",
            fields.types.subtypes.join(", ")
        );
    }
    if let Some(p) = fields.power {
        let _ = writeln!(literal, "    power: Some({p}),");
    }
    if let Some(t) = fields.toughness {
        let _ = writeln!(literal, "    toughness: Some({t}),");
    }
    if let Some(loyalty) = fields.loyalty {
        let _ = writeln!(literal, "    loyalty: Some({loyalty}),");
    }
    if let Some(defense) = fields.defense {
        let _ = writeln!(literal, "    defense: Some({defense}),");
    }
    if !fields.keywords.is_empty() {
        let _ = writeln!(literal, "    keywords: {},", union_of(fields.keywords));
    }
    if !fields.abilities.is_empty() {
        let _ = writeln!(
            literal,
            "    abilities: &[{}],",
            fields.abilities.join(", ")
        );
    }
    if let Some(back) = fields.back_face {
        literal.push_str("    back_face: Some(&");
        for (idx, line) in back.lines().enumerate() {
            if idx > 0 {
                literal.push('\n');
                literal.push_str("    ");
            }
            literal.push_str(line);
        }
        literal.push_str("),\n");
    }
    literal.push_str("    ..TokenDef::DEFAULT\n}");
    literal
}

fn token_shape_ok(raw: &RawToken, types: &TypeLine) -> bool {
    if types.set.contains("TypeSet::PLANESWALKER") != raw.loyalty.is_some() {
        return false;
    }
    if types.set.contains("TypeSet::BATTLE") != raw.defense.is_some() {
        return false;
    }
    let is_equipment =
        !types.is_creature && types.subtypes.iter().any(|s| s == "artifact::EQUIPMENT");
    if !is_equipment && raw.ability_keywords.iter().any(|k| k.starts_with("Equip:")) {
        return false;
    }
    // CR 303.7 Role uniqueness SBA is enforced in `sba.rs`, so both plain and
    // Role Aura tokens are legal to generate.
    let is_aura = !types.is_creature && types.subtypes.iter().any(|s| s == "enchantment::AURA");
    is_aura
        || !raw
            .ability_keywords
            .iter()
            .any(|k| k.starts_with("Enchant:"))
}

/// Reads a token script, or refuses it.
///
/// `None` is an honest refusal, and a token whose ability this cannot write
/// is one the engine would put on the battlefield inert — which is worse
/// than a card that stays a stub, because the deckbuilder offers a card
/// marked `Implemented` as playable.
///
/// How often that happens is counted by [`TokenLookup::reach`] and printed
/// by `xtask transcode-report`, not written down here: the reasons move
/// every time a rule is added, and a doc comment that named them would be a
/// worklist with nothing keeping it true.
#[must_use]
pub fn read(text: &str, cats: &SubtypeCatalogs) -> Option<TokenBody> {
    read_with_tokens(text, cats, None)
}

/// Reads a token script with an optional [`TokenLookup`] for nested token
/// creation (e.g. a token whose ability creates a Treasure or Food token).
///
/// Only nested references to hand-written `crate::tokens::*` constants are
/// admitted, because `xtask/src/corpus.rs` collects token stems only from card
/// scripts and `generated_tokens.rs` brings `crate::tokens` into scope.
#[must_use]
pub fn read_with_tokens(
    text: &str,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<TokenBody> {
    let (front_text, back_text) = split_double_faced(text)?;
    let back = match back_text.as_deref() {
        Some(back) => Some(read_face(back, cats, tokens, false, None)?),
        None => None,
    };
    read_face(&front_text, cats, tokens, true, back.as_ref())
}

fn parse_token_pt(pt: Option<&str>, types: &TypeLine) -> Option<TokenPt> {
    let is_vehicle = !types.is_creature && types.subtypes.iter().any(|s| s == "artifact::VEHICLE");
    match (pt, types.is_creature, is_vehicle) {
        (Some(pt), true, false) => {
            let (raw_p, raw_t) = pt.split_once('/')?;
            let (power, star_power) = match raw_p.trim() {
                "*" => (None, true),
                other => (Some(other.parse::<i16>().ok()?), false),
            };
            let (toughness, star_toughness, toughness_plus) = match raw_t.trim() {
                "*" => (None, true, None),
                other => {
                    if let Some(plus) = other.strip_prefix("*+") {
                        let n = plus.parse::<u8>().ok().filter(|&n| (1..=9).contains(&n))?;
                        (None, true, Some(n))
                    } else {
                        (Some(other.parse::<i16>().ok()?), false, None)
                    }
                }
            };
            Some(TokenPt {
                power,
                toughness,
                star_power,
                star_toughness,
                toughness_plus,
            })
        }
        (Some(pt), false, true) => {
            let (p, t) = pt.split_once('/')?;
            let p = p.trim().parse::<i16>().ok()?;
            let t = t.trim().parse::<i16>().ok()?;
            Some(TokenPt {
                power: Some(p),
                toughness: Some(t),
                star_power: false,
                star_toughness: false,
                toughness_plus: None,
            })
        }
        (None, false, false) => Some(TokenPt {
            power: None,
            toughness: None,
            star_power: false,
            star_toughness: false,
            toughness_plus: None,
        }),
        // A creature or Vehicle with no size, a Vehicle with `*/*`, or a size on
        // something that is neither a creature nor a Vehicle: refuse rather than
        // guessing.
        _ => None,
    }
}

fn star_pt_ok(
    pt: &TokenPt,
    abilities: &[String],
    allow_double_faced: bool,
    double_faced: bool,
) -> bool {
    let has_cda = abilities
        .iter()
        .any(|a| a.contains("Modifier::CharacteristicPT"));
    if let Some(plus) = pt.toughness_plus {
        let needle = format!("toughness_plus: {plus}");
        return pt.star_power
            && abilities
                .iter()
                .any(|a| a.contains("Modifier::CharacteristicPT") && a.contains(&needle));
    }
    if abilities.iter().any(|a| {
        a.contains("Modifier::CharacteristicPT")
            && a.contains("toughness_plus: ")
            && !a.contains("toughness_plus: 0")
    }) {
        return false;
    }
    if (!allow_double_faced || double_faced) && (pt.star_power || pt.star_toughness) && !has_cda {
        return false;
    }
    !(pt.star_power != pt.star_toughness && has_cda)
}

fn read_face(
    text: &str,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
    allow_double_faced: bool,
    back: Option<&TokenBody>,
) -> Option<TokenBody> {
    let raw = parse_token_lines(text, allow_double_faced)?;
    if raw.double_faced != back.is_some() {
        return None;
    }
    let mut types = type_line(&raw.types, cats)?;
    if !token_shape_ok(&raw, &types) {
        return None;
    }
    let pt = parse_token_pt(raw.pt.as_deref(), &types)?;
    let colors = match raw.colors.as_deref() {
        Some(raw_colors) => Some(color_line(raw_colors)?),
        None => None,
    };
    if raw.text_all_colors && colors.as_ref().is_none_or(|c| c.words.len() != 5) {
        return None;
    }
    let (abilities, extra_keywords) = abilities(
        text,
        &raw.ability_keywords,
        raw.text_all_colors,
        raw.double_faced,
        cats,
        tokens,
        &mut types.modules,
    )?;
    if !star_pt_ok(&pt, &abilities, allow_double_faced, raw.double_faced) {
        return None;
    }
    let mut render_keywords = raw.keywords.clone();
    render_keywords.extend(extra_keywords);
    render_keywords.sort_unstable();
    render_keywords.dedup();
    let literal = render_token_literal(&LiteralFields {
        name: &raw.name,
        colors: colors.as_ref(),
        types: &types,
        power: pt.power,
        toughness: pt.toughness,
        loyalty: raw.loyalty,
        defense: raw.defense,
        keywords: &render_keywords,
        abilities: &abilities,
        back_face: back.map(|b| b.literal.as_str()),
    });
    let mut keyword_tags: Vec<String> = raw
        .keywords
        .iter()
        .map(|k| k.trim_start_matches("KeywordSet::").to_string())
        .collect();
    for kw in &raw.ability_keywords {
        keyword_tags.push(ability_keyword_slug(kw)?);
    }
    keyword_tags.sort_unstable();
    let stats = TokenStats {
        power: pt.power,
        toughness: pt.toughness,
        star_power: pt.star_power,
        star_toughness: pt.star_toughness,
        toughness_plus: pt.toughness_plus,
        loyalty: raw.loyalty,
    };
    let mut constant = constant_name(
        &raw.name,
        &types.qualifiers,
        &stats,
        colors.as_ref(),
        &keyword_tags,
    );
    if raw.has_rule_lines && !abilities.is_empty() {
        let hash = fnv1a_32_bare(&abilities.join(","));
        if !is_legacy_unsuffixed_ability_token(&constant, hash) {
            let _ = write!(constant, "_A{hash:08X}");
        }
    }
    let mut doc = doc_line(
        &raw.name,
        &types.qualifiers,
        &stats,
        colors.as_ref(),
        &keyword_tags,
    );
    if let Some(back) = back {
        let _ = write!(constant, "_DFC_{}", back.constant);
        doc = format!("{} // {}", doc.trim_end_matches('.'), back.doc);
        for module in &back.modules {
            if !types.modules.iter().any(|m| m == module) {
                types.modules.push(module.clone());
            }
        }
    }
    Some(TokenBody {
        constant: crate::tokenledger::canonical_name(&constant).to_string(),
        doc,
        literal,
        modules: types.modules,
    })
}

/// Deterministic 32-bit FNV-1a hash ignoring ASCII whitespace, matching
/// [`crate::tokenledger`]'s whitespace-insensitive definition equivalence.
const fn fnv1a_32_bare(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut h: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !matches!(b, b' ' | b'\n' | b'\r' | b'\t') {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        i += 1;
    }
    h
}

/// Rules-bearing token constants that were already frozen in `tokens.rs` or
/// `generated_tokens.rs` before content-addressed ability suffixes existed.
///
/// Keyed by `(constant, abilities_hash)` so ONLY the exact legacy definition
/// retains the unsuffixed constant name while any different rules-bearing
/// variant on the same base body receives its own `_A<HEX>` constant.
const LEGACY_UNSUFFIXED_ABILITY_TOKENS: &[(&str, u32)] = &[
    // Predefined hand-written artifact tokens in `baylee_cards::tokens`.
    (
        "TREASURE",
        fnv1a_32_bare(
            "mana_ability!(cost!(TapSelf, SacrificeSelf), &[Effect::mana_of_any_color()])",
        ),
    ),
    (
        "FOOD",
        fnv1a_32_bare(
            "activated!(cost!(\"{2}\", TapSelf, SacrificeSelf), &[Effect::gain_life(3)])",
        ),
    ),
    (
        "CLUE",
        fnv1a_32_bare("activated!(cost!(\"{2}\", SacrificeSelf), &[Effect::draw(1)])"),
    ),
    (
        "BLOOD",
        fnv1a_32_bare(
            "activated!(cost!(\"{1}\", TapSelf, Discard(&Filter::Any), SacrificeSelf), &[Effect::draw(1)])",
        ),
    ),
    (
        "POWERSTONE",
        fnv1a_32_bare(
            "mana_ability!(&[Effect::mana(ManaColor::Colorless, 1).restricted_for(&Filter::ARTIFACT, SpendRider::None, ManaUse::NonSpellPaymentsOrMatchingSpells)])",
        ),
    ),
    (
        "MAP",
        fnv1a_32_bare(
            "activated!(cost!(\"{1}\", TapSelf, SacrificeSelf), &[Effect::Explore { \
             target: TargetSpec::Object(&Filter::YOUR_CREATURE) }], \
             timing = ActivationTiming::SorcerySpeed, \
             target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)))",
        ),
    ),
    // Previously assigned generated ability-bearing tokens in `generated_tokens.rs`.
    (
        "GOLD",
        fnv1a_32_bare("mana_ability!(cost!(SacrificeSelf), &[Effect::mana_of_any_color()])"),
    ),
    (
        "ELDRAZI_SPAWN_0_1",
        fnv1a_32_bare(
            "mana_ability!(cost!(SacrificeSelf), &[Effect::mana(ManaColor::Colorless, 1)])",
        ),
    ),
    (
        "ELDRAZI_SCION_1_1",
        fnv1a_32_bare(
            "mana_ability!(cost!(SacrificeSelf), &[Effect::mana(ManaColor::Colorless, 1)])",
        ),
    ),
    (
        "BIRD_2_2_GREEN",
        fnv1a_32_bare(
            "triggered!(Trigger::EntersBattlefield(&Filter::YOUR_LAND), &[Effect::PumpFilter { \
             filter: &Filter::This, controlled_by: None, power: Amount::Fixed(1), \
             toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, \
             duration: Duration::UntilEndOfTurn }])",
        ),
    ),
    (
        "TRISKELAVITE_ARTIFACT_1_1_FLYING",
        fnv1a_32_bare(
            "activated!(cost!(SacrificeSelf), &[Effect::DealDamage { amount: Amount::Fixed(1), \
             target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget))",
        ),
    ),
    (
        "FESTERING_GOBLIN_1_1_BLACK",
        fnv1a_32_bare(
            "triggered!(Trigger::Dies(&Filter::This), &[Effect::PumpTarget { \
             power: Amount::NegXFixed(1), toughness: Amount::NegXFixed(1), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
        ),
    ),
    (
        "GOLDMEADOW_HARRIER_1_1_WHITE",
        fnv1a_32_bare(
            "activated!(cost!(\"{W}\", TapSelf), &[Effect::TapTarget], \
             target = Some(TargetSpec::Object(&Filter::CREATURE)))",
        ),
    ),
    (
        "LLANOWAR_ELVES_1_1_GREEN",
        fnv1a_32_bare("mana_ability!(&[Effect::mana(ManaColor::Green, 1)])"),
    ),
    (
        "PEST_1_1_BLACK_GREEN",
        fnv1a_32_bare(
            "triggered!(Trigger::Dies(&Filter::This), &[Effect::GainLifeFor { \
             amount: Amount::Fixed(1), who: PlayerRel::ControllerOfEvent }])",
        ),
    ),
    (
        "DEVIL_1_1_RED",
        fnv1a_32_bare(
            "triggered!(Trigger::Dies(&Filter::This), &[Effect::DealDamage { \
             amount: Amount::Fixed(1), target: TargetSpec::AnyTarget }], \
             targets = Some(TargetReq::one(TargetSpec::AnyTarget)))",
        ),
    ),
    (
        "DRAGON_2_2_RED_FLYING",
        fnv1a_32_bare(
            "activated!(cost!(\"{R}\"), &[Effect::PumpFilter { filter: &Filter::This, \
             controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(0), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])",
        ),
    ),
    (
        "REDWING_LEGENDARY_1_1_BLUE_FLYING",
        fnv1a_32_bare("triggered!(Trigger::Attacks(&Filter::This), &[Effect::surveil(1)])"),
    ),
    (
        "WIZARD_1_1_BLUE",
        fnv1a_32_bare(
            "activated!(cost!(\"{1}\", SacrificeSelf), &[Effect::PlayerMayPayOr { \
             player: PlayerRel::ControllerOfTarget, mana: Amount::Fixed(1), \
             effect: &Effect::CounterTargetSpell }], \
             target = Some(TargetSpec::Spell(&Filter::NONCREATURE)))",
        ),
    ),
    (
        "SNAKE_ARTIFACT_1_1",
        fnv1a_32_bare(
            "triggered!(Trigger::DealsDamageToPlayer { source: &Filter::This, \
             player: PlayerRel::EachPlayer }, &[Effect::Poison { \
             who: PlayerRel::DamagedPlayer, amount: Amount::Fixed(1) }])",
        ),
    ),
];

fn is_legacy_unsuffixed_ability_token(constant: &str, hash: u32) -> bool {
    LEGACY_UNSUFFIXED_ABILITY_TOKENS
        .iter()
        .any(|&(name, expected)| name == constant && expected == hash)
}

fn sanitize_tag(raw: &str) -> Option<String> {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_string();
    (!out.is_empty()).then_some(out)
}

fn ability_keyword_slug(kw: &str) -> Option<String> {
    if let Some(rest) = kw.strip_prefix("Ward:") {
        return Some(format!("WARD_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Toxic:") {
        return Some(format!("TOXIC_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Crew:") {
        return Some(format!("CREW_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Saddle:") {
        return Some(format!("SADDLE_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Equip:") {
        return Some(format!("EQUIP_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Protection from ") {
        return Some(format!("PROTECTION_FROM_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Protection:") {
        let quality = rest.split(':').next()?;
        return Some(format!("PROTECTION_FROM_{}", sanitize_tag(quality)?));
    }
    if let Some(rest) = kw.strip_prefix("Landwalk:") {
        let land = rest.split(':').next()?.trim();
        return match land {
            "Plains" => Some("PLAINSWALK".to_string()),
            "Island" => Some("ISLANDWALK".to_string()),
            "Swamp" => Some("SWAMPWALK".to_string()),
            "Mountain" => Some("MOUNTAINWALK".to_string()),
            "Forest" => Some("FORESTWALK".to_string()),
            other => Some(format!("LANDWALK_{}", sanitize_tag(other)?)),
        };
    }
    if let Some(rest) = kw.strip_prefix("Rampage:") {
        return Some(format!("RAMPAGE_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Cumulative upkeep:") {
        let cost = rest.split(':').next()?;
        return Some(format!("CUMULATIVE_UPKEEP_{}", sanitize_tag(cost)?));
    }
    if let Some(rest) = kw.strip_prefix("Enchant:") {
        let valid = rest.split(':').next()?;
        return Some(format!("ENCHANT_{}", sanitize_tag(valid)?));
    }
    if let Some(rest) = kw.strip_prefix("Bands with other:") {
        let valid = rest.split(':').next()?;
        return Some(format!("BANDS_WITH_OTHER_{}", sanitize_tag(valid)?));
    }
    if let Some(rest) = kw.strip_prefix("Annihilator:") {
        return Some(format!("ANNIHILATOR_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Firebending:") {
        return Some(format!("FIREBENDING_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Afterlife:") {
        return Some(format!("AFTERLIFE_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Afflict:") {
        return Some(format!("AFFLICT_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Mobilize:") {
        return Some(format!("MOBILIZE_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Bushido:") {
        return Some(format!("BUSHIDO_{}", sanitize_tag(rest)?));
    }
    if let Some(rest) = kw.strip_prefix("Soulshift:") {
        return Some(format!("SOULSHIFT_{}", sanitize_tag(rest)?));
    }
    match kw {
        "Exalted" => Some("EXALTED".to_string()),
        "Prevent all damage that would be dealt to CARDNAME." => Some("PREVENT_DAMAGE".to_string()),
        "CARDNAME can only attack alone." => Some("ATTACKS_ALONE".to_string()),
        "You may choose not to untap CARDNAME during your untap step." => {
            Some("MAY_NOT_UNTAP".to_string())
        }
        "All creatures able to block CARDNAME do so." => Some("MUST_BE_BLOCKED_BY_ALL".to_string()),
        _ => None,
    }
}

fn is_numeric_token_param(rest: &str) -> bool {
    !rest.contains(':') && rest.trim().parse::<u32>().is_ok_and(|n| n >= 1)
}

fn is_token_ability_keyword(kw: &str) -> bool {
    kw.starts_with("Ward:")
        || kw.starts_with("Toxic:")
        || kw.starts_with("Crew:")
        || kw.starts_with("Saddle:")
        || kw.starts_with("Equip:")
        || kw.starts_with("Protection from ")
        || kw.starts_with("Protection:")
        || kw.starts_with("Landwalk:")
        || kw.starts_with("Rampage:")
        || kw.starts_with("Cumulative upkeep:")
        || kw.starts_with("Enchant:")
        || kw.starts_with("Bands with other:")
        || kw.starts_with("Annihilator:")
        || kw
            .strip_prefix("Firebending:")
            .is_some_and(is_numeric_token_param)
        || kw
            .strip_prefix("Afterlife:")
            .is_some_and(is_numeric_token_param)
        || kw
            .strip_prefix("Afflict:")
            .is_some_and(is_numeric_token_param)
        || kw
            .strip_prefix("Mobilize:")
            .is_some_and(is_numeric_token_param)
        || kw
            .strip_prefix("Bushido:")
            .is_some_and(is_numeric_token_param)
        || kw
            .strip_prefix("Soulshift:")
            .is_some_and(|rest| !rest.contains(':') && rest.trim().parse::<u32>().is_ok())
        || matches!(
            kw,
            "Exalted"
                | "Prevent all damage that would be dealt to CARDNAME."
                | "CARDNAME can only attack alone."
                | "You may choose not to untap CARDNAME during your untap step."
                | "All creatures able to block CARDNAME do so."
        )
}

/// The abilities a token script prints, read by the card transcoder.
///
/// `Some(vec![])` is a token with no rules line at all, `None` a refusal.
///
/// Filter and condition `static` items produced by `scriptgen` are inlined back
/// into the `AbilityDef` expressions (valid under Rust constant promotion inside
/// `pub static ...: TokenDef`), and any `subtypes::<mod>::` references are
/// rewritten to `<mod>::` and recorded in `modules` so `generated_tokens.rs`
/// imports them cleanly. Non-ability `CardBody` fields (`EnterModifier`, cast
/// cost mechanics, extra `KeywordSet` writes) remain refused because `TokenDef`
/// does not carry them.
/// Hand-written `crate::tokens::*` artifact constants guaranteed to be in scope
/// inside `generated_tokens.rs` via `use crate::tokens;`, so a generated token's
/// ability may safely rewrite `&generated_tokens::<NAME>` -> `&tokens::<NAME>`.
const HAND_WRITTEN_NESTED_TOKENS: &[&str] = &[
    "ALLY_1_1_WHITE",
    "ANGEL_4_4_WHITE_FLYING",
    "ARMY_0_0_BLACK",
    "BEAST_5_5_GREEN",
    "BIRD_1_1_WHITE_FLYING",
    "BLOOD",
    "BOAR_2_2_GREEN",
    "CLUE",
    "CONSTRUCT_ARTIFACT_0_0",
    "FOOD",
    "HUMAN_SOLDIER_1_1_WHITE",
    "JUNK",
    "KNIGHT_2_2_WHITE_BLUE_VIGILANCE",
    "MAP",
    "POWERSTONE",
    "SERVO_1_1",
    "SPIRIT_1_1_COLORLESS",
    "TREASURE",
    "VIBRANIUM",
    "ZOMBIE_2_2_BLACK",
];

/// Generated `pub static` token constants already frozen in the append-only
/// `crates/cards/src/generated_tokens.rs` ledger, so another generated token's
/// ability may safely rewrite `&generated_tokens::<NAME>` -> `&<NAME>`.
const GENERATED_NESTED_TOKENS: &[&str] = &[
    "BEAST_3_3_GREEN",
    "CAT_1_1_WHITE",
    "DRAGON_2_2_RED_FLYING",
    "ELDRAZI_SCION_1_1",
    "ELDRAZI_SPAWN_0_1",
    "ELF_WARRIOR_1_1_GREEN",
    "GOLD",
    "HUMAN_1_1_WHITE",
    "INSECT_1_1_GREEN",
    "KNIGHT_2_2_WHITE_VIGILANCE",
    "KRAKEN_9_9_BLUE",
    "MERFOLK_1_1_BLUE",
    "OOZE_2_2_GREEN",
    "PLANT_0_1_GREEN",
    "RABBIT_1_1_WHITE",
    "SAPROLING_1_1_GREEN",
    "SERVO_ARTIFACT_1_1",
    "SKELETON_1_1_BLACK",
    "SKELETON_4_1_BLACK_MENACE",
    "SPIRIT_1_1_WHITE_BLACK_FLYING",
    "THOPTER_ARTIFACT_1_1_FLYING",
    "WARRIOR_1_1_RED",
    "WOLF_2_2_GREEN",
];

fn rewrite_nested_token_refs(abilities: &mut [String]) -> bool {
    for ability in abilities.iter_mut() {
        let mut out = String::with_capacity(ability.len());
        let mut rest = ability.as_str();
        while let Some((before, after)) = rest.split_once("&generated_tokens::") {
            let ident: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if ident.is_empty() {
                return false;
            }
            out.push_str(before);
            if HAND_WRITTEN_NESTED_TOKENS.contains(&ident.as_str()) {
                out.push_str("&tokens::");
                out.push_str(&ident);
            } else if GENERATED_NESTED_TOKENS.contains(&ident.as_str()) {
                out.push('&');
                out.push_str(&ident);
            } else {
                return false;
            }
            rest = &after[ident.len()..];
        }
        out.push_str(rest);
        *ability = out;
    }
    true
}

fn abilities(
    text: &str,
    ability_keywords: &[String],
    strip_all_colors_text: bool,
    strip_double_faced: bool,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
    modules: &mut Vec<String>,
) -> Option<(Vec<String>, Vec<&'static str>)> {
    let cleaned;
    let source = if strip_all_colors_text || strip_double_faced {
        cleaned = text
            .lines()
            .filter(|l| {
                let trimmed = l.trim_start();
                !(strip_all_colors_text && trimmed.starts_with("Text:")
                    || strip_double_faced && trimmed.starts_with("AlternateMode:"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        cleaned.as_str()
    } else {
        text
    };
    let mut script = crate::scriptgen::parse(source);
    if script.rules.is_empty() && script.svars.is_empty() && ability_keywords.is_empty() {
        return Some((Vec::new(), Vec::new()));
    }
    // A head this module read is not a head the transcoder may read again,
    // and a head neither of them knows has already refused the token above
    // — except a malformed `SVar:` with no second colon, which only the
    // parser sees.
    if !script.unknown_lines.is_empty()
        || script.alternate_mode.is_some()
        || script.alternate.is_some()
    {
        return None;
    }
    script.keywords = ability_keywords.to_vec();
    let body = crate::scriptgen::transcode(&script, cats, tokens)?;
    let mut extra_keywords = Vec::new();
    for kw in &body.keywords {
        let bit = match kw.as_str() {
            "KeywordSet::FIREBENDING" => "KeywordSet::FIREBENDING",
            "KeywordSet::AFTERLIFE" => "KeywordSet::AFTERLIFE",
            "KeywordSet::AFFLICT" => "KeywordSet::AFFLICT",
            "KeywordSet::MOBILIZE" => "KeywordSet::MOBILIZE",
            _ => return None,
        };
        extra_keywords.push(bit);
    }
    if !body.enter_modifiers.is_empty()
        || !body.alternative_costs.is_empty()
        || !body.mandatory_additional_costs.is_empty()
        || body.convoke
        || body.delve
        || body.improvise
        || body.replicate.is_some()
        || body.disguise.is_some()
        || body.flashback.is_some()
        || !body.flashback_parts.is_empty()
        || body.madness.is_some()
        || body.foretell.is_some()
        || body.miracle.is_some()
        || body.dash.is_some()
        || body.blitz.is_some()
        || body.morph.is_some()
        || body.megamorph.is_some()
        || !body.morph_parts.is_empty()
        || body.bestow.is_some()
        || body.prototype.is_some()
        || body.bargain
        || body.emerge.is_some()
        || body.jump_start
        || body.aftermath
        || body.disturb.is_some()
        || body.escape.is_some()
        || body.squad.is_some()
        || body.offspring.is_some()
        || body.fuse
        || body.behold_or_pay.is_some()
        || body.mutate.is_some()
        || body.plot.is_some()
        || body.casualty.is_some()
        || body.impending.is_some()
        || body.gift.is_some()
        || body.warp.is_some()
        || body.optional_cost.is_some()
        || body.optional_cost_grants_flash
        || body.cost_reduction.is_some()
        || body.entwine.is_some()
        || body.buyback.is_some()
        || body.multikicker.is_some()
        || !body.additional_costs.is_empty()
        || body.kicked_targets.is_some()
        || body.has_partner
        || body.back.is_some()
        || body.back_is_adventure
        || body.back_is_prepare
    {
        return None;
    }
    let mut abilities = body.abilities;
    if !body.statics.is_empty() {
        inline_statics(&body.statics, &mut abilities)?;
    }
    if !rewrite_nested_token_refs(&mut abilities) {
        return None;
    }
    rewrite_subtypes(&mut abilities, modules)?;
    Some((abilities, extra_keywords))
}

fn inline_statics(statics: &str, abilities: &mut [String]) -> Option<()> {
    let mut defs: Vec<(&str, &str)> = Vec::new();
    for line in statics.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rest = trimmed.strip_prefix("static ")?;
        let (name, rhs) = rest.split_once(": ")?;
        let (ty, expr) = rhs.split_once(" = ")?;
        let ty = ty.trim();
        let valid_ty = matches!(
            ty,
            "Filter"
                | "Condition"
                | "AbilityDef"
                | "Amount"
                | "PtCount"
                | "Effect"
                | "Trigger"
                | "Modifier"
        ) || ((ty.starts_with("[AbilityDef; ")
            || ty.starts_with("[Effect; ")
            || ty.starts_with("[Filter; ")
            || ty.starts_with("[Modifier; ")
            || ty.starts_with("[SubtypeId; "))
            && ty.ends_with(']'));
        if !valid_ty {
            return None;
        }
        let expr = expr.strip_suffix(';')?;
        defs.push((name.trim(), expr.trim()));
    }
    if defs.is_empty() {
        return None;
    }
    for _ in 0..6 {
        let mut changed = false;
        for ability in abilities.iter_mut() {
            for &(name, expr) in &defs {
                if let Some(updated) = replace_ident(ability, name, expr) {
                    *ability = updated;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // Verify no static identifier remains unresolved (with or without `&`).
    for ability in abilities.iter() {
        for &(name, _) in &defs {
            if contains_ident(ability, name) {
                return None;
            }
        }
    }
    Some(())
}

fn contains_ident(text: &str, ident: &str) -> bool {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while let Some(rel) = text[cursor..].find(ident) {
        let start = cursor + rel;
        let end = start + ident.len();
        let prev_is_ident = start > 0
            && bytes
                .get(start - 1)
                .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
        let next_is_ident = bytes
            .get(end)
            .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
        if !prev_is_ident && !next_is_ident {
            return true;
        }
        cursor = end;
    }
    false
}

fn replace_ident(text: &str, ident: &str, replacement: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut replaced = false;
    while i < bytes.len() {
        if in_string {
            let ch = text[i..].chars().next()?;
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
            continue;
        }
        if text[i..].starts_with(ident) {
            let end = i + ident.len();
            let prev = i.checked_sub(1).and_then(|p| bytes.get(p).copied());
            let next = bytes.get(end).copied();
            let prev_ok = prev.is_none_or(|b| {
                !(b.is_ascii_alphanumeric() || b == b'_' || b == b':' || b == b'.')
            });
            let next_ok =
                next.is_none_or(|b| !(b.is_ascii_alphanumeric() || b == b'_' || b == b':'));
            if prev_ok && next_ok {
                out.push_str(replacement);
                replaced = true;
                i = end;
                continue;
            }
        }
        let ch = text[i..].chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
    }
    replaced.then_some(out)
}

fn rewrite_subtypes(abilities: &mut [String], modules: &mut Vec<String>) -> Option<()> {
    const KNOWN: &[&str] = &[
        "artifact",
        "battle",
        "creature",
        "enchantment",
        "land",
        "planeswalker",
        "spell",
    ];
    for ability in abilities.iter_mut() {
        for &module in KNOWN {
            let full = format!("subtypes::{module}::");
            if ability.contains(&full) {
                let short = format!("{module}::");
                *ability = ability.replace(&full, &short);
                if !modules.iter().any(|m| m == module) {
                    modules.push(module.to_string());
                }
            }
        }
        if ability.contains("subtypes::") {
            return None;
        }
    }
    Some(())
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
        if word.eq_ignore_ascii_case("all") {
            if raw.contains(',') {
                return None;
            }
            found = [true; 5];
            continue;
        }
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
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        if let Some((_, konst)) = TYPES.iter().find(|(w, _)| *w == word) {
            if *konst == "TypeSet::CREATURE" {
                is_creature = true;
            } else {
                qualifiers.push(word.to_ascii_uppercase());
            }
            types.push(*konst);
            i += 1;
        } else if let Some((_, konst)) = SUPERTYPES.iter().find(|(w, _)| *w == word) {
            qualifiers.push(word.to_ascii_uppercase());
            supers.push(*konst);
            i += 1;
        } else if TypeSet::from_word(word).is_some() || SupertypeSet::from_word(word).is_some() {
            // A card type this reader does not write. Refused by name rather
            // than left to the subtype lookup below, which would answer
            // `None` for the same word and hide which of the two questions
            // the token failed.
            return None;
        } else {
            let (raw_sub, consumed) = if i + 1 < words.len() {
                let next = words[i + 1];
                let next_is_type = TYPES.iter().any(|(w, _)| *w == next)
                    || SUPERTYPES.iter().any(|(w, _)| *w == next)
                    || TypeSet::from_word(next).is_some()
                    || SupertypeSet::from_word(next).is_some();
                if next_is_type {
                    (cats.const_path(word)?, 1)
                } else {
                    let pair = format!("{word} {next}");
                    if let Some(path) = cats.const_path(&pair) {
                        (path, 2)
                    } else {
                        (cats.const_path(word)?, 1)
                    }
                }
            } else {
                (cats.const_path(word)?, 1)
            };
            // `subtypes::creature::SOLDIER` — the generated file imports the
            // modules, so the literal names the shorter path.
            let path = raw_sub.strip_prefix("subtypes::")?.to_string();
            let module = path.split("::").next()?.to_string();
            if !modules.contains(&module) {
                modules.push(module);
            }
            subtypes.push(path);
            i += consumed;
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
    stats: &TokenStats,
    colors: Option<&Colors>,
    keyword_tags: &[String],
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
    match (
        stats.power,
        stats.toughness,
        stats.star_power,
        stats.star_toughness,
    ) {
        (Some(p), Some(t), _, _) => {
            let _ = write!(out, "_{}_{}", pt_word(p), pt_word(t));
        }
        (None, None, true, true) => match stats.toughness_plus {
            Some(n) => {
                let _ = write!(out, "_X_X_PLUS_{n}");
            }
            None => out.push_str("_X_X"),
        },
        (None, Some(t), true, false) => {
            let _ = write!(out, "_X_{}", pt_word(t));
        }
        (Some(p), None, false, true) => {
            let _ = write!(out, "_{}_X", pt_word(p));
        }
        _ => {}
    }
    if let Some(loyalty) = stats.loyalty {
        let _ = write!(out, "_L{loyalty}");
    }
    for word in colors.map_or_default(|c| c.words.as_slice()) {
        let _ = write!(out, "_{}", word.to_ascii_uppercase());
    }
    for tag in keyword_tags {
        let _ = write!(out, "_{tag}");
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
    stats: &TokenStats,
    colors: Option<&Colors>,
    keyword_tags: &[String],
) -> String {
    let mut out = String::new();
    match (
        stats.power,
        stats.toughness,
        stats.star_power,
        stats.star_toughness,
    ) {
        (Some(p), Some(t), _, _) => {
            let _ = write!(out, "{p}/{t} ");
        }
        (None, None, true, true) => match stats.toughness_plus {
            Some(n) => {
                let _ = write!(out, "*/*+{n} ");
            }
            None => out.push_str("*/* "),
        },
        (None, Some(t), true, false) => {
            let _ = write!(out, "*/{t} ");
        }
        (Some(p), None, false, true) => {
            let _ = write!(out, "{p}/* ");
        }
        _ => {}
    }
    match colors.map_or_default(|c| c.words.as_slice()) {
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
    if !keyword_tags.is_empty() {
        let words: Vec<String> = keyword_tags
            .iter()
            .map(|k| k.to_ascii_lowercase().replace('_', " "))
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
    source: Source,
    stems: std::collections::BTreeSet<String>,
}

/// Where a [`TokenLookup`]'s scripts are.
///
/// The in-memory half is not a convenience: the reference is not vendored, so
/// a test that had to read a token off disk would be a test CI skips, and a
/// rule nobody runs is a rule nobody has.
enum Source {
    /// A directory of `<stem>.txt` files.
    Dir(std::path::PathBuf),
    /// Scripts held by stem, written by a test.
    Held(std::collections::BTreeMap<String, String>),
}

/// The environment variable that names the token corpus, for a checkout laid
/// out in a way [`TokenLookup::beside`] cannot derive.
pub const TOKENS_ENV: &str = "BAYLEE_TOKEN_SCRIPTS";

impl TokenLookup {
    /// The token scripts beside a checkout's card scripts, if there are any.
    ///
    /// Derived rather than asked for: `res/cardsfolder` and `res/tokenscripts`
    /// are siblings in one checkout, so a second path to configure would be a
    /// second thing to point at last month's copy. [`TOKENS_ENV`] is the way
    /// out for a layout this cannot guess, and it is read first so that it can
    /// also point a run at nothing at all.
    ///
    /// It lives here rather than in the one command that writes the ledger
    /// because every tool that *reports* on the transcoder has to find the
    /// same directory the same way — a report run without the tokens would
    /// otherwise rank "there is no token directory" as a gap in the DSL.
    ///
    /// # Errors
    ///
    /// IO errors while reading the directory.
    pub fn beside(
        scripts_dir: &std::path::Path,
    ) -> Result<Option<Self>, crate::error::CodegenError> {
        let root = if let Some(named) = std::env::var_os(TOKENS_ENV) {
            std::path::PathBuf::from(named)
        } else {
            let Some(parent) = scripts_dir.parent() else {
                return Ok(None);
            };
            parent.join("tokenscripts")
        };
        if !root.is_dir() {
            return Ok(None);
        }
        Self::new(root).map(Some)
    }

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
        Ok(Self {
            source: Source::Dir(root),
            stems,
        })
    }

    /// A lookup over scripts already in hand.
    #[must_use]
    pub fn held(scripts: &[(&str, &str)]) -> Self {
        let held: std::collections::BTreeMap<String, String> = scripts
            .iter()
            .map(|(stem, text)| ((*stem).to_string(), (*text).to_string()))
            .collect();
        Self {
            stems: held.keys().cloned().collect(),
            source: Source::Held(held),
        }
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
        let text = self.text(stem)?;
        self.read_entry(&text, cats)
    }

    /// Like [`Self::body`], but refuses creature tokens whose `*` power or
    /// toughness has no `Modifier::CharacteristicPT` CDA — those may only be
    /// created through `Effect::CreateTokenSized` (`CR 111.3`).
    #[must_use]
    pub fn body_fixed(&self, stem: &str, cats: &SubtypeCatalogs) -> Option<TokenBody> {
        let body = self.body(stem, cats)?;
        (body.needs_creation_pt() == (false, false)).then_some(body)
    }

    fn read_entry(&self, text: &str, cats: &SubtypeCatalogs) -> Option<TokenBody> {
        std::thread_local! {
            static IN_NESTED_TOKEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        }
        IN_NESTED_TOKEN.with(|nested| {
            if nested.get() {
                return read_with_tokens(text, cats, None);
            }
            nested.set(true);
            let res = read_with_tokens(text, cats, Some(self));
            nested.set(false);
            res
        })
    }

    /// The script a stem names, whichever half of the lookup holds it.
    fn text(&self, stem: &str) -> Option<String> {
        match &self.source {
            Source::Dir(root) => std::fs::read_to_string(root.join(format!("{stem}.txt"))).ok(),
            Source::Held(held) => held.get(stem).cloned(),
        }
    }

    /// How far this reader gets over the whole corpus.
    ///
    /// Counted rather than remembered. Every one of these numbers used to
    /// sit in a doc comment as "measured", which is a number that is right
    /// on the day it is typed and silently wrong after the next rule — and
    /// the naming collision in particular is a fact about the corpus that
    /// only a count can keep honest.
    #[must_use]
    pub fn reach(&self, cats: &SubtypeCatalogs) -> TokenReach {
        let mut out = TokenReach {
            total: self.stems.len(),
            ..TokenReach::default()
        };
        let mut names: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            std::collections::BTreeMap::new();
        for stem in &self.stems {
            let Some(text) = self.text(stem) else {
                continue;
            };
            let prints = text.lines().any(|line| {
                line.split_once(':')
                    .is_some_and(|(head, _)| TRANSCODED.contains(&head))
            });
            out.with_ability += usize::from(prints);
            if let Some(body) = self.read_entry(&text, cats) {
                out.read += 1;
                out.ability_read += usize::from(prints);
                names.entry(body.constant).or_default().insert(body.literal);
            }
        }
        out.names = names.len();
        out.collisions = names
            .iter()
            .filter(|(_, defs)| defs.len() > 1)
            .map(|(name, _)| name.clone())
            .collect();
        out
    }
}

/// How far [`read`] reaches over a whole token corpus. See
/// [`TokenLookup::reach`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TokenReach {
    /// Token scripts in the corpus.
    pub total: usize,
    /// Of those, the ones read in full.
    pub read: usize,
    /// Scripts that print at least one rules line.
    pub with_ability: usize,
    /// Of those, the ones read in full.
    pub ability_read: usize,
    /// Distinct constant names over everything read.
    pub names: usize,
    /// Names two different definitions claim, which is two tokens at one id.
    ///
    /// Named rather than counted, because four is a number somebody can act
    /// on and "four" is not: each of these is a pair the naming rule cannot
    /// tell apart, and the fix for it is decided by looking at the pair.
    pub collisions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats() -> SubtypeCatalogs {
        let mut cats = SubtypeCatalogs {
            creature: vec![
                "Soldier".into(),
                "Bird".into(),
                "Shapeshifter".into(),
                "Pest".into(),
            ],
            artifact: vec!["Treasure".into(), "Equipment".into()],
            enchantment: vec!["Aura".into(), "Role".into()],
            ..SubtypeCatalogs::default()
        };
        cats.normalize();
        cats
    }

    #[test]
    fn battle_token_preserves_defense_and_refuses_missing_or_invalid_values() {
        let valid = "Name:Battle Token\nTypes:Battle\nDefense:5\nOracle:";
        let body = read(valid, &cats()).expect("numeric Battle token");
        assert!(body.literal.contains("defense: Some(5),"));
        for text in [
            "Name:Battle Token\nTypes:Battle\nOracle:",
            "Name:Battle Token\nTypes:Battle\nDefense:*\nOracle:",
            "Name:Battle Token\nTypes:Battle\nDefense:65536\nOracle:",
            "Name:Rock Token\nTypes:Artifact\nDefense:5\nOracle:",
        ] {
            assert!(read(text, &cats()).is_none());
        }
        assert!(
            read(
                "Name:Battle Token\nTypes:Battle\nDefense:5\nAlternateMode:Back\nOracle:",
                &cats()
            )
            .is_none()
        );
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

    /// A token's abilities are read by the transcoder that reads a card's,
    /// and come out spelled the way a card spells them.
    ///
    /// Treasure is the case the pool actually waits on — 98 of the
    /// reference's cards name `c_a_treasure_sac` — and it is also the one
    /// that has to come out as a **mana ability**: CR 605.1 makes that the
    /// exception, and a Treasure whose ability used the stack could not be
    /// cracked to pay for the spell it is being cracked for. Nothing in the
    /// engine's tests would read that as a rules bug.
    ///
    /// The Skeleton beside it is the other half of the same claim: an
    /// ordinary activated ability stays ordinary.
    #[test]
    fn a_token_that_prints_an_ability_carries_it() {
        let treasure = read(
            "Name:Treasure Token\nManaCost:no cost\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any | Amount$ 1",
            &cats(),
        )
        .expect("the Treasure is read in full");
        assert!(
            treasure.literal.contains(
                "abilities: &[mana_ability!(cost!(TapSelf, SacrificeSelf), \
                 &[Effect::mana_of_any_color()])],"
            ),
            "{}",
            treasure.literal
        );

        let food = read(
            "Name:Food Token\nManaCost:no cost\nTypes:Artifact Treasure\n\
             A:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME/this token> | LifeAmount$ 3",
            &cats(),
        )
        .expect("the Food is read in full");
        assert!(
            food.literal.contains(
                "abilities: &[activated!(cost!(\"{2}\", TapSelf, SacrificeSelf), \
                 &[Effect::gain_life(3)])],"
            ),
            "{}",
            food.literal
        );
    }

    /// Two tokens that share every printed characteristic but differ in their
    /// rules-line abilities receive distinct content-addressed constants so the
    /// ledger never assigns two definitions to one id, while two identical
    /// rules-bearing scripts resolve to the same constant.
    #[test]
    fn two_tokens_that_differ_only_in_their_ability_get_distinct_names() {
        let plain = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1",
            &cats(),
        )
        .expect("read");
        let drawing = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1",
            &cats(),
        )
        .expect("read");
        let drawing_again = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1",
            &cats(),
        )
        .expect("read");
        let lifegain = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\n\
             A:AB$ GainLife | Cost$ T | LifeAmount$ 1",
            &cats(),
        )
        .expect("read");
        assert_eq!(plain.constant, "SOLDIER_1_1_WHITE");
        assert_ne!(plain.constant, drawing.constant, "distinct from vanilla");
        assert_eq!(drawing.constant, drawing_again.constant, "deterministic");
        assert_ne!(
            drawing.constant, lifegain.constant,
            "distinct abilities get distinct constants"
        );
        assert!(drawing.constant.starts_with("SOLDIER_1_1_WHITE_A"));
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
            // An ability the transcoder cannot write. Four shapes, and each
            // is unread for its own reason: a trigger with no `Execute$`, a
            // static naming no modifier, an `SVar` no rule reaches, and an
            // ability that creates a token — which is refused because this
            // reader hands the transcoder no token lookup, so the reference's
            // Mitotic Ooze cannot read itself into existence.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nT:Mode$ Attacks",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nS:Mode$ Continuous",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\n\
             S:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ 0",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nSVar:X:Count$Valid",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:2/2\n\
             A:AB$ Token | Cost$ T | TokenScript$ w_1_1_soldier | TokenOwner$ You",
            // A second face.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nAlternateMode:Double",
            // A computed size with an unsupported offset expression.
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:*/*+1",
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

    #[test]
    fn ability_keywords_and_keyword_sentences_read_on_tokens() {
        let ward_toxic = read(
            "Name:Mite Token\nTypes:Artifact Creature Soldier\nPT:1/1\n\
             K:CARDNAME can't block.\nK:Toxic:1\nK:Ward:2",
            &cats(),
        )
        .expect("token with can't block + toxic + ward");
        assert_eq!(
            ward_toxic.constant,
            "MITE_ARTIFACT_1_1_CANT_BLOCK_TOXIC_1_WARD_2"
        );
        assert!(
            ward_toxic
                .literal
                .contains("keywords: KeywordSet::CANT_BLOCK,")
        );
        assert!(
            ward_toxic
                .literal
                .contains("AbilityDef::Toxic { poison: 1 }")
        );
        assert!(ward_toxic.literal.contains("AbilityDef::Ward { mana: 2 }"));

        let equip_token = read(
            "Name:Blade Token\nTypes:Artifact Equipment\n\
             S:Mode$ Continuous | Affected$ Creature.EquippedBy | AddPower$ 1 | AddToughness$ 1\n\
             K:Equip:2",
            &cats(),
        )
        .expect("equipment token with equip keyword");
        assert!(
            equip_token.constant.starts_with("BLADE_EQUIP_2_A"),
            "constant: {}",
            equip_token.constant
        );
        assert!(equip_token.literal.contains("equip!(cost!(\"{2}\"))"));

        let pro_token = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:2/2\n\
             K:Protection from black",
            &cats(),
        )
        .expect("protection token avoids plain collision");
        assert_eq!(
            pro_token.constant,
            "SOLDIER_2_2_WHITE_PROTECTION_FROM_BLACK"
        );
        assert_eq!(
            pro_token.doc,
            "2/2 white Soldier with protection from black."
        );

        let provoke_token = read(
            "Name:Scout Token\nColors:white\nTypes:Creature Soldier\nPT:2/2\nK:Provoke",
            &cats(),
        )
        .expect("provoke token");
        assert_eq!(provoke_token.constant, "SCOUT_2_2_WHITE_PROVOKE");
        assert!(
            provoke_token
                .literal
                .contains("keywords: KeywordSet::PROVOKE,")
        );

        // Duplicate Provoke must be refused rather than silently collapsed.
        assert!(
            read(
                "Name:Scout Token\nColors:white\nTypes:Creature Soldier\nPT:2/2\n\
                 K:Provoke\nK:Provoke",
                &cats()
            )
            .is_none()
        );
    }

    #[test]
    fn token_inlines_filter_statics_and_rewrites_subtype_modules() {
        let body = read(
            "Name:Banner Token\nTypes:Artifact Treasure\n\
             A:AB$ Pump | Cost$ T | ValidTgts$ Soldier | NumAtt$ 1 | NumDef$ 1",
            &cats(),
        )
        .expect("token referencing a subtype filter in an ability");
        assert!(
            !body.literal.contains("TARGET1") && !body.literal.contains("subtypes::"),
            "literal: {}",
            body.literal
        );
        assert!(
            body.literal.contains("creature::SOLDIER"),
            "literal: {}",
            body.literal
        );
        assert!(
            body.modules.iter().any(|m| m == "creature")
                && body.modules.iter().any(|m| m == "artifact"),
            "modules: {:?}",
            body.modules
        );
    }

    #[test]
    fn star_pt_token_reads_only_with_characteristic_pt_modifier() {
        let body = read(
            "Name:Avatar Token\nColors:white\nTypes:Creature Soldier\nPT:*/*\n\
             S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
             SVar:X:Count$Valid Creature.YouCtrl",
            &cats(),
        )
        .expect("star P/T token with CDA");
        assert!(
            body.constant.starts_with("AVATAR_X_X_WHITE_A"),
            "constant: {}",
            body.constant
        );
        assert_eq!(body.doc, "*/* white Avatar.");
        assert!(!body.literal.contains("power:"));
        assert!(!body.literal.contains("toughness:"));
        assert!(
            body.literal.contains("Modifier::CharacteristicPT"),
            "literal: {}",
            body.literal
        );
        assert_eq!(body.needs_creation_pt(), (false, false));

        // Creation-sized `*/*` and `*/1` tokens without a CDA read for
        // `Effect::CreateTokenSized` but are refused by `TokenLookup::body_fixed`.
        let lookup = TokenLookup::held(&[
            (
                "soldier_x_x",
                "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:*/*",
            ),
            (
                "bird_x_1",
                "Name:Bird Token\nColors:red\nTypes:Creature Bird\nPT:*/1\nK:Trample\nK:Haste",
            ),
            (
                "pest_1_x",
                "Name:Pest Token\nColors:black\nTypes:Creature Pest\nPT:1/*",
            ),
        ]);
        let soldier = lookup.body("soldier_x_x", &cats()).expect("sized soldier");
        assert_eq!(soldier.constant, "SOLDIER_X_X_GREEN");
        assert_eq!(soldier.doc, "*/* green Soldier.");
        assert_eq!(soldier.needs_creation_pt(), (true, true));
        assert!(lookup.body_fixed("soldier_x_x", &cats()).is_none());

        let bird = lookup.body("bird_x_1", &cats()).expect("sized bird");
        assert_eq!(bird.constant, "BIRD_X_1_RED_HASTE_TRAMPLE");
        assert_eq!(bird.doc, "*/1 red Bird with haste and trample.");
        assert_eq!(bird.needs_creation_pt(), (true, false));
        assert_eq!(bird.printed_pt(), (None, Some(1)));
        assert!(bird.literal.contains("toughness: Some(1),"));
        assert!(!bird.literal.contains("power:"));
        assert!(lookup.body_fixed("bird_x_1", &cats()).is_none());

        let pest = lookup.body("pest_1_x", &cats()).expect("sized pest");
        assert_eq!(pest.constant, "PEST_1_X_BLACK");
        assert_eq!(pest.doc, "1/* black Pest.");
        assert_eq!(pest.needs_creation_pt(), (false, true));
        assert_eq!(pest.printed_pt(), (Some(1), None));
        assert!(pest.literal.contains("power: Some(1),"));
        assert!(!pest.literal.contains("toughness:"));
        assert!(lookup.body_fixed("pest_1_x", &cats()).is_none());
    }

    #[test]
    fn legacy_ability_tokens_keep_unsuffixed_names_while_variants_disambiguate() {
        let treasure = read(
            "Name:Treasure Token\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any | Amount$ 1",
            &cats(),
        )
        .expect("canonical treasure");
        assert_eq!(treasure.constant, "TREASURE");

        let dies_pest = read(
            "Name:Pest Token\nColors:black,green\nTypes:Creature Pest\nPT:1/1\n\
             T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | \
             ValidCard$ Card.Self | Execute$ TrigGain\n\
             SVar:TrigGain:DB$ GainLife | Defined$ TriggeredCardController | LifeAmount$ 1",
            &cats(),
        )
        .expect("legacy dies pest");
        assert_eq!(dies_pest.constant, "PEST_1_1_BLACK_GREEN");

        let attack_pest = read(
            "Name:Pest Token\nColors:black,green\nTypes:Creature Pest\nPT:1/1\n\
             T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigGain\n\
             SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ 1",
            &cats(),
        )
        .expect("attack pest variant");
        assert!(
            attack_pest.constant.starts_with("PEST_1_1_BLACK_GREEN_A"),
            "constant: {}",
            attack_pest.constant
        );
        assert_ne!(dies_pest.constant, attack_pest.constant);
    }

    #[test]
    fn all_colors_cumulative_upkeep_and_non_role_aura_tokens() {
        let five_color = read(
            "Name:Citizen Token\nColors:all\nTypes:Creature Soldier\nPT:2/2\n\
             Text:This creature is all colors.",
            &cats(),
        )
        .expect("all-colors token");
        assert_eq!(
            five_color.constant,
            "CITIZEN_2_2_WHITE_BLUE_BLACK_RED_GREEN"
        );
        assert!(
            five_color
                .literal
                .contains("Color::White, Color::Blue, Color::Black, Color::Red, Color::Green")
        );

        let upkeep_tok = read(
            "Name:Splinter Token\nColors:green\nTypes:Creature Soldier\nPT:1/1\n\
             K:Flying\nK:Cumulative upkeep:G",
            &cats(),
        )
        .expect("cumulative upkeep token");
        assert_eq!(
            upkeep_tok.constant,
            "SPLINTER_1_1_GREEN_CUMULATIVE_UPKEEP_G_FLYING"
        );

        let aura_tok = read(
            "Name:Aura Token\nColors:green\nTypes:Enchantment Aura\n\
             K:Enchant:Land\n\
             S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddAbility$ TapAny\n\
             SVar:TapAny:AB$ Mana | Cost$ T | Produced$ Any",
            &cats(),
        )
        .expect("non-role aura token");
        assert!(
            aura_tok.constant.starts_with("AURA_GREEN_ENCHANT_LAND_A"),
            "constant: {}",
            aura_tok.constant
        );

        // Role Aura tokens are now supported with CR 303.7 enforced in `sba.rs`.
        let role_tok = read(
            "Name:Role Token\nTypes:Enchantment Aura Role\nK:Enchant:Creature",
            &cats(),
        )
        .expect("role aura token");
        assert!(role_tok.literal.contains("enchantment::ROLE"));
    }

    #[test]
    fn wave5_vehicle_annihilator_bands_and_cant_attack_tokens() {
        let mut c = cats();
        c.artifact.push("Vehicle".to_string());
        c.creature.push("Eldrazi".to_string());
        c.creature.push("Wolf".to_string());

        let vehicle = read(
            "Name:Skiff Token\nTypes:Artifact Vehicle\nPT:3/2\nK:Flying\nK:Crew:1",
            &c,
        )
        .expect("vehicle token with fixed P/T and crew");
        assert_eq!(vehicle.constant, "SKIFF_3_2_CREW_1_FLYING");
        assert!(vehicle.literal.contains("power: Some(3),"));
        assert!(vehicle.literal.contains("toughness: Some(2),"));
        assert!(vehicle.literal.contains("crew!(1)"));

        // Vehicle with `*/*` or missing P/T, and non-Vehicle artifact with P/T, stay refused.
        assert!(
            read(
                "Name:Star Skiff\nTypes:Artifact Vehicle\nPT:*/*\nK:Crew:2",
                &c
            )
            .is_none()
        );
        assert!(read("Name:Boxless Skiff\nTypes:Artifact Vehicle\nK:Crew:2", &c).is_none());
        assert!(read("Name:Sized Rock\nTypes:Artifact\nPT:2/2", &c).is_none());

        let eldrazi = read(
            "Name:Titan Token\nTypes:Creature Eldrazi\nPT:7/7\nK:Annihilator:1",
            &c,
        )
        .expect("annihilator creature token");
        assert_eq!(eldrazi.constant, "TITAN_7_7_ANNIHILATOR_1");
        assert!(
            eldrazi
                .literal
                .contains("Effect::SacrificeAmountForPlayers")
        );
        assert!(eldrazi.literal.contains("PlayerRel::DefendingPlayer"));

        let bands = read(
            "Name:Pack Token\nColors:green\nTypes:Creature Wolf\nPT:1/1\n\
             K:Bands with other:Creature.Wolf:Wolves",
            &c,
        )
        .expect("bands-with-other token");
        assert_eq!(
            bands.constant,
            "PACK_1_1_GREEN_BANDS_WITH_OTHER_CREATURE_WOLF"
        );
        assert!(bands.literal.contains("Modifier::BandsWithOther"));
        assert!(bands.literal.contains("creature::WOLF"));

        let pacific = read(
            "Name:Sentry Token\nColors:white\nTypes:Creature Soldier\nPT:0/4\n\
             K:CARDNAME can't attack.",
            &c,
        )
        .expect("cant-attack token");
        assert_eq!(pacific.constant, "SENTRY_0_4_WHITE_CANT_ATTACK");
        assert!(pacific.literal.contains("KeywordSet::CANT_ATTACK"));
    }

    #[test]
    fn wave6_powerstone_and_nested_token_refs() {
        let mut c = cats();
        c.artifact.push("Powerstone".to_string());
        c.creature.push("Rabbit".to_string());

        let powerstone = read(
            "Name:Powerstone Token\nTypes:Artifact Powerstone\n\
             A:AB$ Mana | Cost$ T | Produced$ C | RestrictValid$ CantCastNonArtifactSpells",
            &c,
        )
        .expect("powerstone token");
        assert_eq!(powerstone.constant, "POWERSTONE");
        assert!(
            powerstone
                .literal
                .contains("ManaUse::NonSpellPaymentsOrMatchingSpells")
        );

        let lookup = TokenLookup::held(&[
            (
                "t_treasure",
                "Name:Treasure Token\nTypes:Artifact Treasure\n\
                 A:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any",
            ),
            (
                "t_rabbit",
                "Name:Rabbit Token\nColors:white\nTypes:Creature Rabbit\nPT:1/1",
            ),
            (
                "t_soldier",
                "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1",
            ),
            (
                "t_raider",
                "Name:Raider Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\n\
                 T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ Mk\n\
                 SVar:Mk:DB$ Token | TokenScript$ t_treasure",
            ),
            (
                "t_hat",
                "Name:Hat Token\nTypes:Artifact\n\
                 A:AB$ Token | Cost$ 1 T Sac<1/CARDNAME> | TokenScript$ t_rabbit",
            ),
            (
                "t_unknown_nested",
                "Name:Banner Token\nTypes:Artifact\n\
                 A:AB$ Token | Cost$ 1 T Sac<1/CARDNAME> | TokenScript$ t_soldier",
            ),
            (
                "t_self_loop",
                "Name:Loop Token\nTypes:Artifact\n\
                 A:AB$ Token | Cost$ T Sac<1/CARDNAME> | TokenScript$ t_self_loop",
            ),
        ]);

        let raider = lookup
            .body("t_raider", &c)
            .expect("hand-written nested token");
        assert!(raider.literal.contains("token: &tokens::TREASURE"));
        let hat = lookup
            .body("t_hat", &c)
            .expect("frozen generated nested token");
        assert!(hat.literal.contains("token: &RABBIT_1_1_WHITE"));
        assert!(lookup.body("t_unknown_nested", &c).is_none());
        assert!(lookup.body("t_self_loop", &c).is_none());
    }

    #[test]
    fn wave7_equipment_grantor_sacrifice_and_owner_graveyard_cda_tokens() {
        let mut c = cats();
        c.creature.push("Elephant".to_string());
        c.normalize();

        let pebble = read(
            "Name:Pebble\nTypes:Artifact Equipment\n\
             S:Mode$ Continuous | Affected$ Creature.EquippedBy | AddAbility$ Toss\n\
             SVar:Toss:AB$ DealDamage | Cost$ 1 T Sac<1/OriginalHost/Pebble> | ValidTgts$ Any | NumDmg$ 2\n\
             K:Equip:1",
            &c,
        )
        .expect("equipment token granting SacrificeGrantor");
        assert!(pebble.literal.contains("SacrificeGrantor"));
        assert!(pebble.literal.contains("GrantActivatedDefinition"));
        assert!(pebble.constant.starts_with("PEBBLE_EQUIP_1_A"));

        let elephant = read(
            "Name:Elephant Token\nColors:green\nTypes:Creature Elephant\nPT:*/*\n\
             S:Mode$ Continuous | SetPower$ G | SetToughness$ G | EffectZone$ Battlefield | \
             CharacteristicDefining$ True\n\
             SVar:G:Count$ValidGraveyard Creature.YouOwn",
            &c,
        )
        .expect("owner-graveyard CDA star creature token");
        assert!(elephant.literal.contains("Modifier::CharacteristicPT"));
        assert!(elephant.literal.contains("PtCount::InAllGraveyards"));
        assert!(elephant.literal.contains("Filter::OwnedByYou"));
        assert!(elephant.constant.starts_with("ELEPHANT_X_X_GREEN_A"));
    }

    #[test]
    fn wave8_training_and_combat_trigger_keywords() {
        let mut c = cats();
        c.creature.push("Human".to_string());
        c.normalize();

        let trainee = read(
            "Name:Human Soldier Token\nColors:white,green\nTypes:Creature Human Soldier\n\
             PT:1/1\nK:Training",
            &c,
        )
        .expect("Training token");
        assert!(trainee.literal.contains("KeywordSet::TRAINING"));
        assert_eq!(trainee.constant, "HUMAN_SOLDIER_1_1_WHITE_GREEN_TRAINING");

        let cry = read(
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Battle Cry",
            &c,
        )
        .expect("Battle cry token");
        assert!(cry.literal.contains("KeywordSet::BATTLE_CRY"));

        // Duplicate triggered-keyword instances must stay refused rather than collapsing.
        assert!(
            read(
                "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\n\
                 PT:1/1\nK:Training\nK:Training",
                &c,
            )
            .is_none()
        );
    }

    #[test]
    fn wave10_myriad_and_extort_tokens() {
        let mut c = cats();
        c.creature.push("Spirit".to_string());
        c.normalize();

        let myriad = read(
            "Name:Spirit Token\nColors:white\nTypes:Creature Spirit\nPT:1/1\nK:Myriad",
            &c,
        )
        .expect("Myriad token");
        assert!(myriad.literal.contains("KeywordSet::MYRIAD"));

        let extort = read(
            "Name:Spirit Token\nColors:black\nTypes:Creature Spirit\nPT:1/1\nK:Extort",
            &c,
        )
        .expect("Extort token");
        assert!(extort.literal.contains("KeywordSet::EXTORT"));

        for dup in ["Myriad", "Extort"] {
            assert!(
                read(
                    &format!(
                        "Name:Spirit Token\nColors:white\nTypes:Creature Spirit\nPT:1/1\nK:{dup}\nK:{dup}"
                    ),
                    &c,
                )
                .is_none(),
                "duplicate {dup} token must stay refused"
            );
        }
    }

    #[test]
    fn wave11a_enchant_player_aura_token_and_keyword_guards() {
        let mut c = cats();
        c.enchantment.push("Curse".to_string());
        c.normalize();

        let curse = read(
            "Name:Hex Token\nColors:black\nTypes:Enchantment Aura Curse\n\
             K:Enchant:Player\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Drain\n\
             SVar:Drain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 1\n\
             Oracle:Enchant player\\nAt the beginning of your upkeep, each opponent loses 1 life.",
            &c,
        )
        .expect("player aura token");
        assert!(
            curse
                .literal
                .contains("Effect::AttachSelf { target: TargetSpec::AnyPlayer }")
                && curse
                    .literal
                    .contains("TargetReq::one(TargetSpec::AnyPlayer)")
                && curse.literal.contains("Effect::LoseLife {"),
            "{}",
            curse.literal
        );

        for unsupported in [
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Squad:1 W",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Offspring:2",
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:2/2\nK:Escape:2 G ExileFromGrave<3/Card.Other>",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave12a_wave5b_and_attacks_alone_tokens() {
        let c = cats();
        for (kw, expected) in [
            ("Wither", "KeywordSet::WITHER"),
            ("Horsemanship", "KeywordSet::HORSEMANSHIP"),
            ("Skulk", "KeywordSet::SKULK"),
            ("Enlist", "KeywordSet::ENLIST"),
        ] {
            let tok = read(
                &format!(
                    "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:{kw}"
                ),
                &c,
            )
            .unwrap_or_else(|| panic!("{kw} token should transcode"));
            assert!(tok.literal.contains(expected), "{kw}: {}", tok.literal);
        }

        let alone = read(
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/1\nK:CARDNAME can only attack alone.",
            &c,
        )
        .expect("attacks alone token");
        assert!(
            alone.literal.contains("Modifier::AttacksAloneOnly"),
            "{}",
            alone.literal
        );

        for unsupported in [
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Enlist\nK:Enlist",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:AlternateAdditionalCost:Behold<1/Soldier>:2",
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:2/2\nK:Mutate:1 G",
            "Name:Soldier Token\nColors:blue\nTypes:Creature Soldier\nPT:2/2\nK:Plot:1 U",
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Casualty:1",
            "Name:Soldier Token\nColors:white\nTypes:Creature Soldier\nPT:1/1\nK:Backup:1:B\nSVar:B:DB$ Pump | KW$ Flying",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave14a_soulbond_two_word_subtype_and_map_legacy_hash() {
        let mut c = cats();
        c.creature.push("Time Lord".into());
        c.normalize();
        let sb = read(
            "Name:Scout Token\nColors:white\nTypes:Creature Time Lord\nPT:2/2\nK:Soulbond",
            &c,
        )
        .expect("Soulbond + Time Lord token");
        assert!(
            sb.literal.contains("KeywordSet::SOULBOND"),
            "{}",
            sb.literal
        );
        assert!(sb.literal.contains("creature::TIME_LORD"), "{}", sb.literal);
        assert!(
            read(
                "Name:Scout Token\nColors:white\nTypes:Creature Time Lord\nPT:2/2\nK:Soulbond\nK:Soulbond",
                &c
            )
            .is_none(),
            "duplicate Soulbond on token must be refused"
        );
        let map_hash = fnv1a_32_bare(
            "activated!(cost!(\"{1}\", TapSelf, SacrificeSelf), &[Effect::Explore { \
             target: TargetSpec::Object(&Filter::YOUR_CREATURE) }], \
             timing = ActivationTiming::SorcerySpeed, \
             target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)))",
        );
        assert!(is_legacy_unsuffixed_ability_token("MAP", map_hash));
    }

    #[test]
    fn wave15a_slice_statics_and_impending_gift_refusals() {
        let c = cats();
        let mut abilities =
            vec!["static_ability!(Filter::This, Modifier::GrantAbilities(&SLICE1))".into()];
        inline_statics(
            "static SLICE1: [AbilityDef; 1] = [activated!(Cost::TAP, &[Effect::draw(1)])];\n",
            &mut abilities,
        )
        .expect("inline slice static");
        assert_eq!(
            abilities[0],
            "static_ability!(Filter::This, Modifier::GrantAbilities(&[activated!(Cost::TAP, &[Effect::draw(1)])]))"
        );

        for unsupported in [
            "Name:ImpToken\nColors:red\nTypes:Creature Giant\nPT:4/4\nK:Impending:3:1 R",
            "Name:GiftToken\nColors:blue\nTypes:Creature Fish\nPT:1/1\nK:Gift\n\
             SVar:GiftAbility:DB$ Draw | Defined$ Promised | GiftDescription$ a card",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave16a_keyword_token_rules_and_refusals() {
        let c = cats();
        let exploit_tok = read(
            "Name:ExploitToken\nColors:black\nTypes:Creature Soldier\nPT:1/1\nK:Exploit",
            &c,
        )
        .expect("single Exploit token");
        assert!(
            exploit_tok.literal.contains("KeywordSet::EXPLOIT"),
            "{}",
            exploit_tok.literal
        );

        let evolve_tok = read(
            "Name:EvolveToken\nColors:green\nTypes:Creature Bird\nPT:0/1\nK:Evolve",
            &c,
        )
        .expect("single Evolve token");
        assert!(
            evolve_tok.literal.contains("KeywordSet::EVOLVE"),
            "{}",
            evolve_tok.literal
        );

        for unsupported in [
            "Name:WarpToken\nColors:blue\nTypes:Creature Bird\nPT:2/2\nK:Warp:1 U",
            "Name:DupExploit\nColors:black\nTypes:Creature Soldier\nPT:1/1\nK:Exploit\nK:Exploit",
            "Name:DupEvolve\nColors:green\nTypes:Creature Bird\nPT:0/1\nK:Evolve\nK:Evolve",
            "Name:DevourToken\nColors:red\nTypes:Creature Soldier\nPT:3/3\nK:Devour:2",
            "Name:SpreeToken\nColors:red\nTypes:Creature Soldier\nPT:1/1\nK:Spree",
            "Name:CraftToken\nColors:white\nTypes:Artifact\nK:Craft:2 ExileCtrlOrGrave<1/Artifact.Other>",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave17a_riot_unleash_and_retrace_refusals() {
        let c = cats();
        let riot_tok = read(
            "Name:RiotToken\nColors:red,green\nTypes:Creature Soldier\nPT:2/2\nK:Riot",
            &c,
        )
        .expect("single Riot token");
        assert!(
            riot_tok.literal.contains("KeywordSet::RIOT"),
            "{}",
            riot_tok.literal
        );

        let unleash_tok = read(
            "Name:UnleashToken\nColors:black,red\nTypes:Creature Soldier\nPT:1/1\nK:Unleash",
            &c,
        )
        .expect("single Unleash token");
        assert!(
            unleash_tok.literal.contains("KeywordSet::UNLEASH"),
            "{}",
            unleash_tok.literal
        );

        for unsupported in [
            "Name:RetraceToken\nColors:green\nTypes:Creature Soldier\nPT:2/2\nK:Retrace",
            "Name:DupRiot\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Riot\nK:Riot",
            "Name:DupUnleash\nColors:black\nTypes:Creature Soldier\nPT:1/1\nK:Unleash\nK:Unleash",
            "Name:BadRiot\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Riot:2",
            "Name:BadUnleash\nColors:black\nTypes:Creature Soldier\nPT:1/1\nK:Unleash:1",
            "Name:LureToken\nColors:green\nTypes:Creature Soldier\nPT:3/3\nK:CARDNAME must be blocked if able.",
            "Name:MayFlashToken\nColors:blue\nTypes:Creature Soldier\nPT:1/1\nK:MayFlashCost:2",
            "Name:PartnerSelToken\nColors:white\nTypes:Creature Soldier\nPT:2/2\nK:Partner:Character select",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave18c_umbra_firebending_afterlife_afflict_mobilize_tokens() {
        let c = cats();
        let mask = read(
            "Name:Mask\nColors:white\nTypes:Enchantment Aura\n\
             K:Enchant:Permanent:permanent\nK:Umbra armor",
            &c,
        )
        .expect("Umbra armor Aura token");
        assert!(
            mask.literal.contains("KeywordSet::UMBRA_ARMOR")
                && mask.literal.contains("Effect::AttachSelf"),
            "{}",
            mask.literal
        );

        let fb1 = read(
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Firebending:1",
            &c,
        )
        .expect("Firebending 1 token");
        assert_eq!(fb1.constant, "SOLDIER_2_2_RED_FIREBENDING_1");
        assert!(
            fb1.literal.contains("KeywordSet::FIREBENDING")
                && fb1.literal.contains("Effect::Firebend(Amount::Fixed(1))"),
            "{}",
            fb1.literal
        );

        let fb4 = read(
            "Name:Bird Token\nColors:red\nTypes:Creature Bird\nPT:4/4\nK:Flying\nK:Firebending:4",
            &c,
        )
        .expect("Flying Firebending 4 token");
        assert_eq!(fb4.constant, "BIRD_4_4_RED_FIREBENDING_4_FLYING");
        assert!(
            fb4.literal
                .contains("KeywordSet::FIREBENDING.union(KeywordSet::FLYING)")
                && fb4.literal.contains("Effect::Firebend(Amount::Fixed(4))"),
            "{}",
            fb4.literal
        );

        let afterlife = read(
            "Name:Pest Token\nColors:black\nTypes:Creature Pest\nPT:1/1\nK:Afterlife:1",
            &c,
        )
        .expect("Afterlife 1 token");
        assert_eq!(afterlife.constant, "PEST_1_1_BLACK_AFTERLIFE_1");
        assert!(
            afterlife.literal.contains("KeywordSet::AFTERLIFE")
                && afterlife
                    .literal
                    .contains("token: &SPIRIT_1_1_WHITE_BLACK_FLYING"),
            "{}",
            afterlife.literal
        );

        let afflict = read(
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Afflict:2",
            &c,
        )
        .expect("Afflict 2 token");
        assert_eq!(afflict.constant, "SOLDIER_2_2_RED_AFFLICT_2");
        assert!(
            afflict.literal.contains("KeywordSet::AFFLICT")
                && afflict.literal.contains("Trigger::BecomesBlocked"),
            "{}",
            afflict.literal
        );

        let mobilize = read(
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Mobilize:2",
            &c,
        )
        .expect("Mobilize 2 token");
        assert_eq!(mobilize.constant, "SOLDIER_2_2_RED_MOBILIZE_2");
        assert!(
            mobilize.literal.contains("KeywordSet::MOBILIZE")
                && mobilize.literal.contains("Effect::mobilize(2)"),
            "{}",
            mobilize.literal
        );

        for unsupported in [
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Firebending:X\nSVar:X:Count$CardPower",
            "Name:Soldier Token\nColors:red\nTypes:Creature Soldier\nPT:2/2\nK:Mobilize:X\nSVar:X:Count$Valid Creature.YouCtrl",
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:0/0\nK:Graft:1",
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:0/0\nK:Sunburst",
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:2/2\nK:Tribute:2",
            "Name:Soldier Token\nColors:blue\nTypes:Creature Soldier\nPT:2/2\nK:Sneak:1 U",
            "Name:Soldier Token\nColors:blue\nTypes:Creature Soldier\nPT:2/2\nK:Transmute:1 U U",
        ] {
            assert!(read(unsupported, &c).is_none(), "{unsupported}");
        }
    }

    #[test]
    fn wave19c_token_static_keywords_and_nested_refs() {
        let c = cats();
        let may_not_untap = read(
            "Name:Soldier Token\nColors:blue\nTypes:Creature Soldier\nPT:2/2\n\
             K:You may choose not to untap CARDNAME during your untap step.",
            &c,
        )
        .expect("may choose not to untap token");
        assert_eq!(may_not_untap.constant, "SOLDIER_2_2_BLUE_MAY_NOT_UNTAP");
        assert!(
            may_not_untap
                .literal
                .contains("Modifier::MayChooseNotToUntap"),
            "{}",
            may_not_untap.literal
        );

        let all_block = read(
            "Name:Soldier Token\nColors:green\nTypes:Creature Soldier\nPT:3/3\n\
             K:All creatures able to block CARDNAME do so.",
            &c,
        )
        .expect("must be blocked by all able token");
        assert_eq!(
            all_block.constant,
            "SOLDIER_3_3_GREEN_MUST_BE_BLOCKED_BY_ALL"
        );
        assert!(
            all_block
                .literal
                .contains("Modifier::MustBeBlockedByAllAble"),
            "{}",
            all_block.literal
        );

        let mut nested = vec![
            "triggered!(Trigger::ETB, &[Effect::create_token(&generated_tokens::ZOMBIE_2_2_BLACK, 1)])"
                .to_string(),
            "triggered!(Trigger::ETB, &[Effect::create_token(&generated_tokens::SKELETON_1_1_BLACK, 1)])"
                .to_string(),
        ];
        assert!(rewrite_nested_token_refs(&mut nested));
        assert!(nested[0].contains("&tokens::ZOMBIE_2_2_BLACK"));
        assert!(nested[1].contains("&SKELETON_1_1_BLACK"));
    }

    #[test]
    fn wave20c_narrowed_equip_token_and_refusals() {
        let c = cats();
        let blade = read(
            "Name:Banner\nTypes:Artifact Equipment\n\
             K:Equip:1:Soldier.YouCtrl:Soldier",
            &c,
        )
        .expect("narrowed Equip Equipment token");
        assert!(
            blade.literal.contains("Effect::AttachSelf")
                && blade.literal.contains("creature::SOLDIER")
                && blade.literal.contains("Filter::ControlledByYou")
                && !blade.literal.contains("EQUIP1"),
            "{}",
            blade.literal
        );
        assert!(
            read(
                "Name:BadPwGear\nTypes:Artifact Equipment\n\
                 K:Equip:1:Planeswalker.YouCtrl:planeswalker",
                &c,
            )
            .is_none()
        );
    }

    #[test]
    fn wave22c_planeswalker_loyalty_and_double_faced_tokens() {
        let mut c = cats();
        c.planeswalker.push("Jace".into());
        c.artifact.push("Incubator".into());
        c.creature.push("Phyrexian".into());
        c.creature.push("Drone".into());
        c.normalize();

        let pw_plain = read(
            "Name:Token\nManaCost:no cost\nColors:blue\nTypes:Planeswalker\nLoyalty:0\n\
             A:AB$ Surveil | Cost$ SubCounter<1/LOYALTY> | Amount$ 1 | Planeswalker$ True\n\
             A:AB$ Draw | Cost$ SubCounter<3/LOYALTY> | Planeswalker$ True | Defined$ You | NumCards$ 1\n\
             Oracle:[-1]: Surveil 1.\\n[-3]: Draw a card.",
            &c,
        )
        .expect("planeswalker token without subtype");
        assert!(pw_plain.constant.starts_with("TOKEN_L0_BLUE_A"));
        assert!(pw_plain.literal.contains("types: TypeSet::PLANESWALKER,"));
        assert!(pw_plain.literal.contains("loyalty: Some(0),"));
        assert!(pw_plain.literal.contains("Effect::surveil(1)"));
        assert!(pw_plain.literal.contains("Effect::draw(1)"));

        let pw_jace = read(
            "Name:Jace Token\nManaCost:no cost\nColors:blue\nTypes:Planeswalker Jace\nLoyalty:0\n\
             A:AB$ Surveil | Cost$ SubCounter<1/LOYALTY> | Amount$ 1 | Planeswalker$ True\n\
             A:AB$ Draw | Cost$ SubCounter<3/LOYALTY> | Planeswalker$ True | Defined$ You | NumCards$ 1\n\
             Oracle:[-1]: Surveil 1.\\n[-3]: Draw a card.",
            &c,
        )
        .expect("planeswalker token with subtype");
        assert!(pw_jace.constant.starts_with("JACE_L0_BLUE_A"));
        assert_ne!(pw_plain.constant, pw_jace.constant);
        assert!(pw_jace.literal.contains("subtypes: &[planeswalker::JACE],"));
        assert_eq!(pw_jace.modules, ["planeswalker"]);

        let dfc = read(
            "Name:Incubator Token\nManaCost:no cost\nTypes:Artifact Incubator\n\
             A:AB$ SetState | Cost$ 2 | Mode$ Transform\n\
             AlternateMode:DoubleFaced\nOracle:{2}: Transform this token.\n\n\
             ALTERNATE\n\n\
             Name:Phyrexian Token\nManaCost:no cost\nTypes:Artifact Creature Phyrexian\n\
             PT:0/0\nOracle:",
            &c,
        )
        .expect("double-faced Incubator // Phyrexian token");
        assert!(
            dfc.constant.contains("_DFC_PHYREXIAN_ARTIFACT_0_0"),
            "constant: {}",
            dfc.constant
        );
        assert_eq!(
            dfc.doc,
            "colorless Incubator // 0/0 colorless artifact Phyrexian."
        );
        assert!(dfc.literal.contains("Effect::TransformSource"));
        assert!(dfc.literal.contains("back_face: Some(&TokenDef {"));
        assert!(dfc.literal.contains("subtypes: &[creature::PHYREXIAN],"));
        assert!(
            dfc.modules.iter().any(|m| m == "artifact")
                && dfc.modules.iter().any(|m| m == "creature"),
            "modules: {:?}",
            dfc.modules
        );

        // Verify double-faced and planeswalker tokens round-trip through the ledger cleanly.
        let entries = crate::tokenledger::assign(Vec::new(), &[], &[pw_jace, dfc.clone()])
            .expect("ledger assign");
        let rendered = crate::tokenledger::render(&entries);
        let reparsed = crate::tokenledger::parse(&rendered).expect("ledger parse");
        assert_eq!(reparsed.len(), 2);
        crate::tokenledger::assign(reparsed, &[], &[dfc]).expect("ledger idempotent");

        // Battlefield-origin ChangesZone with omitted Destination$ reads as LeavesBattlefield.
        let ltb_drone = read(
            "Name:Drone Token\nManaCost:no cost\nTypes:Artifact Creature Drone\nPT:2/2\n\
             K:Deathtouch\n\
             T:Mode$ ChangesZone | Origin$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDrain\n\
             SVar:TrigDrain:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 2",
            &c,
        )
        .expect("LTB drone token with omitted Destination$");
        assert!(
            ltb_drone
                .literal
                .contains("Trigger::LeavesBattlefield(&Filter::This)"),
            "{}",
            ltb_drone.literal
        );

        // Refusals: invalid/missing/duplicate Loyalty or Defense, real ManaCost, malformed DFC.
        for refused in [
            "Name:NoLoyalty\nColors:blue\nTypes:Planeswalker",
            "Name:StarLoyalty\nColors:blue\nTypes:Planeswalker\nLoyalty:*",
            "Name:DupLoyalty\nColors:blue\nTypes:Planeswalker\nLoyalty:1\nLoyalty:2",
            "Name:LoyaltyRock\nTypes:Artifact\nLoyalty:3",
            "Name:DupDefense\nTypes:Battle\nDefense:3\nDefense:4",
            "Name:CostedToken\nManaCost:1 B\nColors:black\nTypes:Creature Soldier\nPT:2/1",
            "Name:MissingBack\nTypes:Artifact Incubator\nAlternateMode:DoubleFaced",
            "Name:MissingMode\nTypes:Artifact Incubator\nALTERNATE\nName:Phyrexian Token\nTypes:Artifact Creature Phyrexian\nPT:0/0",
            "Name:CostedBack\nTypes:Artifact Incubator\nAlternateMode:DoubleFaced\nALTERNATE\nName:Phyrexian Token\nManaCost:1 B\nTypes:Artifact Creature Phyrexian\nPT:2/1",
            "Name:ThreeFaces\nTypes:Artifact Incubator\nAlternateMode:DoubleFaced\nALTERNATE\nName:Phyrexian Token\nTypes:Artifact Creature Phyrexian\nPT:0/0\nALTERNATE\nName:Third Token\nTypes:Artifact",
            "Name:NestedDfc\nTypes:Artifact Incubator\nAlternateMode:DoubleFaced\nALTERNATE\nName:Phyrexian Token\nTypes:Artifact Creature Phyrexian\nPT:0/0\nAlternateMode:DoubleFaced",
        ] {
            assert!(read(refused, &c).is_none(), "must refuse:\n{refused}");
        }
    }
}
