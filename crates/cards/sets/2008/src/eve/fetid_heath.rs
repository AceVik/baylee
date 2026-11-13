//! Fetid Heath — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W/B}, {T}: Add {W}{W}, {W}{B}, or {B}{B}.
//! Set: SOC #372 — Secrets of Strixhaven Commander | Scryfall ID: f465ded8-0d38-42ac-bafc-a12185013c5d | Oracle ID: 42bf259d-4bb9-49c3-b4ec-223dca62f4d6
// IMPLEMENTED — filter land: a colorless tap, and a hybrid {W/B} that turns
// one mana into two, each of its own colour.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FETID_HEATH,
    oracle_id = "42bf259d-4bb9-49c3-b4ec-223dca62f4d6",
    scryfall_id = "f465ded8-0d38-42ac-bafc-a12185013c5d",
    faces = &[face!(name = "Fetid Heath", types = TypeSet::LAND,)],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{W/B}", TapSelf),
            &[Effect::mana_combination(
                &[ManaColor::White, ManaColor::Black],
                Amount::Fixed(2),
            )]
        ),
    ],
);
