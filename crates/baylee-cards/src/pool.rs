//! The playable pool: what a deck builder is allowed to offer.
//!
//! The catalog holds every printing of every card ever made; the engine
//! implements a few hundred of them. A builder that searched the catalog
//! would offer a player a hundred thousand cards of which almost none can be
//! put in a deck, and the ones that cannot are indistinguishable until the
//! save is refused. So the pool is the **compiled registry**, and `Coverage`
//! travels with every row — a card that is only partly implemented says so,
//! with the reason its author wrote.
//!
//! It lives here rather than in the gateway because it is a fact about the
//! *registry*, and two things need it: `GET /pool` serves it to a client with
//! an account, and offline play reads it straight out of this process. Two
//! copies of this mapping would be two answers to "what may I put in a deck",
//! and the one a player never sees is the one that would rot.
//!
//! Nothing here needs a database. Names, costs, types, colors and stats all
//! come from `CardDef`, so a gateway without `DATABASE_URL` still has a
//! working deck builder and so does a client with no gateway at all — what
//! neither can do without a catalog is show rules text or translate a name.

use baylee_cards_dsl::{CardDef, Coverage};
use baylee_core::types::{SupertypeSet, TypeSet};
use serde::Serialize;
use std::sync::OnceLock;

/// One card, as a deck builder needs to see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PoolCard {
    /// Registry index — the rules identity, and what a saved deck names.
    pub index: u32,
    /// Name in the served language, which is the English name until a catalog
    /// says otherwise.
    pub name: String,
    /// English name, always. A deck is stored by name, and the name it is
    /// stored under is this one whatever language the player reads in.
    pub english_name: String,
    /// Mana cost in the notation `docs/mana-notation.md` describes; empty for
    /// a land.
    pub mana_cost: String,
    /// Mana value (CR 202.3).
    pub cmc: u32,
    /// Colors of the card itself, as `WUBRG` letters.
    pub colors: String,
    /// Color identity (CR 903.4) — what a commander deck is bounded by.
    pub identity: String,
    /// Full printed type line, supertypes through subtypes, in the served
    /// language.
    pub type_line: String,
    /// The card types as English words (`Creature`, `Land`, …).
    ///
    /// Separate from `type_line` because a builder groups and filters on
    /// these, and a localized type line is not something to parse: the words
    /// move, and in German they are not even the same words.
    pub kinds: Vec<&'static str>,
    /// Power/toughness, or a planeswalker's loyalty. `None` for everything
    /// that prints neither.
    pub stats: Option<String>,
    /// Rules text, when a catalog is configured. Empty otherwise — the engine
    /// never carries card text.
    pub oracle_text: String,
    /// `implemented`, `partial` or `unimplemented`.
    pub coverage: &'static str,
    /// Why a `partial` card is only partly there, in its author's words.
    pub note: Option<&'static str>,
    /// Whether the card may lead a commander deck.
    pub commander: bool,
    /// Basic lands are the one card a deck may hold any number of.
    pub basic_land: bool,
    /// Whether the card is printed on both sides.
    ///
    /// The client cannot work this out for itself: a back-face image URL can
    /// be *built* for any printing, and Scryfall answers 404 for the ones
    /// that have no back. Sending the bit is one byte and saves the builder
    /// from offering to turn a card that has nothing on the other side.
    pub two_faced: bool,
    /// The printing codegen referenced, for art and for the catalog.
    pub scryfall_id: &'static str,
    /// Rules identity, shared by every printing and every language.
    ///
    /// This is what a printing lookup is keyed on: a player picking the art
    /// they own is asking about the *card*, and the answer crosses every set
    /// it was ever printed in.
    pub oracle_id: &'static str,
    /// Every other name this card is printed under, across languages.
    ///
    /// The builder shows one row per card and lets a player find it by typing
    /// any of its names — a German player types "Blitzschlag" and gets the
    /// row that a deck stores as "Lightning Bolt". Empty without a catalog,
    /// and omitted from the wire when empty: for two hundred cards in a dozen
    /// languages this is the largest field in the answer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alt_names: Vec<String>,
}

/// The registry as builder rows, built once.
///
/// Nothing here depends on a request, a language or a database, and walking a
/// few hundred `CardDef`s to format type lines is work that would otherwise
/// repeat on every visit to the deck builder.
#[must_use]
pub fn rows() -> &'static [PoolCard] {
    static ROWS: OnceLock<Vec<PoolCard>> = OnceLock::new();
    ROWS.get_or_init(|| crate::all().map(row).collect())
}

/// One card's row, from the registry alone.
#[must_use]
pub fn row(def: &'static CardDef) -> PoolCard {
    let face = def.faces.first();
    let cost = face.map(|f| f.mana_cost).unwrap_or_default();
    let name = def.name().to_string();
    let (coverage, note) = match def.coverage {
        Coverage::Implemented => ("implemented", None),
        Coverage::Partial(why) => ("partial", Some(why)),
        Coverage::Unimplemented => ("unimplemented", None),
    };
    PoolCard {
        index: def.index.get(),
        english_name: name.clone(),
        name,
        mana_cost: cost.to_string(),
        cmc: cost.cmc(),
        colors: letters(cost.colors()),
        identity: letters(def.color_identity),
        type_line: face.map(type_line).unwrap_or_default(),
        kinds: face.map(|f| f.types.words().collect()).unwrap_or_default(),
        stats: face.and_then(stats),
        oracle_text: String::new(),
        coverage,
        note,
        commander: !matches!(def.commander, baylee_cards_dsl::CommanderRule::NotEligible),
        basic_land: face.is_some_and(|f| {
            f.supertypes.contains(SupertypeSet::BASIC) && f.types.contains(TypeSet::LAND)
        }),
        two_faced: def.faces.len() > 1,
        scryfall_id: def.scryfall_id,
        oracle_id: def.oracle_id,
        alt_names: Vec::new(),
    }
}

/// A color set as `WUBRG` letters, in that order.
#[must_use]
pub fn letters(colors: baylee_core::color::ColorSet) -> String {
    colors
        .iter()
        .map(baylee_core::color::Color::symbol)
        .collect()
}

/// The printed type line: supertypes, types, then subtypes after an em dash.
#[must_use]
pub fn type_line(face: &'static baylee_cards_dsl::FaceDef) -> String {
    let mut line = String::new();
    for word in face.supertypes.words().chain(face.types.words()) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    let subtypes: Vec<&str> = face
        .subtypes
        .iter()
        .filter_map(|id| baylee_core::generated::subtypes::name(*id))
        .collect();
    if !subtypes.is_empty() {
        line.push_str(" \u{2014} ");
        line.push_str(&subtypes.join(" "));
    }
    line
}

/// Power/toughness, or loyalty, or nothing.
#[must_use]
pub fn stats(face: &'static baylee_cards_dsl::FaceDef) -> Option<String> {
    match (face.power, face.toughness, face.loyalty) {
        (Some(p), Some(t), _) => Some(format!("{p}/{t}")),
        (_, _, Some(l)) => Some(l.to_string()),
        _ => None,
    }
}
