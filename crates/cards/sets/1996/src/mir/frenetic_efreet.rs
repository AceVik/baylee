//! Frenetic Efreet — {1}{U}{R} — Creature — Efreet
//! Oracle: Flying
//! Oracle: {0}: Flip a coin. If you win the flip, this creature phases out. If you lose the flip, sacrifice this creature. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: MIR #264 — Mirage | Scryfall ID: 50d4468b-f7de-44fe-898a-4125d26d242f | Oracle ID: c21d3360-8be0-477e-a592-7e64a76570d3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FRENETIC_EFREET,
    oracle_id = "c21d3360-8be0-477e-a592-7e64a76570d3",
    scryfall_id = "50d4468b-f7de-44fe-898a-4125d26d242f",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Frenetic Efreet",
        mana_cost = mana!("{1}{U}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(2),
        toughness = Some(1),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!("{0}"),
        &[Effect::FlipCoin {
            won: &[Effect::PhaseOut { target: None }],
            lost: &[Effect::SacrificeSelf],
        }],
    )],
);
