//! Worldly Tutor — {G} — Instant
//! Oracle: Search your library for a creature card, reveal it, then shuffle and put the card on top.
//! Set: DMR #185 — Dominaria Remastered | Scryfall ID: f39aa2e9-e294-4ce6-bf5e-e1f579101a7a | Oracle ID: e8863518-0bfa-49c3-8c6e-6c9116a81051
// IMPLEMENTED — creature tutor to the top of the library. The reveal is the
// engine's own: a search narrower than "a card" that ends somewhere hidden
// shows its find, and the shuffle happens before the card is placed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORLDLY_TUTOR,
    oracle_id = "e8863518-0bfa-49c3-8c6e-6c9116a81051",
    scryfall_id = "f39aa2e9-e294-4ce6-bf5e-e1f579101a7a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Worldly Tutor",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::CREATURE,
        finds: &[Find::TOP_OF_LIBRARY],
        optional: false,
    }])],
);
