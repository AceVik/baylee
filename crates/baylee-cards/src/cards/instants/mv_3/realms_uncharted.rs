//! Realms Uncharted — {2}{G} — Instant
//! Oracle: Search your library for up to four land cards with different names and reveal them. An opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest into your hand. Then shuffle.
//! Set: ROE #206 — Rise of the Eldrazi | Scryfall ID: 8967ceff-9e25-46da-926b-e05f4ee98325 | Oracle ID: e21c8fc6-d4ef-42b6-b11e-d9c931da1387
// IMPLEMENTED — up to four lands with different names, an opponent sends two
// to the graveyard, the rest go to hand, then a shuffle.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REALMS_UNCHARTED,
    oracle_id = "e21c8fc6-d4ef-42b6-b11e-d9c931da1387",
    scryfall_id = "8967ceff-9e25-46da-926b-e05f4ee98325",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Realms Uncharted",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::SearchOpponentSplits {
        filter: &Filter::LAND,
        up_to: 4,
        chosen: 2,
    }])],
);
