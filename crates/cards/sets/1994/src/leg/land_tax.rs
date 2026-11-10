//! Land Tax — {W} — Enchantment
//! Oracle: At the beginning of your upkeep, if an opponent controls more lands than you, you may search your library for up to three basic land cards, reveal them, put them into your hand, then shuffle.
//! Set: SOC #153 — Secrets of Strixhaven Commander | Scryfall ID: c70d3706-9d28-49ac-8718-4a516413e727 | Oracle ID: d2d9ecea-7925-420e-98b9-2f87f41f387c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_TAX,
    oracle_id = "d2d9ecea-7925-420e-98b9-2f87f41f387c",
    scryfall_id = "c70d3706-9d28-49ac-8718-4a516413e727",
    color_identity = ColorSet::from_slice(&[Color::White]),
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::MayDo {
            effects: &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::HAND, Find::HAND, Find::HAND],
                optional: true
            }],
        }],
        condition = Some(Condition::OpponentControlsMore(&Filter::LAND))
    )],
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Land Tax",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
