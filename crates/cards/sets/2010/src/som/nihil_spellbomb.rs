//! Nihil Spellbomb — {1} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Exile target player's graveyard.
//! Oracle: When this artifact is put into a graveyard from the battlefield, you may pay {B}. If you do, draw a card.
//! Set: A25 #226 — Masters 25 | Scryfall ID: e5892a23-efae-4731-9b8f-41c87960fe93 | Oracle ID: b713e49f-1b13-42d1-91f2-cc7a579e7614
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NIHIL_SPELLBOMB,
    oracle_id = "b713e49f-1b13-42d1-91f2-cc7a579e7614",
    scryfall_id = "e5892a23-efae-4731-9b8f-41c87960fe93",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nihil Spellbomb",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
