//! `GET /pool` and `GET /printings`: the playable pool, served.
//!
//! What the pool *is* — the compiled registry as builder rows, with
//! `Coverage` on every one of them — is `baylee_cards::pool`, because it is a
//! fact about the registry rather than about this server. It moved there when
//! offline play needed the same answer with no gateway in the picture: two
//! copies of that mapping would be two answers to "what may I put in a deck",
//! and the copy nobody with an account ever sees is the one that would rot.
//!
//! What this module adds is the two things only a gateway has. A catalog
//! fills in rules text and localized names ([`enrich`], [`name_cards`]), and
//! neither is available until one has been ingested. The database is no
//! longer the optional half — a gateway without `DATABASE_URL` refuses to
//! start at all, because that is where the accounts live — but an *empty*
//! one still leaves a working deck builder that cannot show text or
//! translate a name.
//! And the answer is sent **whole** (a few hundred rows), which is what lets a
//! client filter at keystroke latency instead of asking per letter. When the
//! registry outgrows one response this is where paging goes; the shape of the
//! answer already carries `total`.

use crate::{ErrorBody, Shared, err};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Json, Response};
use baylee_cards::pool::{PoolCard, rows as registry_rows};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// `GET /pool` — every card a deck may be built from.
///
/// Deliberately unauthenticated, for the same reason `/catalog/text` is: this
/// is what the game can play, which is public reference data, and a player
/// looking at what the platform supports has not signed up yet.
///
/// The answer is built once per language and catalog version and sent as
/// the bytes it was serialized to ([`crate::texts`]): it is a few hundred
/// rows that are the same for every reader until an ingest changes them.
pub async fn pool(State(state): State<Shared>, Query(params): Query<PoolQuery>) -> Response {
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();
    if let Some(catalog) = state.catalog.as_ref() {
        match state.texts.get(catalog, &lang).await {
            Ok(language) => {
                return ([(CONTENT_TYPE, "application/json")], language.pool_json())
                    .into_response();
            }
            // Card text is presentation. A catalog that is down costs a
            // player rules text and their own language, not the deck builder.
            Err(e) => tracing::warn!(%e, "pool text lookup failed; serving the registry alone"),
        }
    }
    Json(PoolBody {
        total: registry_rows().len(),
        pool_hash: format!("{:016x}", baylee_cards::pool_hash()),
        lang,
        has_text: false,
        cards: registry_rows().to_vec(),
    })
    .into_response()
}

/// The pool as `/pool` sends it in `lang`: the registry's rows with the
/// catalog's text and every name the cards are printed under.
pub(crate) fn body(
    lang: &str,
    by_card: &HashMap<String, baylee_catalog::CardTextEntry>,
    names: &[baylee_catalog::LocalName],
) -> PoolBody {
    let mut cards = registry_rows().to_vec();
    enrich(&mut cards, by_card);
    name_cards(&mut cards, names);
    PoolBody {
        total: cards.len(),
        pool_hash: format!("{:016x}", baylee_cards::pool_hash()),
        lang: lang.to_owned(),
        has_text: true,
        cards,
    }
}

/// Fills in rules text and localized names from the catalog.
///
/// Matched on the card, not on the printing codegen referenced: the text is
/// the card's (`Catalog::text_by_card`), and 24 reference printings are not
/// in the catalog at all while their cards are. A card the catalog has never
/// heard of keeps its registry row rather than disappearing: the pool is what
/// the *engine* can play, and the catalog does not get a vote on that.
fn enrich(cards: &mut [PoolCard], by_card: &HashMap<String, baylee_catalog::CardTextEntry>) {
    for card in cards {
        let Some(entry) = by_card.get(card.oracle_id) else {
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
    let mut at: HashMap<&str, usize> = HashMap::with_capacity(cards.len());
    for (i, card) in cards.iter().enumerate() {
        at.entry(card.oracle_id).or_insert(i);
    }
    for local in names {
        let Some(&i) = at.get(local.oracle_id.as_str()) else {
            continue;
        };
        let card = &mut cards[i];
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
        layout: if card.has_back_image {
            "transform".to_string()
        } else {
            String::new()
        },
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
    ///
    /// It said Human until `validate` learned to ask the printing, and Ondu
    /// Cleric is a **Kor** Cleric Ally. This test was written from the card
    /// rather than from the printing, so it pinned the wrong word — the same
    /// way `triome_cycling_from_hand_draws` pinned a `{2}` cycling cost the
    /// card never had.
    #[test]
    fn a_type_line_reads_as_printed() {
        assert_eq!(find("Ondu Cleric").type_line, "Creature — Kor Cleric Ally");
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

    /// One card's entry, filed under its card as the text cache holds it.
    fn filed(
        oracle_id: &str,
        face: baylee_catalog::FaceText,
    ) -> HashMap<String, baylee_catalog::CardTextEntry> {
        HashMap::from([(
            oracle_id.to_string(),
            baylee_catalog::CardTextEntry {
                oracle_id: oracle_id.to_string(),
                layout: "normal".to_string(),
                scryfall_id: "00000000-0000-0000-0000-000000000001".to_string(),
                lang: "de".to_string(),
                faces: vec![face],
            },
        )])
    }

    /// The catalog fills in text and translates a name; it never adds or
    /// removes a card, because it does not decide what the engine can play.
    /// It is matched on the card and not on the printing codegen referenced,
    /// which the entry here is not.
    #[test]
    fn the_catalog_fills_a_row_in_without_replacing_it() {
        let mut cards = vec![find("Ondu Cleric")];
        let before = cards.len();
        let by_card = filed(
            cards[0].oracle_id,
            baylee_catalog::FaceText {
                printed: Some("Immer wenn …".to_string()),
                name: "Ondu-Kleriker".to_string(),
                english_name: "Ondu Cleric".to_string(),
                type_line: "Kreatur — Mensch, Kleriker".to_string(),
                oracle_text: "Immer wenn …".to_string(),
                mana_cost: "{1}{W}".to_string(),
            },
        );
        enrich(&mut cards, &by_card);
        assert_eq!(cards.len(), before);
        assert_eq!(cards[0].name, "Ondu-Kleriker");
        assert_eq!(
            cards[0].english_name, "Ondu Cleric",
            "a deck is stored under the English name whatever the player reads"
        );
        assert_eq!(cards[0].oracle_text, "Immer wenn …");
        assert_eq!(cards[0].index, find("Ondu Cleric").index);
    }

    /// A card the catalog has never seen leaves the row exactly as the
    /// registry built it.
    #[test]
    fn an_unknown_card_changes_nothing() {
        let mut cards = vec![find("Forest")];
        let before = cards.clone();
        let by_card = filed(
            "00000000-0000-0000-0000-000000000000",
            baylee_catalog::FaceText::default(),
        );
        enrich(&mut cards, &by_card);
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
