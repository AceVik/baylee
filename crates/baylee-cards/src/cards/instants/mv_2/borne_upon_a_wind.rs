//! Borne Upon a Wind — {1}{U} — Instant
//! Oracle: You may cast spells this turn as though they had flash.
//! Oracle: Draw a card.
//! Set: LTR #44 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: a9379675-1a32-4e2b-8aaf-5f908c595f31 | Oracle ID: ce19962d-94f9-4b2b-b668-963c0acce308

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BORNE_UPON_A_WIND,
    oracle_id = "ce19962d-94f9-4b2b-b668-963c0acce308",
    scryfall_id = "a9379675-1a32-4e2b-8aaf-5f908c595f31",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Borne Upon a Wind",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::continuous(
            &Filter::This,
            Modifier::SpellsHaveFlash(None),
            Duration::UntilEndOfTurn
        ),
        Effect::draw(1),
    ]),],
);
