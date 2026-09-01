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
    /// Rules identity, shared by every printing and every language.
    ///
    /// This is what `GET /printings` is keyed on: a player picking the art
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
        oracle_id: def.oracle_id,
        alt_names: Vec::new(),
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
        // Names in every language the catalog has. Separate query and separate
        // failure: a builder that cannot translate still searches, and one
        // that cannot search in Japanese still builds decks.
        let oracle_ids: Vec<String> = cards
            .iter()
            .map(|c| c.oracle_id.to_string())
            .filter(|id| !id.is_empty())
            .collect();
        match catalog.names(&oracle_ids).await {
            Ok(names) => name_cards(&mut cards, &names),
            Err(e) => tracing::warn!(%e, "pool name lookup failed; searching English only"),
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

/// Files every localized name onto the card it belongs to.
///
/// A card the catalog knows in one language only ends up with no alternates,
/// which is correct: there is nothing else to type.
fn name_cards(cards: &mut [PoolCard], names: &[baylee_catalog::LocalName]) {
    for local in names {
        let Some(card) = cards.iter_mut().find(|c| c.oracle_id == local.oracle_id) else {
            continue;
        };
        // The card's own two names are already searchable; repeating them
        // here would only make the answer bigger.
        if local.name == card.english_name || local.name == card.name {
            continue;
        }
        if !card.alt_names.contains(&local.name) {
            card.alt_names.push(local.name.clone());
        }
    }
}

/// Query for `GET /printings`.
#[derive(Deserialize)]
pub struct PrintingsQuery {
    /// Registry index of the card being picked for.
    card: u32,
}

/// The answer to `GET /printings`.
#[derive(Clone, Debug, Serialize)]
pub struct PrintingsBody {
    /// The card that was asked about.
    pub card: u32,
    /// Its English name, so a client can label the dialog before its own pool
    /// row is to hand.
    pub english_name: String,
    /// Whether these came from the catalog. `false` means the one printing
    /// below is the registry's own reference, and a picker should say so
    /// rather than implying the card was printed exactly once.
    pub from_catalog: bool,
    /// Every printing, newest set first.
    pub printings: Vec<baylee_catalog::Printing>,
}

/// `GET /printings?card=<index>` — every printing of one card.
///
/// Unauthenticated for the same reason `/pool` is: which sets a card appeared
/// in is public reference data.
///
/// Without a catalog the answer is not an error but a single row — the
/// printing codegen referenced, in English, plain. A picker that got a 503
/// here would have to grow a second code path for a gateway with no database;
/// one that always gets at least one printing does not, and the deck row it
/// writes is the same row either way.
pub async fn printings(
    State(state): State<Shared>,
    Query(params): Query<PrintingsQuery>,
) -> Result<Json<PrintingsBody>, (StatusCode, Json<ErrorBody>)> {
    let card = registry_rows()
        .iter()
        .find(|c| c.index == params.card)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such card"))?;

    let mut body = PrintingsBody {
        card: card.index,
        english_name: card.english_name.clone(),
        from_catalog: false,
        printings: vec![reference_printing(card)],
    };

    if let Some(catalog) = state.catalog.as_ref()
        && !card.oracle_id.is_empty()
    {
        match catalog.printings(&[card.oracle_id.to_string()]).await {
            Ok(found) if !found.is_empty() => {
                body.from_catalog = true;
                body.printings = found;
            }
            // An empty answer is a card the ingest has not reached, which is
            // the same situation as no catalog at all.
            Ok(_) => {}
            Err(e) => tracing::warn!(%e, "printing lookup failed; serving the reference printing"),
        }
    }
    Ok(Json(body))
}

/// The one printing the registry itself names, as a `Printing`.
///
/// Everything a catalog would know about it is left empty rather than guessed:
/// codegen records the id, not the set it came from.
fn reference_printing(card: &PoolCard) -> baylee_catalog::Printing {
    baylee_catalog::Printing {
        scryfall_id: card.scryfall_id.to_string(),
        oracle_id: card.oracle_id.to_string(),
        lang: "en".to_string(),
        set: String::new(),
        set_name: String::new(),
        collector_number: String::new(),
        rarity: String::new(),
        released_at: String::new(),
        artist: String::new(),
        finishes: vec!["nonfoil".to_string()],
        frame_effects: Vec::new(),
        border_color: String::new(),
        promo: false,
        name: card.english_name.clone(),
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
    /// The picker asks about a card, not about a printing, so every row has
    /// to carry the identity that question is keyed on.
    #[test]
    fn every_row_names_the_card_behind_its_printing() {
        for card in registry_rows() {
            assert!(
                !card.oracle_id.is_empty(),
                "{} has no oracle id, so no printing of it can be found",
                card.english_name
            );
        }
    }

    /// Without a catalog the picker still gets a printing to pick — the one
    /// the registry itself names — and it is honestly marked as the only one
    /// this build knows about.
    #[test]
    fn a_gateway_with_no_catalog_still_offers_one_printing() {
        let strix = find("Baleful Strix");
        let printing = reference_printing(&strix);
        assert_eq!(printing.scryfall_id, strix.scryfall_id);
        assert_eq!(printing.oracle_id, strix.oracle_id);
        assert_eq!(printing.lang, "en");
        assert_eq!(printing.finishes, vec!["nonfoil".to_string()]);
        assert_eq!(printing.name, "Baleful Strix");
    }

    /// A German player types "Blitzschlag"; the deck stores "Lightning Bolt".
    /// Both names have to reach the same row.
    #[test]
    fn a_card_collects_its_names_and_not_the_ones_it_already_has() {
        let mut cards = vec![find("Counterspell")];
        let oracle = cards[0].oracle_id.to_string();
        let names = [
            // The English name is already searchable.
            ("en", "Counterspell"),
            ("de", "Gegenzauber"),
            ("ja", "対抗呪文"),
            // A second German printing of the same card, same name.
            ("de", "Gegenzauber"),
            // Another card's name must not land here.
            ("de", "Blitzschlag"),
        ];
        let locals: Vec<baylee_catalog::LocalName> = names
            .iter()
            .map(|(lang, name)| baylee_catalog::LocalName {
                oracle_id: if *name == "Blitzschlag" {
                    "someone-else".to_string()
                } else {
                    oracle.clone()
                },
                lang: (*lang).to_string(),
                name: (*name).to_string(),
            })
            .collect();

        name_cards(&mut cards, &locals);
        assert_eq!(cards[0].alt_names, vec!["Gegenzauber", "対抗呪文"]);
    }

    /// The largest field in the answer is the one most rows do not have, so
    /// it has to leave the wire entirely when it is empty.
    #[test]
    fn a_card_with_no_other_names_costs_nothing_to_send() {
        let plain = find("Forest");
        let json = serde_json::to_string(&plain).expect("serializes");
        assert!(!json.contains("alt_names"), "{json}");

        let mut named = [plain];
        named[0].alt_names = vec!["Wald".to_string()];
        let json = serde_json::to_string(&named[0]).expect("serializes");
        assert!(json.contains(r#""alt_names":["Wald"]"#), "{json}");
    }
}
