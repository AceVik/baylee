//! Rain of Filth — {B} — Instant
//! Oracle: Until end of turn, lands you control gain "Sacrifice this land: Add {B}."
//! Set: USG #151 — Urza's Saga | Scryfall ID: 1cf07d5a-6618-48e2-a9f0-669b75bb6e85 | Oracle ID: aa77980d-0434-40f0-990f-1c8c50610174
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAIN_OF_FILTH,
    oracle_id = "aa77980d-0434-40f0-990f-1c8c50610174",
    scryfall_id = "1cf07d5a-6618-48e2-a9f0-669b75bb6e85",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rain of Filth",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::YOUR_LAND,
        Modifier::GrantActivated {
            cost: cost!(SacrificeSelf),
            effects: &[Effect::mana(ManaColor::Black, 1)],
            mana_ability: true,
        },
        Duration::UntilEndOfTurn,
    )]),],
);
