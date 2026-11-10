//! Surtland Frostpyre — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{U}{U}{R}, {T}, Sacrifice this land: Scry 2. This land deals 2 damage to each creature. Activate only as a sorcery.
//! Set: KHM #271 — Kaldheim | Scryfall ID: 0b02149a-4a8f-4be9-9944-3d216248b549 | Oracle ID: 965aa666-3919-4053-8584-b773bdd54f0b
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SURTLAND_FROSTPYRE,
    oracle_id = "965aa666-3919-4053-8584-b773bdd54f0b",
    scryfall_id = "0b02149a-4a8f-4be9-9944-3d216248b549",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Surtland Frostpyre",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{2}{U}{U}{R}", TapSelf, SacrificeSelf),
            &[Effect::scry(2), Effect::damage_each(2, &Filter::CREATURE)],
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
