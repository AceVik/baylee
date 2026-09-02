//! Scryfall card → per-card stub Rust file + registry tables.

// One-shot string rendering; the allocation lint adds noise, not value.
#![allow(clippy::format_push_string)]

use crate::body::CardBody;
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

/// Appends `name: value` unless `value` is what the struct-update tail
/// would supply anyway (an empty `value` always counts as the default).
fn push_field(fields: &mut Vec<String>, name: &str, value: &str, default: &str) {
    if value.is_empty() || value == default {
        return;
    }
    fields.push(format!("{name}: {value}"));
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

/// Renders one `FaceDef` literal.
///
/// Only fields that differ from [`baylee_cards_dsl::FaceDef::DEFAULT`] are
/// written out; the rest come from the struct-update tail. A stub therefore
/// says exactly what is printed on the face and nothing else, and a new
/// `FaceDef` field does not have to be back-filled into every card file.
fn render_face(
    card_name: &str,
    f: &FaceData,
    cats: &SubtypeCatalogs,
    enter_modifiers: &[String],
) -> Result<String, CodegenError> {
    let (types, supers, subtype_paths, unknown) = type_expr(&f.type_line, cats);
    let subtypes = if subtype_paths.is_empty() {
        String::new()
    } else {
        format!("&[{}]", subtype_paths.join(", "))
    };
    let mut fields = vec![format!("name: {:?}", f.name)];
    push_field(
        &mut fields,
        "mana_cost",
        &mana_expr(card_name, &f.mana_cost)?,
        "ManaCost::ZERO",
    );
    fields.push(format!("types: {types}"));
    push_field(&mut fields, "supertypes", &supers, "SupertypeSet::EMPTY");
    push_field(&mut fields, "subtypes", &subtypes, "");
    push_field(&mut fields, "power", &pt_expr(f.power.as_deref()), "None");
    push_field(
        &mut fields,
        "toughness",
        &pt_expr(f.toughness.as_deref()),
        "None",
    );
    push_field(
        &mut fields,
        "loyalty",
        &loyalty_expr(f.loyalty.as_deref()),
        "None",
    );
    if !enter_modifiers.is_empty() {
        fields.push(format!(
            "enter_modifiers: &[{}]",
            enter_modifiers.join(", ")
        ));
    }

    let mut out = String::from("    face! {\n");
    for field in &fields {
        out.push_str(&format!("        {field},\n"));
    }
    out.push_str("    },\n");
    for u in unknown {
        out.push_str(&format!(
            "    // FIXME(codegen): unknown type-line word {u:?}\n"
        ));
    }
    Ok(out)
}

/// Renders the `pub static CARD: CardDef = …` literal.
///
/// `coverage` is emitted only for a card [`landgen::recognize`] read in full;
/// otherwise it is left out entirely, because `CardDef::DEFAULT` is
/// [`baylee_cards_dsl::Coverage::Unimplemented`] and a stub must not pass for
/// playable until somebody writes the line by hand.
fn render_card_literal(
    card: &ScryfallCard,
    index: u32,
    oracle_id: &str,
    faces: &[FaceData],
    face_defs: &str,
    land: Option<&CardBody>,
) -> String {
    let mut fields = vec![
        format!("index: {index}"),
        format!("oracle_id: {oracle_id:?}"),
        format!("scryfall_id: {:?}", card.id),
    ];
    push_field(
        &mut fields,
        "color_identity",
        &color_identity_expr(card.color_identity.as_ref()),
        "ColorSet::EMPTY",
    );
    push_field(
        &mut fields,
        "commander",
        commander_rule(faces),
        "CommanderRule::NotEligible",
    );
    push_field(
        &mut fields,
        "partner",
        &partner_kind(faces),
        "PartnerKind::None",
    );

    if let Some(land) = land {
        if !land.keywords.is_empty() {
            fields.push(format!("keywords: {}", join_union_owned(&land.keywords)));
        }
        fields.push("coverage: Coverage::Implemented".to_string());
    }

    let mut out = String::from("card! {\n");
    for field in &fields {
        out.push_str(&format!("    {field},\n"));
    }
    out.push_str(&format!("    faces: &[\n{face_defs}    ],\n"));
    if let Some(land) = land
        && !land.abilities.is_empty()
    {
        out.push_str("    abilities: &[\n");
        for ability in &land.abilities {
            out.push_str(&format!("        {ability},\n"));
        }
        out.push_str("    ],\n");
    }
    out.push_str("}\n\n");
    out
}

fn join_union_owned(bits: &[String]) -> String {
    let refs: Vec<&str> = bits.iter().map(String::as_str).collect();
    join_union(&refs)
}

/// Renders one stub file (`crates/baylee-cards/src/cards/<slug>.rs`).
///
/// # Errors
/// [`CodegenError::Mana`] when a mana cost fails validation.
pub fn render_stub(
    card: &ScryfallCard,
    index: u32,
    cats: &SubtypeCatalogs,
    forge: Option<&crate::forgegen::ForgeLookup>,
) -> Result<(StubInfo, String), CodegenError> {
    // Multi-face cards slug by their front face ("Brightclimb Pathway // …"
    // → "brightclimb_pathway").
    let slug = slug(card.name.split(" // ").next().unwrap_or(&card.name));
    let faces = faces_of(card);
    let oracle_id = card.oracle_id.clone().unwrap_or_default();
    // A card is written out finished only when a reader understood the whole
    // of it; one clause left over and it stays an ordinary stub. Its own
    // printed text is tried first, because a land's intrinsic mana comes from
    // its type line (CR 305.6) and no forge script restates it.
    let land = crate::landgen::recognize(card, cats).or_else(|| {
        let script = forge?.script(&card.name)?;
        crate::forgegen::transcode(&script, cats)
    });

    let mut out = String::with_capacity(4096);
    // Human-verifiable header (docs/card-dsl.md).
    out.push_str(&format!("//! {} \u{2014} ", card.name));
    // The front face's cost, not the card's: a modal double-faced card
    // carries no top-level `mana_cost`, so reading it there printed
    // "(no cost)" over a `FaceDef` that plainly had one — which `xtask
    // validate` compares, and which stayed invisible while the pool held no
    // two-faced cards.
    let front_cost = faces.first().map_or("", |f| f.mana_cost.as_str());
    out.push_str(if front_cost.is_empty() {
        "(no cost)"
    } else {
        front_cost
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
    if let Some(body) = &land {
        let mut notes = body.notes.clone();
        notes.dedup();
        out.push_str(&format!(
            "// IMPLEMENTED — generated by `xtask codegen`: {}.\n\n",
            if notes.is_empty() {
                "printed abilities".to_string()
            } else {
                notes.join(", ")
            }
        ));
    } else {
        out.push_str("// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.\n\n");
    }
    // One import, not six. The old list was the same for every card whether
    // or not the card used it, which is why every stub also had to carry
    // `#![allow(unused_imports)]` — an allow that then hid two dozen genuinely
    // dead imports in hand-finished files for as long as it existed.
    out.push_str("use baylee_cards_dsl::prelude::*;\n");

    let mut face_defs = String::new();
    for (i, f) in faces.iter().enumerate() {
        // Enter modifiers are printed on the front face; a recognised land is
        // single-faced by construction (`landgen::recognize` refuses the rest).
        let enters = match (&land, i) {
            (Some(body), 0) => body.enter_modifiers.as_slice(),
            _ => &[],
        };
        face_defs.push_str(&render_face(&card.name, f, cats, enters)?);
    }
    let literal = render_card_literal(card, index, &oracle_id, &faces, &face_defs, land.as_ref());
    let statics = land.as_ref().map_or("", |b| b.statics.as_str());
    // Only when something names one: an unused import is a warning now that
    // the stub no longer carries a blanket `allow`. The card literal counts
    // too — `Modifier::AddSubtype` puts a subtype inside an ability, where
    // neither the faces nor the hoisted filters would see it.
    if face_defs.contains("subtypes::")
        || statics.contains("subtypes::")
        || literal.contains("subtypes::")
    {
        out.push_str("use baylee_core::generated::subtypes;\n");
    }
    out.push('\n');
    out.push_str(statics);
    out.push_str(&literal);
    // No test module. A card file is data, and a test that reads the literal
    // it sits under proves nothing — every one of the 194 hand-written cards
    // left the module empty, and the generated cards that filled it produced
    // assertions like `assert_eq!(*filter, BASIC_LAND_FILTER)`, which compares
    // the filter with itself. Behaviour is tested in baylee-engine, and a rule
    // that must hold for *every* card belongs in the cross-cutting tests at
    // the foot of baylee-cards/src/lib.rs.
    if land.is_none() {
        out.push_str("// TODO(card): implement abilities, see docs/card-dsl.md.\n");
    }

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
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         //! One module per registered card.\n\
         //!\n\
         //! A card file opens with the printed name, cost and type line, which\n\
         //! is a proper noun from Scryfall rather than prose: `doc_markdown`\n\
         //! reads the intercaps in \"SeeD Academy\" or \"Ashiok, Dream Render\"\n\
         //! as unbackticked code. Lint levels reach the child modules from\n\
         //! here, so this is the one place that has to say so.\n\
         #![allow(clippy::doc_markdown)]\n\n",
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
pub fn render_registry(stubs: &[StubInfo], slots: usize) -> String {
    let mut by_oracle: Vec<&StubInfo> = stubs.iter().collect();
    by_oracle.sort_by(|a, b| a.oracle_id.cmp(&b.oracle_id));
    // Indices come from the ledger and are permanent, so the table is as long
    // as the highest one ever assigned and a card that left the pool leaves a
    // hole rather than shifting its neighbours.
    let mut slot: Vec<Option<&StubInfo>> = vec![None; slots];
    for s in stubs {
        slot[s.index as usize] = Some(s);
    }

    let mut hash = FNV_OFFSET;
    for s in &by_oracle {
        hash = fnv1a(hash, s.oracle_id.as_bytes());
        hash = fnv1a(hash, s.slug.as_bytes());
        // The index is part of the pool's identity now that it is permanent:
        // a client holding a cache keyed on this hash must drop it if a card
        // it knows has moved.
        hash = fnv1a(hash, &s.index.to_le_bytes());
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
    out.push_str(
        "];\n\n/// Registered cards by `CardIndex`. `None` is a retired index:\n/// the card left the pool, and the slot is never handed on.\npub static BY_INDEX: &[Option<&CardDef>] = &[\n",
    );
    for s in &slot {
        match s {
            Some(s) => out.push_str(&format!("    Some(&crate::cards::{}::CARD),\n", s.slug)),
            None => out.push_str("    None,\n"),
        }
    }
    out.push_str(&format!(
        "];\n\n/// FNV-1a hash over the registry content.\npub const POOL_HASH: u64 = {hash:#x};\n\n"
    ));
    out.push_str(
        "pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {\n    ALL.binary_search_by(|(id, _)| (*id).cmp(oracle_id))\n        .ok()\n        .map(|i| ALL[i].1)\n}\n\npub fn by_index(index: CardIndex) -> Option<&'static CardDef> {\n    BY_INDEX.get(index.get() as usize).copied().flatten()\n}\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A retired index leaves a hole, and the hole must reach the generated
    /// table as a `None` rather than closing up — closing it would slide every
    /// later card onto its neighbour's number, which is the whole bug this
    /// design exists to prevent. Nothing in the repo has a hole yet, so this
    /// is the only place the path is exercised.
    #[test]
    fn a_retired_index_becomes_an_empty_slot_and_moves_nothing() {
        let stubs = vec![
            StubInfo {
                slug: "first".into(),
                oracle_id: "oracle-a".into(),
                index: 0,
            },
            StubInfo {
                slug: "third".into(),
                oracle_id: "oracle-c".into(),
                index: 2,
            },
        ];
        let out = render_registry(&stubs, 3);
        let table = out
            .split("pub static BY_INDEX")
            .nth(1)
            .expect("index table is rendered");
        let slots: Vec<&str> = table
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                l.starts_with("Some(")
                    .then_some(l)
                    .or(l.eq("None,").then_some(l))
            })
            .collect();
        assert_eq!(
            slots,
            vec![
                "Some(&crate::cards::first::CARD),",
                "None,",
                "Some(&crate::cards::third::CARD),",
            ],
            "index 1 is retired: the slot stays empty and `third` keeps index 2"
        );
    }

    /// The pool hash covers the indices, not just the names: a client cache
    /// keyed on it has to drop when a card it knows moves.
    #[test]
    fn the_pool_hash_notices_a_moved_card() {
        let at = |index| {
            vec![StubInfo {
                slug: "only".into(),
                oracle_id: "oracle-a".into(),
                index,
            }]
        };
        let hash_of = |s: &str| {
            s.split("POOL_HASH: u64 = ")
                .nth(1)
                .and_then(|r| r.split(';').next())
                .expect("hash is rendered")
                .to_string()
        };
        assert_ne!(
            hash_of(&render_registry(&at(0), 1)),
            hash_of(&render_registry(&at(7), 8))
        );
    }

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

    fn bare_card(name: &str, type_line: &str) -> ScryfallCard {
        ScryfallCard {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            oracle_id: Some("11111111-1111-1111-1111-111111111111".to_string()),
            name: name.to_string(),
            mana_cost: None,
            type_line: Some(type_line.to_string()),
            oracle_text: Some(String::new()),
            colors: None,
            color_identity: None,
            set: None,
            set_name: None,
            collector_number: None,
            rarity: None,
            layout: None,
            power: None,
            toughness: None,
            loyalty: None,
            card_faces: None,
        }
    }

    /// A stub states only what is printed and inherits the rest, so a new
    /// `FaceDef` field never has to be back-filled into 200 card files.
    #[test]
    fn a_stub_omits_every_field_that_matches_the_default() {
        let cats = SubtypeCatalogs::default();
        let (_, text) = render_stub(&bare_card("Nothing", "Land"), 7, &cats, None).unwrap();
        // The tail is the macro's job now; what matters is unchanged — a
        // stub states what is printed and nothing else.
        assert!(text.contains("card! {"));
        assert!(text.contains("face! {"));
        assert!(!text.contains("#![allow("), "the blanket allow is gone");
        for absent in [
            "mana_cost:",
            "supertypes:",
            "subtypes:",
            "power:",
            "toughness:",
            "loyalty:",
            "color_identity:",
            "keywords:",
            "commander:",
            "partner:",
            "alternative_costs:",
            "castable_from_hand:",
        ] {
            assert!(
                !text.contains(absent),
                "stub restated the default for {absent}\n{text}"
            );
        }
        assert!(text.contains("    types: TypeSet::LAND,\n"));
        assert!(text.contains("    index: 7,\n"));
    }

    /// `coverage` is never emitted: `CardDef::DEFAULT` is
    /// `Unimplemented`, so an abandoned stub cannot pass for playable.
    #[test]
    fn a_stub_is_unimplemented_by_omission() {
        let cats = SubtypeCatalogs::default();
        let (_, text) = render_stub(&bare_card("Nothing", "Land"), 0, &cats, None).unwrap();
        assert!(!text.contains("coverage:"));
    }

    /// Printed values still appear — the tail only supplies what the card
    /// does not say.
    #[test]
    fn a_stub_keeps_the_fields_the_card_actually_prints() {
        let cats = SubtypeCatalogs::default();
        let mut card = bare_card("Something", "Legendary Creature");
        card.mana_cost = Some("{1}{W}".to_string());
        card.power = Some("2".to_string());
        card.toughness = Some("3".to_string());
        card.color_identity = Some(vec!["W".to_string()]);
        let (_, text) = render_stub(&card, 1, &cats, None).unwrap();
        assert!(text.contains("mana_cost: baylee_core::mana!(\"{1}{W}\"),"));
        assert!(text.contains("power: Some(2),"));
        assert!(text.contains("toughness: Some(3),"));
        assert!(text.contains("supertypes: SupertypeSet::LEGENDARY,"));
        assert!(text.contains("color_identity: ColorSet::from_slice(&[Color::White]),"));
        assert!(text.contains("commander: CommanderRule::Legendary,"));
    }
}
