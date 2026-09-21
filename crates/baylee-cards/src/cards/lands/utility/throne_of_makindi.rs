//! Throne of Makindi — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Put a charge counter on this land.
//! Oracle: {T}, Remove a charge counter from this land: Add two mana of any one color. Spend this mana only to cast kicked spells.
//! Set: ZNR #265 — Zendikar Rising | Scryfall ID: d5a0563e-c83b-40df-abf6-51c83bf6792d | Oracle ID: 7e8198e9-0f3b-420b-ab09-74f13f4fd548
// PARTIAL — tap for {C} and {1}, {T} to add a charge counter are implemented;
// the restricted mana ability for kicked spells cannot be filtered in ManaRestriction.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRONE_OF_MAKINDI,
    oracle_id = "7e8198e9-0f3b-420b-ab09-74f13f4fd548",
    scryfall_id = "d5a0563e-c83b-40df-abf6-51c83bf6792d",
    coverage = Coverage::Partial("ManaRestriction has no filter for kicked spells"),
    faces = &[face!(name = "Throne of Makindi", types = TypeSet::LAND,)],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
        ),
        // NOT SUPPORTED: {T}, Remove a charge counter from this land: Add two mana of any one color. Spend this mana only to cast kicked spells.
    ],
);
