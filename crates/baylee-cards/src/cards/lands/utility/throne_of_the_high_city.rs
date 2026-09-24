//! Throne of the High City — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Sacrifice this land: You become the monarch.
//! Set: MSC #274 — Marvel Super Heroes Commander | Scryfall ID: 6aac7a6d-90c7-4fe7-b6d3-f63a7170ee96 | Oracle ID: 9684447a-5955-4bc7-8ad0-8bb8b316873b
// IMPLEMENTED — colorless mana ability and the {4}, {T}, sacrifice activation
// that makes you the monarch.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRONE_OF_THE_HIGH_CITY,
    oracle_id = "9684447a-5955-4bc7-8ad0-8bb8b316873b",
    scryfall_id = "6aac7a6d-90c7-4fe7-b6d3-f63a7170ee96",
    faces = &[face!(
        name = "Throne of the High City",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::BecomeMonarch(PlayerRel::You)]
        ),
    ],
);
