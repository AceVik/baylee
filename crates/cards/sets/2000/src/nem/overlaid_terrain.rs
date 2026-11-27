//! Overlaid Terrain — {2}{G}{G} — Enchantment
//! Oracle: As this enchantment enters, sacrifice all lands you control.
//! Oracle: Lands you control have "{T}: Add two mana of any one color."
//! Set: NEM #108 — Nemesis | Scryfall ID: 230c7926-9a4b-4ead-b4c8-889f84210545 | Oracle ID: e5818ab3-8c37-4063-9e9d-cbe4ae0bba14
// IMPLEMENTED — sacrifices all your lands on entry and grants lands you control "{T}: Add two mana of any one color."

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERLAID_TERRAIN,
    oracle_id = "e5818ab3-8c37-4063-9e9d-cbe4ae0bba14",
    scryfall_id = "230c7926-9a4b-4ead-b4c8-889f84210545",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Overlaid Terrain",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::SacrificeOthers(&Filter::YOUR_LAND)],
    ),],
    abilities = &[static_ability!(
        Filter::YOUR_LAND,
        Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: &[Effect::mana_choice_dynamic(
                ALL_MANA_COLORS,
                Amount::Fixed(2),
            )],
            mana_ability: true,
        }
    ),],
);
