//! Borne Upon a Wind — {1}{U} — Instant
//! Oracle: You may cast spells this turn as though they had flash.
//! Oracle: Draw a card.
//! Set: LTR #44 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: a9379675-1a32-4e2b-8aaf-5f908c595f31 | Oracle ID: ce19962d-94f9-4b2b-b668-963c0acce308
// IMPLEMENTED — draw a card; the flash grant is Partial.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BORNE_UPON_A_WIND,
    oracle_id = "ce19962d-94f9-4b2b-b668-963c0acce308",
    scryfall_id = "a9379675-1a32-4e2b-8aaf-5f908c595f31",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the turn-long \"you may cast spells this turn as though they had flash\" has no \
         Modifier variant: Modifier::SorceriesHaveFlash reaches sorcery spells only"
    ),
    faces = &[face!(
        name = "Borne Upon a Wind",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        // NOT SUPPORTED: "You may cast spells this turn as though they had
        // flash" — no Modifier grants flash to every spell for a turn;
        // Modifier::SorceriesHaveFlash covers sorceries alone.
        spell!(&[Effect::draw(1)]),
    ],
);
