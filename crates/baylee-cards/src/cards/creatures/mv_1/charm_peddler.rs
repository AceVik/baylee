//! Charm Peddler — {W} — Creature — Human Spellshaper
//! Oracle: {W}, {T}, Discard a card: The next time a source of your choice would deal damage to target creature this turn, prevent that damage.
//! Set: MMQ #6 — Mercadian Masques | Scryfall ID: 082e6ee3-cc1f-46c7-9d82-56751478b3cf | Oracle ID: c425fe26-0027-4319-80a9-a053925651cb
// NOT SUPPORTED: optional Spellshaper ability preventing the next damage from a chosen source to target creature is not implemented
// PARTIAL — 1/1 Human Spellshaper body is implemented; optional `{W}, {T}, Discard a card` chosen-source prevention shield on a targeted creature is omitted because `PreventNextFromChosenSource` only shields `you`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHARM_PEDDLER,
    oracle_id = "c425fe26-0027-4319-80a9-a053925651cb",
    scryfall_id = "082e6ee3-cc1f-46c7-9d82-56751478b3cf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "optional Spellshaper ability preventing the next damage from a chosen source to target creature is not implemented",
    ),
    faces = &[face!(
        name = "Charm Peddler",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);
