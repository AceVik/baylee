//! Great Hall of Starnheim — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {W}{W}{B}, {T}, Sacrifice this land and a creature you control: Create a 4/4 white Angel Warrior creature token with flying and vigilance. Activate only as a sorcery.
//! Set: KHM #259 — Kaldheim | Scryfall ID: a23c757e-5944-47ce-b06f-27b4c403044c | Oracle ID: 75d8ef26-b745-448b-94b9-3c64668ce171
// IMPLEMENTED — enters tapped, taps for {B}, and sells itself and a creature
// for the Angel.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREAT_HALL_OF_STARNHEIM,
    oracle_id = "75d8ef26-b745-448b-94b9-3c64668ce171",
    scryfall_id = "a23c757e-5944-47ce-b06f-27b4c403044c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Great Hall of Starnheim",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // "Sacrifice this land **and** a creature you control" is two cost
        // parts and not one: `SacrificeSelf` cannot be answered with the
        // creature and `Sacrifice(&Filter::YOUR_CREATURE)` may not be
        // answered with the land, so a player holding only the Hall cannot
        // pay at all — which is the printing.
        activated!(
            cost!(
                "{W}{W}{B}",
                TapSelf,
                SacrificeSelf,
                Sacrifice(&Filter::YOUR_CREATURE)
            ),
            &[Effect::CreateToken {
                token: &generated_tokens::ANGEL_WARRIOR_4_4_WHITE_FLYING_VIGILANCE
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
