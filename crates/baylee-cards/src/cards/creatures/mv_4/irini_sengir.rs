//! Irini Sengir — {2}{B}{B} — Legendary Creature — Vampire Dwarf
//! Oracle: Green enchantment spells and white enchantment spells cost {2} more to cast.
//! Set: HML #54 — Homelands | Scryfall ID: 518e3b77-d482-4b90-94c0-0b8cdd949b9f | Oracle ID: 2727a7ec-1cf7-4cd7-b5d4-955db940ddd9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_OR_WHITE_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::Or(&[
        Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
        Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    ]),
]);

card!(
    index = index::IRINI_SENGIR,
    oracle_id = "2727a7ec-1cf7-4cd7-b5d4-955db940ddd9",
    scryfall_id = "518e3b77-d482-4b90-94c0-0b8cdd949b9f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Irini Sengir",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::DWARF],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        GREEN_OR_WHITE_ENCHANTMENT,
        Modifier::SpellsCostMoreMana(mana!("{2}"))
    )],
);
