//! The playable pool: what a deck builder is allowed to offer.
//!
//! The catalog holds every printing of every card ever made; the engine
//! implements a few hundred of them. A builder that searched the catalog would
//! offer a player a hundred thousand cards of which almost none can be put in
//! a deck, and the ones that cannot are indistinguishable until `POST /decks`
//! refuses them. So the pool is the **compiled registry**, and `Coverage`
//! travels with every row — a card that is only partly implemented says so,
//! with the reason its author wrote.
//!
//! Two consequences worth knowing:
//!
//! - The pool needs no database. Names, costs, types, colors and stats all
//!   come from `CardDef`, so a gateway without `DATABASE_URL` still has a
//!   working deck builder — it just cannot show rules text or translate a
//!   name. With a catalog configured, both are filled in.
//! - It is small enough to send whole (a few hundred rows), which is what lets
//!   a client filter at keystroke latency instead of asking the gateway per
//!   letter. When the registry outgrows one response this is where paging
//!   goes; the shape of the answer already carries `total`.

use crate::{ErrorBody, Shared, err};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use baylee_cards_dsl::{CardDef, Coverage};
use baylee_core::types::{SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};
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
    /// The printing codegen referenced, for art and for the catalog.
    pub scryfall_id: &'static str,
}

/// The answer to `GET /pool`.
#[derive(Clone, Debug, Serialize)]
pub struct PoolBody {
    /// How many cards the registry holds, which is how many rows follow.
    pub total: usize,
    /// The compiled card pool's hash, so a client can cache the rows and ask
    /// again only when the pool itself changed.
    pub pool_hash: String,
    /// The language the rows were served in.
    pub lang: String,
    /// Whether rules text is available at all. A client that knows the answer
    /// is "no" can say so once, rather than drawing every card as textless.
    pub has_text: bool,
    /// The cards.
    pub cards: Vec<PoolCard>,
}

/// Query for `GET /pool`.
#[derive(Deserialize)]
pub struct PoolQuery {
    /// Preferred language for names and rules text.
    lang: Option<String>,
}

/// The registry as builder rows, built once.
///
/// Nothing here depends on a request, a language or a database, and walking a
/// few hundred `CardDef`s to format type lines is work that would otherwise
/// repeat on every visit to the deck builder.
fn registry_rows() -> &'static [PoolCard] {
    static ROWS: OnceLock<Vec<PoolCard>> = OnceLock::new();
    ROWS.get_or_init(|| baylee_cards::all().map(row).collect())
}

/// One card's row, from the registry alone.
fn row(def: &'static CardDef) -> PoolCard {
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
        scryfall_id: def.scryfall_id,
    }
}

/// A color set as `WUBRG` letters, in that order.
fn letters(colors: baylee_core::color::ColorSet) -> String {
    colors
        .iter()
        .map(baylee_core::color::Color::symbol)
        .collect()
}

/// The printed type line: supertypes, types, then subtypes after an em dash.
fn type_line(face: &'static baylee_cards_dsl::FaceDef) -> String {
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
        line.push_str(" — ");
        line.push_str(&subtypes.join(" "));
    }
    line
}

/// Power/toughness, or loyalty, or nothing.
fn stats(face: &'static baylee_cards_dsl::FaceDef) -> Option<String> {
    match (face.power, face.toughness, face.loyalty) {
        (Some(p), Some(t), _) => Some(format!("{p}/{t}")),
        (_, _, Some(l)) => Some(l.to_string()),
        _ => None,
    }
}

/// `GET /pool` — every card a deck may be built from.
///
/// Deliberately unauthenticated, for the same reason `/catalog/text` is: this
/// is what the game can play, which is public reference data, and a player
/// looking at what the platform supports has not signed up yet.
pub async fn pool(
    State(state): State<Shared>,
    Query(params): Query<PoolQuery>,
) -> Result<Json<PoolBody>, (StatusCode, Json<ErrorBody>)> {
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();
    let mut cards = registry_rows().to_vec();
    let mut has_text = false;
    if let Some(catalog) = state.catalog.as_ref() {
        let ids: Vec<String> = cards.iter().map(|c| c.scryfall_id.to_string()).collect();
        match catalog.text(&ids, &lang).await {
            Ok(entries) => {
                has_text = true;
                enrich(&mut cards, &entries);
            }
            // Card text is presentation. A catalog that is down costs a player
            // rules text and their own language, not the deck builder.
            Err(e) => tracing::warn!(%e, "pool text lookup failed; serving the registry alone"),
        }
    }
    if cards.is_empty() {
        return Err(err(StatusCode::INTERNAL_SERVER_ERROR, "empty card pool"));
    }
    Ok(Json(PoolBody {
        total: cards.len(),
        pool_hash: format!("{:016x}", baylee_cards::pool_hash()),
        lang,
        has_text,
        cards,
    }))
}

/// Fills in rules text and localized names from the catalog.
///
/// Matched on the printing codegen referenced, which is the one printing the
/// registry names. A card the catalog has never heard of keeps its registry
/// row rather than disappearing: the pool is what the *engine* can play, and
/// the catalog does not get a vote on that.
fn enrich(cards: &mut [PoolCard], entries: &[baylee_catalog::CardTextEntry]) {
    for entry in entries {
        let Some(card) = cards
            .iter_mut()
            .find(|c| c.scryfall_id == entry.scryfall_id)
        else {
            continue;
        };
        let Some(front) = entry.faces.first() else {
            continue;
        };
        if !front.name.is_empty() {
            card.name.clone_from(&front.name);
        }
        if !front.type_line.is_empty() {
            card.type_line.clone_from(&front.type_line);
        }
        // Both faces' text, in printed order: a modal card's back is part of
        // what a player is choosing between when they add it to a deck.
        card.oracle_text = entry
            .faces
            .iter()
            .map(|f| f.oracle_text.as_str())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n//\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(name: &str) -> PoolCard {
        registry_rows()
            .iter()
            .find(|c| c.english_name == name)
            .unwrap_or_else(|| panic!("the pool has {name}"))
            .clone()
    }

    /// The pool is the registry, whole. A card missing from it is a card no
    /// deck can be built with, however finished it is.
    #[test]
    fn the_pool_is_every_card_the_engine_knows() {
        assert_eq!(registry_rows().len(), baylee_cards::count());
        assert!(baylee_cards::count() > 100, "the pool should not be empty");
    }

    /// Every row has to carry what a deck builder draws with. A blank name or
    /// a missing type line is a row that renders as an empty box.
    #[test]
    fn every_row_can_be_drawn() {
        for card in registry_rows() {
            assert!(
                !card.english_name.is_empty(),
                "card {} has no name",
                card.index
            );
            assert!(
                !card.type_line.is_empty(),
                "{} has no type line",
                card.english_name
            );
            assert!(
                !card.scryfall_id.is_empty(),
                "{} names no printing",
                card.english_name
            );
        }
    }

    /// A basic land is the one card a deck may hold any number of, and the
    /// builder has to know which those are before it can say so.
    #[test]
    fn basic_lands_are_marked_and_nothing_else_is() {
        assert!(find("Forest").basic_land);
        assert!(find("Island").basic_land);
        assert!(!find("Ondu Cleric").basic_land);
    }

    /// The type line is what a player reads to know what a card *is*, so the
    /// subtypes have to survive the trip out of the registry.
    #[test]
    fn a_type_line_reads_as_printed() {
        assert_eq!(
            find("Ondu Cleric").type_line,
            "Creature — Human Cleric Ally"
        );
        assert_eq!(find("Ondu Cleric").kinds, vec!["Creature"]);
        assert_eq!(find("Forest").kinds, vec!["Land"]);
        assert_eq!(find("Forest").type_line, "Basic Land — Forest");
    }

    /// Cost, mana value and colors all come off the same cost, and a builder
    /// filters on each of them separately.
    #[test]
    fn a_cost_arrives_whole() {
        let cleric = find("Ondu Cleric");
        assert_eq!(cleric.mana_cost, "{1}{W}");
        assert_eq!(cleric.cmc, 2);
        assert_eq!(cleric.colors, "W");
        assert_eq!(cleric.stats.as_deref(), Some("1/1"));
        let forest = find("Forest");
        assert_eq!(forest.mana_cost, "", "a land prints no cost at all");
        assert_eq!(forest.cmc, 0);
        assert!(forest.colors.is_empty());
        assert_eq!(forest.stats, None);
    }

    /// An unimplemented stub must be visibly unplayable. The whole reason the
    /// pool is the registry rather than the catalog is that a player should
    /// never be offered a card that cannot be played.
    #[test]
    fn coverage_says_whether_a_card_can_be_played() {
        for card in registry_rows() {
            assert!(
                matches!(card.coverage, "implemented" | "partial" | "unimplemented"),
                "{} has an unknown coverage",
                card.english_name
            );
            assert_eq!(
                card.note.is_some(),
                card.coverage == "partial",
                "{} explains itself inconsistently",
                card.english_name
            );
        }
    }

    /// The catalog fills in text and translates a name; it never adds or
    /// removes a card, because it does not decide what the engine can play.
    #[test]
    fn the_catalog_fills_a_row_in_without_replacing_it() {
        let mut cards = vec![find("Ondu Cleric")];
        let id = cards[0].scryfall_id.to_string();
        let before = cards.len();
        enrich(
            &mut cards,
            &[baylee_catalog::CardTextEntry {
                scryfall_id: id,
                lang: "de".to_string(),
                faces: vec![baylee_catalog::FaceText {
                    name: "Ondu-Kleriker".to_string(),
                    english_name: "Ondu Cleric".to_string(),
                    type_line: "Kreatur — Mensch, Kleriker".to_string(),
                    oracle_text: "Immer wenn …".to_string(),
                    mana_cost: "{1}{W}".to_string(),
                }],
            }],
        );
        assert_eq!(cards.len(), before);
        assert_eq!(cards[0].name, "Ondu-Kleriker");
        assert_eq!(
            cards[0].english_name, "Ondu Cleric",
            "a deck is stored under the English name whatever the player reads"
        );
        assert_eq!(cards[0].oracle_text, "Immer wenn …");
        assert_eq!(cards[0].index, find("Ondu Cleric").index);
    }

    /// A printing the catalog has never seen leaves the row exactly as the
    /// registry built it.
    #[test]
    fn an_unknown_printing_changes_nothing() {
        let mut cards = vec![find("Forest")];
        let before = cards.clone();
        enrich(
            &mut cards,
            &[baylee_catalog::CardTextEntry {
                scryfall_id: "00000000-0000-0000-0000-000000000000".to_string(),
                lang: "en".to_string(),
                faces: vec![baylee_catalog::FaceText::default()],
            }],
        );
        assert_eq!(cards, before);
    }
}
