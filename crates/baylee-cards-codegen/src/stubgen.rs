//! Scryfall card → per-card stub Rust file + registry tables.

// One-shot string rendering; the allocation lint adds noise, not value.
#![allow(clippy::format_push_string)]

use crate::catalog::SubtypeCatalogs;
use crate::error::CodegenError;
use crate::scryfall::{ScryfallCard, ScryfallFace};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};
use heck::ToSnakeCase;

/// Registry metadata for one generated stub.
#[derive(Debug, Clone)]
pub struct StubInfo {
    /// File/module slug (`lightning_bolt`).
    pub slug: String,
    /// Scryfall oracle id.
    pub oracle_id: String,
    /// Dense `CardIndex`.
    pub index: u32,
}

/// Filesystem-safe module slug for a card name.
#[must_use]
pub fn slug(name: &str) -> String {
    let snake = name.to_snake_case();
    let mut out = String::with_capacity(snake.len());
    for c in snake.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_').to_string();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return format!("c_{out}");
    }
    out
}

struct FaceData {
    name: String,
    mana_cost: String,
    type_line: String,
    oracle_text: String,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
}

impl FaceData {
    fn from_top(card: &ScryfallCard) -> Self {
        Self {
            name: card.name.clone(),
            mana_cost: card.mana_cost.clone().unwrap_or_default(),
            type_line: card.type_line.clone().unwrap_or_default(),
            oracle_text: card.oracle_text.clone().unwrap_or_default(),
            power: card.power.clone(),
            toughness: card.toughness.clone(),
            loyalty: card.loyalty.clone(),
        }
    }

    fn from_face(face: &ScryfallFace) -> Self {
        Self {
            name: face.name.clone(),
            mana_cost: face.mana_cost.clone().unwrap_or_default(),
            type_line: face.type_line.clone().unwrap_or_default(),
            oracle_text: face.oracle_text.clone().unwrap_or_default(),
            power: face.power.clone(),
            toughness: face.toughness.clone(),
            loyalty: face.loyalty.clone(),
        }
    }
}

fn faces_of(card: &ScryfallCard) -> Vec<FaceData> {
    if let Some(faces) = &card.card_faces
        && faces.len() >= 2
    {
        return faces.iter().map(FaceData::from_face).collect();
    }
    vec![FaceData::from_top(card)]
}

fn type_expr(line: &str, cats: &SubtypeCatalogs) -> (String, String, Vec<String>, Vec<String>) {
    let mut type_bits: Vec<&'static str> = Vec::new();
    let mut super_bits: Vec<&'static str> = Vec::new();
    let mut subtype_paths: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    let mut parts = line.splitn(2, '\u{2014}'); // em dash
    let left = parts.next().unwrap_or("");
    let right = parts.next().unwrap_or("");

    for word in left.split_whitespace() {
        if let Some(t) = TypeSet::from_word(word) {
            type_bits.push(type_const(t));
        } else if let Some(s) = SupertypeSet::from_word(word) {
            super_bits.push(super_const(s));
        } else {
            unknown.push(word.to_string());
        }
    }
    for word in right.split_whitespace() {
        match cats.const_path(word) {
            Some(path) => subtype_paths.push(path),
            None => unknown.push(word.to_string()),
        }
    }

    let types = if type_bits.is_empty() {
        "TypeSet::EMPTY".to_string()
    } else {
        join_union(&type_bits)
    };
    let supers = if super_bits.is_empty() {
        "SupertypeSet::EMPTY".to_string()
    } else {
        join_union(&super_bits)
    };
    (types, supers, subtype_paths, unknown)
}

fn join_union(bits: &[&str]) -> String {
    let mut out = bits[0].to_string();
    for b in &bits[1..] {
        out.push_str(&format!(".union({b})"));
    }
    out
}

fn type_const(t: TypeSet) -> &'static str {
    match t {
        TypeSet::ARTIFACT => "TypeSet::ARTIFACT",
        TypeSet::CREATURE => "TypeSet::CREATURE",
        TypeSet::ENCHANTMENT => "TypeSet::ENCHANTMENT",
        TypeSet::INSTANT => "TypeSet::INSTANT",
        TypeSet::KINDRED => "TypeSet::KINDRED",
        TypeSet::LAND => "TypeSet::LAND",
        TypeSet::PLANESWALKER => "TypeSet::PLANESWALKER",
        TypeSet::SORCERY => "TypeSet::SORCERY",
        TypeSet::BATTLE => "TypeSet::BATTLE",
        TypeSet::DUNGEON => "TypeSet::DUNGEON",
        TypeSet::PLANE => "TypeSet::PLANE",
        TypeSet::SCHEME => "TypeSet::SCHEME",
        TypeSet::VANGUARD => "TypeSet::VANGUARD",
        TypeSet::PHENOMENON => "TypeSet::PHENOMENON",
        TypeSet::CONSPIRACY => "TypeSet::CONSPIRACY",
        TypeSet::ATTRACTION => "TypeSet::ATTRACTION",
        _ => "TypeSet::EMPTY",
    }
}

fn super_const(s: SupertypeSet) -> &'static str {
    match s {
        SupertypeSet::BASIC => "SupertypeSet::BASIC",
        SupertypeSet::LEGENDARY => "SupertypeSet::LEGENDARY",
        SupertypeSet::SNOW => "SupertypeSet::SNOW",
        SupertypeSet::WORLD => "SupertypeSet::WORLD",
        SupertypeSet::ONGOING => "SupertypeSet::ONGOING",
        SupertypeSet::HOST => "SupertypeSet::HOST",
        _ => "SupertypeSet::EMPTY",
    }
}

fn mana_expr(card_name: &str, cost: &str) -> Result<String, CodegenError> {
    if cost.is_empty() {
        return Ok("ManaCost::ZERO".to_string());
    }
    ManaCost::try_parse(cost).map_err(|reason| CodegenError::Mana {
        card: card_name.to_string(),
        cost: cost.to_string(),
        reason,
    })?;
    Ok(format!("baylee_core::mana!(\"{cost}\")"))
}

fn pt_expr(value: Option<&str>) -> String {
    value
        .and_then(|v| v.parse::<i16>().ok())
        .map_or_else(|| "None".to_string(), |v| format!("Some({v})"))
}

fn loyalty_expr(value: Option<&str>) -> String {
    value
        .and_then(|v| v.parse::<u16>().ok())
        .map_or_else(|| "None".to_string(), |v| format!("Some({v})"))
}

fn color_identity_expr(identity: Option<&Vec<String>>) -> String {
    let Some(ids) = identity else {
        return "ColorSet::EMPTY".to_string();
    };
    if ids.is_empty() {
        return "ColorSet::EMPTY".to_string();
    }
    let colors: Vec<String> = ids
        .iter()
        .filter_map(|c| match c.as_str() {
            "W" => Some("Color::White"),
            "U" => Some("Color::Blue"),
            "B" => Some("Color::Black"),
            "R" => Some("Color::Red"),
            "G" => Some("Color::Green"),
            _ => None,
        })
        .map(str::to_string)
        .collect();
    format!("ColorSet::from_slice(&[{}])", colors.join(", "))
}

fn commander_rule(faces: &[FaceData]) -> &'static str {
    let type_line = faces.first().map_or("", |f| f.type_line.as_str());
    let oracle = faces
        .iter()
        .map(|f| f.oracle_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if oracle.contains("can be your commander") {
        "CommanderRule::ExplicitlyAllowed"
    } else if type_line.contains("Legendary") && type_line.contains("Creature") {
        "CommanderRule::Legendary"
    } else {
        "CommanderRule::NotEligible"
    }
}

fn partner_kind(faces: &[FaceData]) -> String {
    let oracle = faces
        .iter()
        .map(|f| f.oracle_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for line in oracle.lines() {
        if let Some(rest) = line.strip_prefix("Partner with ") {
            let name = rest.trim().trim_end_matches(['.', ',']);
            return format!(
                "PartnerKind::PartnerWith(\"{}\")",
                name.replace('"', "\\\"")
            );
        }
    }
    if oracle.contains("Doctor's companion") {
        "PartnerKind::DoctorsCompanion".to_string()
    } else if oracle.contains("Choose a Background") {
        "PartnerKind::ChooseABackground".to_string()
    } else if oracle.contains("Friends forever") {
        "PartnerKind::FriendsForever".to_string()
    } else if oracle
        .lines()
        .any(|l| l == "Partner" || l.starts_with("Partner ("))
    {
        "PartnerKind::Partner".to_string()
    } else {
        "PartnerKind::None".to_string()
    }
}

fn doc_lines(out: &mut String, prefix: &str, text: &str) {
    for line in text.lines() {
        out.push_str(&format!("//! {prefix}{line}\n"));
    }
}

/// Renders one stub file (`crates/baylee-cards/src/cards/<slug>.rs`).
///
/// # Errors
/// [`CodegenError::Mana`] when a mana cost fails validation.
pub fn render_stub(
    card: &ScryfallCard,
    index: u32,
    cats: &SubtypeCatalogs,
) -> Result<(StubInfo, String), CodegenError> {
    // Multi-face cards slug by their front face ("Brightclimb Pathway // …"
    // → "brightclimb_pathway").
    let slug = slug(card.name.split(" // ").next().unwrap_or(&card.name));
    let faces = faces_of(card);
    let oracle_id = card.oracle_id.clone().unwrap_or_default();

    let mut out = String::with_capacity(4096);
    // Human-verifiable header (docs/card-dsl.md).
    out.push_str(&format!("//! {} \u{2014} ", card.name));
    out.push_str(if card.mana_cost.as_deref().unwrap_or("").is_empty() {
        "(no cost)"
    } else {
        card.mana_cost.as_deref().unwrap_or("")
    });
    out.push_str(&format!(
        " \u{2014} {}\n",
        card.type_line.as_deref().unwrap_or("")
    ));
    doc_lines(
        &mut out,
        "Oracle: ",
        &card.oracle_text.clone().unwrap_or_default(),
    );
    out.push_str(&format!(
        "//! Set: {} #{} \u{2014} {} | Scryfall ID: {} | Oracle ID: {}\n",
        card.set.as_deref().unwrap_or("?").to_uppercase(),
        card.collector_number.as_deref().unwrap_or("?"),
        card.set_name.as_deref().unwrap_or("?"),
        card.id,
        oracle_id,
    ));
    if faces.len() > 1 {
        for f in &faces {
            out.push_str(&format!(
                "//! Face: {} \u{2014} {} \u{2014} {}\n",
                f.name, f.mana_cost, f.type_line
            ));
        }
    }
    out.push_str("// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.\n#![allow(unused_imports, missing_docs)]\n\n");
    out.push_str(
        "use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};\n\
         use baylee_core::color::{Color, ColorSet};\n\
         use baylee_core::generated::subtypes;\n\
         use baylee_core::ids::CardIndex;\n\
         use baylee_core::mana::ManaCost;\n\
         use baylee_core::types::{SupertypeSet, TypeSet};\n\n",
    );

    let mut face_defs = String::new();
    for f in &faces {
        let (types, supers, subtype_paths, unknown) = type_expr(&f.type_line, cats);
        let mana = mana_expr(&card.name, &f.mana_cost)?;
        let subtypes = if subtype_paths.is_empty() {
            "&[]".to_string()
        } else {
            format!("&[{}]", subtype_paths.join(", "))
        };
        face_defs.push_str(&format!(
            "    FaceDef {{\n        name: {:?},\n        mana_cost: {mana},\n        types: {types},\n        supertypes: {supers},\n        subtypes: {subtypes},\n        power: {},\n        toughness: {},\n        loyalty: {},\n        alternative_costs: &[],\n        additional_costs: &[],\n        mandatory_additional_costs: &[],\n    }},\n",
            f.name,
            pt_expr(f.power.as_deref()),
            pt_expr(f.toughness.as_deref()),
            loyalty_expr(f.loyalty.as_deref()),
        ));
        for u in unknown {
            face_defs.push_str(&format!(
                "    // FIXME(codegen): unknown type-line word {u:?}\n"
            ));
        }
    }

    out.push_str(&format!(
        "pub static CARD: CardDef = CardDef {{\n    index: CardIndex::new({index}),\n    oracle_id: {:?},\n    scryfall_id: {:?},\n    faces: &[\n{face_defs}    ],\n    color_identity: {},\n    keywords: KeywordSet::EMPTY,\n    commander: {},\n    partner: {},\n    coverage: Coverage::Unimplemented,\n    abilities: &[],\n}};\n\n",
        oracle_id,
        card.id,
        color_identity_expr(card.color_identity.as_ref()),
        commander_rule(&faces),
        partner_kind(&faces),
    ));
    out.push_str(
        "#[cfg(test)]\nmod tests {\n    // TODO(card): implement abilities + tests, see docs/card-dsl.md.\n}\n",
    );

    Ok((
        StubInfo {
            slug,
            oracle_id,
            index,
        },
        out,
    ))
}

/// Renders `crates/baylee-cards/src/cards/mod.rs`.
#[must_use]
pub fn render_cards_mod(stubs: &[StubInfo]) -> String {
    let mut out = String::from(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n//! One module per registered card.\n\n",
    );
    let mut slugs: Vec<&str> = stubs.iter().map(|s| s.slug.as_str()).collect();
    slugs.sort_unstable();
    for s in slugs {
        out.push_str(&format!("pub mod {s};\n"));
    }
    out
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Renders `crates/baylee-cards/src/generated.rs` (registry tables).
#[must_use]
pub fn render_registry(stubs: &[StubInfo]) -> String {
    let mut by_oracle: Vec<&StubInfo> = stubs.iter().collect();
    by_oracle.sort_by(|a, b| a.oracle_id.cmp(&b.oracle_id));
    let mut by_index: Vec<&StubInfo> = stubs.iter().collect();
    by_index.sort_by_key(|s| s.index);

    let mut hash = FNV_OFFSET;
    for s in &by_oracle {
        hash = fnv1a(hash, s.oracle_id.as_bytes());
        hash = fnv1a(hash, s.slug.as_bytes());
    }

    let mut out = String::from(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\n#![allow(missing_docs, unused_imports, dead_code, clippy::all, clippy::pedantic)]\n\nuse baylee_cards_dsl::CardDef;\nuse baylee_core::ids::CardIndex;\n\n",
    );
    out.push_str("/// All registered cards, sorted by oracle id for binary search.\npub static ALL: &[(&str, &CardDef)] = &[\n");
    for s in &by_oracle {
        out.push_str(&format!(
            "    ({:?}, &crate::cards::{}::CARD),\n",
            s.oracle_id, s.slug
        ));
    }
    out.push_str("];\n\n/// All registered cards, ordered by `CardIndex`.\npub static BY_INDEX: &[&CardDef] = &[\n");
    for s in &by_index {
        out.push_str(&format!("    &crate::cards::{}::CARD,\n", s.slug));
    }
    out.push_str(&format!(
        "];\n\n/// FNV-1a hash over the registry content.\npub const POOL_HASH: u64 = {hash:#x};\n\n"
    ));
    out.push_str(
        "pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {\n    ALL.binary_search_by(|(id, _)| (*id).cmp(oracle_id))\n        .ok()\n        .map(|i| ALL[i].1)\n}\n\npub fn by_index(index: CardIndex) -> Option<&'static CardDef> {\n    BY_INDEX.get(index.get() as usize).copied()\n}\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Lightning Bolt"), "lightning_bolt");
        assert_eq!(
            slug("Aminatou, the Fateshifter"),
            "aminatou_the_fateshifter"
        );
        assert_eq!(
            slug("Jin-Gitaxias, Progress Tyrant"),
            "jin_gitaxias_progress_tyrant"
        );
    }
}
