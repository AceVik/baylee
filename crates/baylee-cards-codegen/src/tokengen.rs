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

/// The two lines a token script carries that say nothing this reads.
///
/// `ManaCost` is always "no cost" and `Oracle` is the printed reminder text,
/// which this side has no use for — a token has no printing to hold a header
/// against. Every other head refuses the token, and each of them is a real
/// shape in the reference: `Text` is rules text ("this creature is all
/// colors"), `Loyalty` is a planeswalker token's starting loyalty, which
/// this reader does not yet transcode, and `AlternateMode` is a second face.
const IGNORED: &[&str] = &["ManaCost", "Oracle"];

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
    defense: Option<u16>,
    keywords: Vec<&'static str>,
    ability_keywords: Vec<String>,
    text_all_colors: bool,
    has_rule_lines: bool,
}

fn parse_token_lines(text: &str) -> Option<RawToken> {
    let (mut name, mut colors, mut types, mut pt) = (None, None, None, None);
    let mut defense = None;
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
            "Colors" => colors = Some(rest.trim().to_string()),
            "Types" => types = Some(rest.trim().to_string()),
            "PT" => pt = Some(rest.trim().to_string()),
            "Defense" => defense = Some(rest.trim().parse::<u16>().ok()?),
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
        defense,
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
        _ => None,
    }
}

fn render_token_literal(
    name: &str,
    colors: Option<&Colors>,
    types: &TypeLine,
    pt: Option<(i16, i16)>,
    defense: Option<u16>,
    keywords: &[&str],
    abilities: &[String],
) -> String {
    let mut literal = String::with_capacity(256);
    literal.push_str("TokenDef {\n");
    let _ = writeln!(literal, "    name: {:?},", token_name(name));
    if let Some(colors) = colors
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
    if let Some((p, t)) = pt {
        let _ = writeln!(literal, "    power: Some({p}),");
        let _ = writeln!(literal, "    toughness: Some({t}),");
    }
    if let Some(defense) = defense {
        let _ = writeln!(literal, "    defense: Some({defense}),");
    }
    if !keywords.is_empty() {
        let _ = writeln!(literal, "    keywords: {},", union_of(keywords));
    }
    if !abilities.is_empty() {
        let _ = writeln!(literal, "    abilities: &[{}],", abilities.join(", "));
    }
    literal.push_str("    ..TokenDef::DEFAULT\n}");
    literal
}

fn token_shape_ok(raw: &RawToken, types: &TypeLine) -> bool {
    if types.set.contains("TypeSet::BATTLE") != raw.defense.is_some() {
        return false;
    }
    let is_equipment =
        !types.is_creature && types.subtypes.iter().any(|s| s == "artifact::EQUIPMENT");
    if !is_equipment && raw.ability_keywords.iter().any(|k| k.starts_with("Equip:")) {
        return false;
    }
    // CR 303.7 Role uniqueness SBA is not yet enforced in the engine; keep
    // Role Aura tokens refused while allowing plain Aura tokens.
    let is_non_role_aura = !types.is_creature
        && types.subtypes.iter().any(|s| s == "enchantment::AURA")
        && !types.subtypes.iter().any(|s| s == "enchantment::ROLE");
    is_non_role_aura
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
    let raw = parse_token_lines(text)?;
    let mut types = type_line(&raw.types, cats)?;
    if !token_shape_ok(&raw, &types) {
        return None;
    }
    let is_vehicle = !types.is_creature && types.subtypes.iter().any(|s| s == "artifact::VEHICLE");
    let (power, toughness, star_pt) = match (raw.pt.as_deref(), types.is_creature, is_vehicle) {
        (Some("*/*"), true, false) => (None, None, true),
        (Some(pt), true, false) | (Some(pt), false, true) => {
            let (p, t) = pt.split_once('/')?;
            let p = p.trim().parse::<i16>().ok()?;
            let t = t.trim().parse::<i16>().ok()?;
            (Some(p), Some(t), false)
        }
        (None, false, false) => (None, None, false),
        // A creature or Vehicle with no size, a Vehicle with `*/*` and no CDA,
        // or a size on something that is neither a creature nor a Vehicle:
        // refuse rather than guessing.
        _ => return None,
    };
    let colors = match raw.colors.as_deref() {
        Some(raw_colors) => Some(color_line(raw_colors)?),
        None => None,
    };
    if raw.text_all_colors && colors.as_ref().is_none_or(|c| c.words.len() != 5) {
        return None;
    }
    let abilities = abilities(
        text,
        &raw.ability_keywords,
        raw.text_all_colors,
        cats,
        tokens,
        &mut types.modules,
    )?;
    if star_pt
        && !abilities
            .iter()
            .any(|a| a.contains("Modifier::CharacteristicPT"))
    {
        return None;
    }
    let literal = render_token_literal(
        &raw.name,
        colors.as_ref(),
        &types,
        power.zip(toughness),
        raw.defense,
        &raw.keywords,
        &abilities,
    );
    let mut keyword_tags: Vec<String> = raw
        .keywords
        .iter()
        .map(|k| k.trim_start_matches("KeywordSet::").to_string())
        .collect();
    for kw in &raw.ability_keywords {
        keyword_tags.push(ability_keyword_slug(kw)?);
    }
    keyword_tags.sort_unstable();
    let mut constant = constant_name(
        &raw.name,
        &types.qualifiers,
        power,
        toughness,
        star_pt,
        colors.as_ref(),
        &keyword_tags,
    );
    if raw.has_rule_lines && !abilities.is_empty() {
        let hash = fnv1a_32_bare(&abilities.join(","));
        if !is_legacy_unsuffixed_ability_token(&constant, hash) {
            let _ = write!(constant, "_A{hash:08X}");
        }
    }
    Some(TokenBody {
        constant,
        doc: doc_line(
            &raw.name,
            &types.qualifiers,
            power,
            toughness,
            star_pt,
            colors.as_ref(),
            &keyword_tags,
        ),
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
    match kw {
        "Exalted" => Some("EXALTED".to_string()),
        "Prevent all damage that would be dealt to CARDNAME." => Some("PREVENT_DAMAGE".to_string()),
        "CARDNAME can only attack alone." => Some("ATTACKS_ALONE".to_string()),
        _ => None,
    }
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
        || matches!(
            kw,
            "Exalted"
                | "Prevent all damage that would be dealt to CARDNAME."
                | "CARDNAME can only attack alone."
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
    "BLOOD",
    "CLUE",
    "FOOD",
    "JUNK",
    "MAP",
    "POWERSTONE",
    "TREASURE",
    "VIBRANIUM",
];

/// Generated `pub static` token constants already frozen in the append-only
/// `crates/cards/src/generated_tokens.rs` ledger, so another generated token's
/// ability may safely rewrite `&generated_tokens::<NAME>` -> `&<NAME>`.
const GENERATED_NESTED_TOKENS: &[&str] = &[
    "DRAGON_2_2_RED_FLYING",
    "KRAKEN_9_9_BLUE",
    "RABBIT_1_1_WHITE",
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
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
    modules: &mut Vec<String>,
) -> Option<Vec<String>> {
    let cleaned;
    let source = if strip_all_colors_text {
        cleaned = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("Text:"))
            .collect::<Vec<_>>()
            .join("\n");
        cleaned.as_str()
    } else {
        text
    };
    let mut script = crate::scriptgen::parse(source);
    if script.rules.is_empty() && script.svars.is_empty() && ability_keywords.is_empty() {
        return Some(Vec::new());
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
    if !body.enter_modifiers.is_empty()
        || !body.alternative_costs.is_empty()
        || !body.mandatory_additional_costs.is_empty()
        || !body.keywords.is_empty()
        || body.convoke
        || body.delve
        || body.improvise
        || body.replicate.is_some()
        || body.disguise.is_some()
        || body.flashback.is_some()
        || !body.flashback_parts.is_empty()
        || body.madness.is_some()
        || body.foretell.is_some()
        || body.morph.is_some()
        || body.megamorph.is_some()
        || !body.morph_parts.is_empty()
        || body.bestow.is_some()
        || body.jump_start
        || body.aftermath
        || body.disturb.is_some()
        || body.escape.is_some()
        || body.squad.is_some()
        || body.offspring.is_some()
        || body.fuse
        || body.cost_reduction.is_some()
        || body.entwine.is_some()
        || body.buyback.is_some()
        || body.multikicker.is_some()
        || !body.additional_costs.is_empty()
        || body.kicked_targets.is_some()
        || body.has_partner
        || body.back.is_some()
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
    Some(abilities)
}

fn inline_statics(statics: &str, abilities: &mut [String]) -> Option<()> {
    let mut defs: Vec<(&str, &str)> = Vec::new();
    for line in statics.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rest = trimmed.strip_prefix("static ")?;
        let (name, expr) = rest
            .split_once(": Filter = ")
            .or_else(|| rest.split_once(": Condition = "))
            .or_else(|| rest.split_once(": AbilityDef = "))?;
        let expr = expr.strip_suffix(';')?;
        defs.push((name.trim(), expr.trim()));
    }
    if defs.is_empty() {
        return None;
    }
    for _ in 0..4 {
        let mut changed = false;
        for ability in abilities.iter_mut() {
            for &(name, expr) in &defs {
                if let Some(updated) = replace_ref_ident(ability, name, expr) {
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

fn replace_ref_ident(text: &str, ident: &str, replacement: &str) -> Option<String> {
    let needle = format!("&{ident}");
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut replaced = false;
    while let Some(rel) = text[cursor..].find(&needle) {
        let start = cursor + rel;
        let end = start + needle.len();
        let next_is_ident = bytes
            .get(end)
            .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
        if next_is_ident {
            out.push_str(&text[cursor..end]);
        } else {
            out.push_str(&text[cursor..start]);
            out.push('&');
            out.push_str(replacement);
            replaced = true;
        }
        cursor = end;
    }
    if !replaced {
        return None;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

fn rewrite_subtypes(abilities: &mut [String], modules: &mut Vec<String>) -> Option<()> {
    const KNOWN: &[&str] = &[
        "artifact",
        "creature",
        "enchantment",
        "land",
        "planeswalker",
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
    star_pt: bool,
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
    if let (Some(p), Some(t)) = (power, toughness) {
        let _ = write!(out, "_{}_{}", pt_word(p), pt_word(t));
    } else if star_pt {
        out.push_str("_X_X");
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
    power: Option<i16>,
    toughness: Option<i16>,
    star_pt: bool,
    colors: Option<&Colors>,
    keyword_tags: &[String],
) -> String {
    let mut out = String::new();
    if let (Some(p), Some(t)) = (power, toughness) {
        let _ = write!(out, "{p}/{t} ");
    } else if star_pt {
        out.push_str("*/* ");
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

        // Role Aura tokens stay refused while CR 303.7 is unmodeled.
        assert!(
            read(
                "Name:Role Token\nTypes:Enchantment Aura Role\nK:Enchant:Creature",
                &cats()
            )
            .is_none()
        );
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
}
