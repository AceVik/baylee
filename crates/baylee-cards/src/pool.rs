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
    /// Every other name this card is printed under — other languages, and
    /// the whole `A // B` spelling of a card with two faces.
    ///
    /// The builder shows one row per card and lets a player find it by typing
    /// any of its names — a German player types "Blitzschlag" and gets the
    /// row that a deck stores as "Lightning Bolt". The search is a substring
    /// match, so the whole spelling earns the **back** face as well: a player
    /// who knows Agadeem's Awakening as the land types "Agadeem, the
    /// Undercrypt" and finds it, without that name being carried separately.
    ///
    /// The translations need a catalog and the whole spelling does not —
    /// it comes off the registry, so it is here on a gateway that has never
    /// ingested and in the client's offline harness, for the same reason
    /// `POST /decks` takes that spelling either way. Omitted from the wire
    /// when empty: for two hundred cards in a dozen languages this is the
    /// largest field in the answer.
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
        // Scryfall's whole spelling, where the pool's own is one face of it.
        // Seeded here rather than joined on later, so it is there on a
        // gateway with no catalog ingest and in the client's offline
        // harness — the same reason `POST /decks` accepts that spelling
        // whether or not a catalog exists.
        alt_names: crate::decks::whole_name(def.index)
            .map(ToString::to_string)
            .into_iter()
            .collect(),
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

#[cfg(test)]
mod whole_name_tests {
    use crate::decks::{by_name, whole_name};

    /// A two-faced card's row carries the spelling the pool does not call it
    /// by, and a single-faced card's carries nothing — the field is omitted
    /// from the wire when empty, so a row per card would cost every card.
    #[test]
    fn only_a_card_with_two_faces_carries_a_second_spelling() {
        let mut with = 0;
        for def in crate::all() {
            let row = super::row(def);
            match whole_name(def.index) {
                Some(whole) => {
                    with += 1;
                    assert_eq!(
                        row.alt_names,
                        vec![whole.to_string()],
                        "{} should carry its whole spelling",
                        def.name()
                    );
                    assert!(
                        whole.contains(" // ") && whole.starts_with(def.name()),
                        "{whole} is not {} plus a second face",
                        def.name()
                    );
                }
                None => assert!(
                    row.alt_names.is_empty(),
                    "{} has one spelling and should carry none",
                    def.name()
                ),
            }
        }
        assert!(
            with > 100,
            "only {with} rows carry a second spelling — the seed is not running"
        );
    }

    /// The claim the field exists for, stated the way the builder asks it.
    ///
    /// The search is a substring match over every name a row carries, so
    /// carrying `A // B` answers three different things a player might type:
    /// the front face, the back face, and the whole spelling a deck site
    /// exports. The back face is the one that had no answer at all before —
    /// somebody who knows Agadeem's Awakening as the land it turns into
    /// could not find it under that name.
    ///
    /// **A search may do this where a deck row may not**, and the asymmetry
    /// is deliberate: a deck row has to *resolve* to exactly one card, so an
    /// ambiguous spelling is fatal there and `Demonic Tutor` must stay
    /// Demonic Tutor. A search *offers* candidates, and showing both cards
    /// that print a name is what a search is for.
    #[test]
    fn a_row_can_be_found_by_the_front_face_the_back_face_or_the_whole_name() {
        let index = by_name("Agadeem's Awakening").expect("in the pool");
        let row = super::row(crate::by_index(index).expect("compiled"));
        let haystack: Vec<String> = std::iter::once(row.name.clone())
            .chain(row.alt_names.iter().cloned())
            .map(|n| n.to_lowercase())
            .collect();
        for needle in [
            "agadeem's awakening",
            "agadeem, the undercrypt",
            "agadeem's awakening // agadeem, the undercrypt",
        ] {
            assert!(
                haystack.iter().any(|n| n.contains(needle)),
                "{needle:?} finds nothing in {haystack:?}"
            );
        }
    }
}
