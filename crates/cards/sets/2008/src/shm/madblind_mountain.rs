//! Madblind Mountain — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped.
//! Oracle: {R}, {T}: Shuffle your library. Activate only if you control two or more red permanents.
//! Set: SHM #274 — Shadowmoor | Scryfall ID: 513adae2-6436-4284-9f23-87ef627e81b7 | Oracle ID: 0ee0b090-3f1e-49d6-bcad-91e0cf1d12ae

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MADBLIND_MOUNTAIN,
    oracle_id = "0ee0b090-3f1e-49d6-bcad-91e0cf1d12ae",
    scryfall_id = "513adae2-6436-4284-9f23-87ef627e81b7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Madblind Mountain",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::MOUNTAIN],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::intrinsic_mana()]),
        activated!(
            cost!("{R}", TapSelf),
            &[Effect::ShuffleLibrary {
                who: PlayerRel::You
            }],
            condition = Some(Condition::ControlCount(
                &Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
                2
            )),
        ),
    ],
);
