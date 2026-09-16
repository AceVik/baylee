//! Delighted Halfling — {G} — Creature — Halfling Citizen
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that spell can't be countered.
//! Set: LTR #158 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 71384418-173a-4f77-adab-56e52fa23692 | Oracle ID: f9d3b046-0b95-4103-a630-4b3fb88bb60b
// IMPLEMENTED — two mana abilities: a plain {C}, and any-colour mana that
// pays for legendary spells only and makes the one it pays for
// uncounterable (mana provenance + the Uncounterable rider).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "a legendary spell" is the Legendary *supertype* and nothing else — a
/// legendary spell of any card type qualifies, so no type is named here.
static LEGENDARY_SPELL: Filter = Filter::HasSupertype(SupertypeSet::LEGENDARY);

card!(
    index = index::DELIGHTED_HALFLING,
    oracle_id = "f9d3b046-0b95-4103-a630-4b3fb88bb60b",
    scryfall_id = "71384418-173a-4f77-adab-56e52fa23692",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Delighted Halfling",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HALFLING, subtypes::creature::CITIZEN],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&LEGENDARY_SPELL, SpendRider::Uncounterable)
        ]),
    ],
);
