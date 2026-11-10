//! Pulsemage Advocate — {2}{W} — Creature — Human Cleric
//! Oracle: {T}: Return three target cards from an opponent's graveyard to their hand. Return target creature card from your graveyard to the battlefield.
//! Set: JUD #19 — Judgment | Scryfall ID: 0dce0e8f-9ad6-42b6-af61-c883613efc97 | Oracle ID: 06e4355b-02d4-4238-a6e6-b03a5773346c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PULSEMAGE_ADVOCATE,
    oracle_id = "06e4355b-02d4-4238-a6e6-b03a5773346c",
    scryfall_id = "0dce0e8f-9ad6-42b6-af61-c883613efc97",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pulsemage Advocate",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
