//! What each card face is, costs and does, read from the same definitions the
//! structure catalog encodes: targets for training the rule encoder on cards
//! (`tools/trainer/card_stage4.py`), keyed by the catalog's `c/<card>/<face>`
//! roots.
//!
//! Exact, as the engine plays the card: mana value and cost symbols, colours,
//! types, supertypes, subtypes, printed power, toughness, loyalty and defense,
//! keywords, the kinds of its abilities, and which DSL effect variants it uses.
//! A characteristic-defining power (`*`) has no printed number and stays
//! absent. Effect *families* group those variants by name ([`families`]): the
//! membership of a variant is exact, the boundaries of a family are ours.
//! Nothing here reads a game, a deck or a recorded choice.

use super::{Forest, TreeEncode, catalog::face_abilities};
use baylee_cards_dsl::CardDef;
use baylee_core::mana::ManaSymbol;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The label format; the trainer refuses any other.
pub const LABELS_VERSION: u32 = 1;

/// A subtype is a target of its own on at least this many faces; rarer ones
/// are not predicted.
pub const MIN_SUBTYPE_FACES: usize = 20;

/// Effect families, in output order.
pub const FAMILIES: [&str; 24] = [
    "damage",
    "prevent_redirect",
    "draw",
    "discard",
    "destroy",
    "exile",
    "counter_spell",
    "token",
    "life_gain",
    "life_loss",
    "mill",
    "bounce",
    "graveyard_return",
    "search_library",
    "look_reveal",
    "mana",
    "pump",
    "counters",
    "tap_untap",
    "sacrifice",
    "control",
    "copy",
    "turns_and_game",
    "delayed",
];

/// The families of one DSL effect variant (`Effect::<variant>`), by name, in
/// [`FAMILIES`] order.
#[must_use]
pub fn families(variant: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    cards_and_life(variant, &mut |hit, family| {
        if hit {
            out.push(family);
        }
    });
    zones_and_board(variant, &mut |hit, family| {
        if hit {
            out.push(family);
        }
    });
    out
}

/// Damage, cards, counterspells, tokens, life and milling: the first eleven families.
fn cards_and_life(v: &str, add: &mut dyn FnMut(bool, &'static str)) {
    add(
        v.contains("DealDamage")
            || v.contains("DealsDamage")
            || v.starts_with("DealHalf")
            || v == "DamageEqualToPower"
            || v == "Fight",
        "damage",
    );
    add(
        v.contains("Prevent") || v.starts_with("Redirect") || v.starts_with("Reflect"),
        "prevent_redirect",
    );
    add(
        (v.starts_with("Draw") && v != "DrawGame")
            || v.ends_with("ThenDraw")
            || v.ends_with("OrDraw")
            || v == "DrawIfGuessWrong"
            || v == "Connive",
        "draw",
    );
    add(v.contains("Discard") || v == "Connive", "discard");
    add(v.starts_with("Destroy"), "destroy");
    add(
        v.contains("Exile") && !v.starts_with("ReturnLinked"),
        "exile",
    );
    add(v.starts_with("Counter"), "counter_spell");
    add(
        v.contains("CreateToken") || v == "Populate" || v == "Amass",
        "token",
    );
    add(
        v.starts_with("GainLife") || v == "RaiseLifeTotalTo",
        "life_gain",
    );
    add(v.contains("LoseLife") || v == "LoseHalfLife", "life_loss");
    add(v.contains("Mill"), "mill");
}

/// Zones, mana, the board and the turn: the remaining families.
fn zones_and_board(v: &str, add: &mut dyn FnMut(bool, &'static str)) {
    add(v.starts_with("Return") && v.ends_with("ToHand"), "bounce");
    add(
        (v.contains("Graveyard")
            && (v.contains("ToHand") || v.contains("ToBattlefield") || v.contains("ToTop")))
            || v.starts_with("Reanimate")
            || v.starts_with("ReturnFromGraveyard")
            || v == "ReturnSourceFromGraveyard",
        "graveyard_return",
    );
    add(v.contains("Search"), "search_library");
    add(
        v.starts_with("Scry")
            || v == "Surveil"
            || v.starts_with("LookAt")
            || v.starts_with("Reveal")
            || v.contains("ReorderTop"),
        "look_reveal",
    );
    add(
        v.starts_with("AddMana")
            || v.starts_with("ManaFor")
            || v == "DelayedManaAtNextMain"
            || v == "ActivateLandsAndTakeMana",
        "mana",
    );
    add(
        v.starts_with("Pump") || v.starts_with("SetPT") || v.starts_with("SetSourcePT"),
        "pump",
    );
    add(
        (v.contains("Counter") && !v.starts_with("Counter")) || v == "Proliferate",
        "counters",
    );
    add(
        v.starts_with("Tap") || v.starts_with("Untap") || v.starts_with("ToggleTap"),
        "tap_untap",
    );
    add(v.contains("Sacrifice"), "sacrifice");
    add(
        v.starts_with("GainControl")
            || v == "ChangeController"
            || v.starts_with("ExchangeControl")
            || v == "AttachedGainControlBy"
            || v == "ControlRotation",
        "control",
    );
    add(v.contains("Copy"), "copy");
    add(
        matches!(v, "TakeExtraTurn" | "WinGame" | "LoseGame" | "DrawGame") || v.starts_with("Skip"),
        "turns_and_game",
    );
    add(
        v.starts_with("AtNext")
            || v.starts_with("AtEnd")
            || v.starts_with("When")
            || v.starts_with("Whenever"),
        "delayed",
    );
}

/// Symbols of a mana cost, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CostSymbols {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
    pub generic: u32,
    pub variable: u32,
    pub hybrid: u32,
    pub phyrexian: u32,
    pub snow: u32,
}

impl CostSymbols {
    fn of(cost: &baylee_core::mana::ManaCost) -> Self {
        let mut out = Self::default();
        for symbol in cost.symbols() {
            match symbol {
                ManaSymbol::White => out.white += 1,
                ManaSymbol::Blue => out.blue += 1,
                ManaSymbol::Black => out.black += 1,
                ManaSymbol::Red => out.red += 1,
                ManaSymbol::Green => out.green += 1,
                ManaSymbol::Colorless => out.colorless += 1,
                ManaSymbol::Generic(n) => out.generic += n,
                ManaSymbol::Variable(_) => out.variable += 1,
                ManaSymbol::Hybrid(_) | ManaSymbol::TwoOrColor(_) => out.hybrid += 1,
                ManaSymbol::Phyrexian(_) | ManaSymbol::HybridPhyrexian(_) => out.phyrexian += 1,
                ManaSymbol::Snow => out.snow += 1,
                ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {}
            }
        }
        out
    }
}

/// One face's targets.
#[derive(Clone, Debug, Serialize)]
pub struct FaceLabels {
    /// The catalog root this face is, `c/<card>/<face>`.
    pub root: String,
    pub card: u32,
    pub face: u32,
    /// Every printed sentence claimed (`CardDef::is_fully_read`); a stub's
    /// tree is what the engine plays, not the printed card.
    pub fully_read: bool,
    pub castable_from_hand: bool,
    pub mana_value: u32,
    pub cost: CostSymbols,
    /// `ColorSet` bits: the cost's colours and the colour indicator.
    pub colors: u8,
    /// `TypeSet` bits.
    pub types: u16,
    /// `SupertypeSet` bits.
    pub supertypes: u8,
    /// Indices into [`Labels::subtypes`].
    pub subtypes: Vec<u16>,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub loyalty: Option<u16>,
    pub defense: Option<u16>,
    /// Bit positions of the face's `KeywordSet`.
    pub keywords: Vec<u8>,
    /// Indices into [`Labels::ability_kinds`].
    pub ability_kinds: Vec<u16>,
    /// Indices into [`FAMILIES`].
    pub families: Vec<u16>,
    /// Indices into [`Labels::effects`]: the effect variants it uses.
    pub effects: Vec<u16>,
}

/// The targets of a whole catalog.
#[derive(Debug, Serialize)]
pub struct Labels {
    pub version: u32,
    /// `digest` of the forest these labels belong to.
    pub catalog_sha256: String,
    pub families: Vec<&'static str>,
    /// `AbilityDef::<kind>` names seen in the catalog.
    pub ability_kinds: Vec<String>,
    /// `Effect::<variant>` names seen in the catalog.
    pub effects: Vec<String>,
    /// Subtype ids with at least [`MIN_SUBTYPE_FACES`] faces.
    pub subtypes: Vec<u16>,
    pub faces: Vec<FaceLabels>,
}

/// Tag names under `root`, each once.
fn tags_under(forest: &Forest, root: u32) -> BTreeSet<&str> {
    let mut seen = BTreeSet::new();
    let mut tags = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        let node = &forest.nodes[node as usize];
        tags.insert(forest.tags[node.tag as usize].as_str());
        stack.extend(&node.children);
    }
    tags
}

/// A DSL variant name from a tag (`Effect::DrawCards` → `DrawCards`); field
/// tags (`Effect::DrawCards.count`) are not variants.
fn variant<'a>(tag: &'a str, family: &str) -> Option<&'a str> {
    tag.strip_prefix(family)?
        .strip_prefix("::")
        .filter(|v| !v.contains('.'))
}

/// One face before its names become indices.
struct RawFace {
    face: FaceLabels,
    subtypes: Vec<u16>,
    kinds: BTreeSet<String>,
    effects: BTreeSet<String>,
}

/// Reads one face of `def` from its definition and its abilities' nodes.
fn read_face(forest: &mut Forest, def: &CardDef, face: usize) -> anyhow::Result<RawFace> {
    let cid = def.index.get();
    let value = &def.faces[face];
    let key = format!("c/{cid}/{face}");
    anyhow::ensure!(forest.roots.contains_key(&key), "no catalog root {key}");
    // Interning an existing tree returns its node: the abilities are read from
    // the catalog's own nodes, and a new node means another catalog.
    let nodes = forest.nodes.len();
    let abilities = face_abilities(def, face).tree(forest);
    anyhow::ensure!(
        forest.nodes.len() == nodes,
        "{key}: abilities are not in this catalog"
    );
    let tags = tags_under(forest, abilities);
    let names = |family| {
        tags.iter()
            .filter_map(|t| variant(t, family))
            .map(str::to_owned)
            .collect::<BTreeSet<String>>()
    };
    let keywords = def.keywords_for_face(face).bits();
    Ok(RawFace {
        face: FaceLabels {
            root: key,
            card: cid,
            face: face as u32,
            fully_read: def.is_fully_read(),
            castable_from_hand: value.castable_from_hand,
            mana_value: value.mana_cost.cmc(),
            cost: CostSymbols::of(&value.mana_cost),
            colors: value.mana_cost.colors().union(value.color_indicator).bits(),
            types: value.types.bits(),
            supertypes: value.supertypes.bits(),
            subtypes: vec![],
            power: value.power,
            toughness: value.toughness,
            loyalty: value.loyalty,
            defense: value.defense,
            keywords: (0..128u8).filter(|b| keywords >> b & 1 == 1).collect(),
            ability_kinds: vec![],
            families: vec![],
            effects: vec![],
        },
        subtypes: value.subtypes.iter().map(|s| s.get()).collect(),
        kinds: names("AbilityDef"),
        effects: names("Effect"),
    })
}

/// Labels every face of `cards` in `forest`, which must be the catalog of
/// exactly those cards (`catalog`).
///
/// # Errors
/// If a card has no definition, a face has no root in `forest`, or reading a
/// face's abilities would add a node: then `forest` is not this build's
/// catalog of `cards`.
///
/// # Panics
/// Never: every name is looked up in a list collected from the same faces.
pub fn labels(forest: &mut Forest, cards: &[u32]) -> anyhow::Result<Labels> {
    let catalog_sha256 = super::digest(forest)?;
    let mut raw = Vec::new();
    for &cid in cards {
        let def: &CardDef = baylee_cards::by_index(baylee_core::ids::CardIndex::new(cid))
            .ok_or_else(|| anyhow::anyhow!("missing card definition {cid}"))?;
        for face in 0..def.faces.len() {
            raw.push(read_face(forest, def, face)?);
        }
    }
    let mut subtype_faces: BTreeMap<u16, usize> = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    let mut effects = BTreeSet::new();
    for face in &raw {
        for &s in &face.subtypes {
            *subtype_faces.entry(s).or_default() += 1;
        }
        kinds.extend(face.kinds.iter().cloned());
        effects.extend(face.effects.iter().cloned());
    }
    let subtype_vocab: Vec<u16> = subtype_faces
        .iter()
        .filter(|&(_, &n)| n >= MIN_SUBTYPE_FACES)
        .map(|(&s, _)| s)
        .collect();
    let kinds: Vec<String> = kinds.into_iter().collect();
    let effects: Vec<String> = effects.into_iter().collect();
    let index = |list: &[String], name: &str| {
        list.iter()
            .position(|x| x == name)
            .expect("collected above") as u16
    };
    let faces = raw
        .into_iter()
        .map(|read| {
            let RawFace {
                mut face,
                subtypes,
                kinds: face_kinds,
                effects: face_effects,
            } = read;
            face.subtypes = subtypes
                .iter()
                .filter_map(|s| subtype_vocab.iter().position(|v| v == s))
                .map(|i| i as u16)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            face.ability_kinds = face_kinds.iter().map(|k| index(&kinds, k)).collect();
            face.effects = face_effects.iter().map(|e| index(&effects, e)).collect();
            let families: BTreeSet<u16> = face_effects
                .iter()
                .flat_map(|e| families(e))
                .map(|f| {
                    FAMILIES
                        .iter()
                        .position(|x| *x == f)
                        .expect("a listed family") as u16
                })
                .collect();
            face.families = families.into_iter().collect();
            face
        })
        .collect();
    Ok(Labels {
        version: LABELS_VERSION,
        catalog_sha256,
        families: FAMILIES.to_vec(),
        ability_kinds: kinds,
        effects,
        subtypes: subtype_vocab,
        faces,
    })
}
