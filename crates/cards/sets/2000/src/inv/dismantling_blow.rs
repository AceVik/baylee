//! Dismantling Blow — {2}{W} — Instant
//! Oracle: Kicker {2}{U} (You may pay an additional {2}{U} as you cast this spell.)
//! Oracle: Destroy target artifact or enchantment. If this spell was kicked, draw two cards.
//! Set: MH1 #5 — Modern Horizons | Scryfall ID: 70edc5da-4477-49f6-bbcc-f208fdaae0d3 | Oracle ID: 300cba5a-adbb-4852-be06-ac00d9d6fd37
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISMANTLING_BLOW,
    oracle_id = "300cba5a-adbb-4852-be06-ac00d9d6fd37",
    scryfall_id = "70edc5da-4477-49f6-bbcc-f208fdaae0d3",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Dismantling Blow",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
