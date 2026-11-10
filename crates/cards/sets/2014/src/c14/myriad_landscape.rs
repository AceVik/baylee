//! Myriad Landscape — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Search your library for up to two basic land cards that share a land type, put them onto the battlefield tapped, then shuffle.
//! Set: EOC #169 — Edge of Eternities Commander | Scryfall ID: a0e2098f-1d94-491a-a7e9-a45a9f69e3a8 | Oracle ID: 2549bc57-9ffb-4053-9f10-f2a5f792b845

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYRIAD_LANDSCAPE,
    oracle_id = "2549bc57-9ffb-4053-9f10-f2a5f792b845",
    scryfall_id = "a0e2098f-1d94-491a-a7e9-a45a9f69e3a8",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Myriad Landscape",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::SearchSharedLandType]
        ),
    ],
);
