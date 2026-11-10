//! Alchemist's Refuge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}{U}, {T}: You may cast spells this turn as though they had flash.
//! Set: SOC #360 — Secrets of Strixhaven Commander | Scryfall ID: 36a11c11-1ea9-48e2-b1bc-49cba1390620 | Oracle ID: 357ed28b-899f-404b-94ff-6fb2ef81d87b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALCHEMIST_S_REFUGE,
    oracle_id = "357ed28b-899f-404b-94ff-6fb2ef81d87b",
    scryfall_id = "36a11c11-1ea9-48e2-b1bc-49cba1390620",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(name = "Alchemist's Refuge", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{G}{U}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SpellsHaveFlash(None),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);
