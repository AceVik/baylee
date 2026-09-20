//! Urza's Workshop — (no cost) — Land — Urza's
//! Oracle: {T}: Add {C}.
//! Oracle: Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts.
//! Set: BRC #28 — The Brothers' War Commander | Scryfall ID: 37c9b9d7-2fa7-4710-94bb-c55ee7bf598c | Oracle ID: 71099427-e110-488f-ab29-7867241fc7f0
// IMPLEMENTED — {C} always; the metalcraft half is a conditional mana
// ability whose amount is `Amount::CountOf` over the Urza's lands you
// control, gated by `Condition::ControlCount(ARTIFACT, 3)`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_WORKSHOP,
    oracle_id = "71099427-e110-488f-ab29-7867241fc7f0",
    scryfall_id = "37c9b9d7-2fa7-4710-94bb-c55ee7bf598c",
    faces = &[face!(
        name = "Urza's Workshop",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_dynamic(
                ManaColor::Colorless,
                Amount::CountOf {
                    filter: &f!(your Filter::HasSubtype(subtypes::land::URZA_S)),
                    zone: ZoneSel::Battlefield,
                },
            )],
            condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 3))
        ),
    ],
);
