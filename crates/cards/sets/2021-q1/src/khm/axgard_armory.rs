//! Axgard Armory — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{R}{R}{W}, {T}, Sacrifice this land: Search your library for an Aura card and/or an Equipment card, reveal them, put them into your hand, then shuffle.
//! Set: KHM #250 — Kaldheim | Scryfall ID: f9c77d35-8418-48fb-b7d7-7cfa763545c5 | Oracle ID: bce30fd0-ed1e-495d-9149-6a4c81c45c7b
// IMPLEMENTED — enters tapped (EnterModifier::Tapped), taps for {W}, and the
// sacrifice activation searches for up to two cards matching the printed
// "Aura card and/or an Equipment card" filter, into hand (revealed, then
// shuffled, both derived).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AXGARD_ARMORY,
    oracle_id = "bce30fd0-ed1e-495d-9149-6a4c81c45c7b",
    scryfall_id = "f9c77d35-8418-48fb-b7d7-7cfa763545c5",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Axgard Armory",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{R}{R}{W}", TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::Or(&[
                    Filter::HasSubtype(subtypes::enchantment::AURA),
                    Filter::HasSubtype(subtypes::artifact::EQUIPMENT),
                ]),
                finds: &[Find::HAND, Find::HAND],
                optional: true,
            }]
        ),
    ],
);
