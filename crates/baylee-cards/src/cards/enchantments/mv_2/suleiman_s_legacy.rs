//! Suleiman's Legacy — {R}{W} — Enchantment
//! Oracle: When this enchantment enters, destroy all Djinns and Efreets. They can't be regenerated.
//! Oracle: Whenever a Djinn or Efreet enters, destroy it. It can't be regenerated.
//! Set: VIS #138 — Visions | Scryfall ID: 3a15e970-e605-425a-b4ec-391d9cacde38 | Oracle ID: 9888d67d-fcad-427f-8ba7-b6d421b3d88e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DJINN_OR_EFREET: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::creature::DJINN),
    Filter::HasSubtype(subtypes::creature::EFREET),
]);

card!(
    index = index::SULEIMAN_S_LEGACY,
    oracle_id = "9888d67d-fcad-427f-8ba7-b6d421b3d88e",
    scryfall_id = "3a15e970-e605-425a-b4ec-391d9cacde38",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Suleiman's Legacy",
        mana_cost = mana!("{R}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::destroy_all_no_regen(&DJINN_OR_EFREET)],
        ),
        triggered!(
            Trigger::EntersBattlefield(&DJINN_OR_EFREET),
            &[Effect::destroy_no_regen(TargetSpec::EventObject)],
        ),
    ],
);
