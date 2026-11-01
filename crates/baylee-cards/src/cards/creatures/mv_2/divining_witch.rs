//! Divining Witch — {1}{B} — Creature — Human Spellshaper
//! Oracle: {1}{B}, {T}, Discard a card: Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way.
//! Set: NEM #57 — Nemesis | Scryfall ID: be981eef-9dd2-4233-82c9-03f9f2e82c59 | Oracle ID: ebd77c52-97ea-421f-99eb-c6e2263cdecf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DIVINING_WITCH,
    oracle_id = "ebd77c52-97ea-421f-99eb-c6e2263cdecf",
    scryfall_id = "be981eef-9dd2-4233-82c9-03f9f2e82c59",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Divining Witch",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
