//! Pillar of the Paruns — (no cost) — Land
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a multicolored spell.
//! Set: 2X2 #328 — Double Masters 2022 | Scryfall ID: 4c66bacf-ad9a-40c3-a2b7-464be8d4dd24 | Oracle ID: 677b8ce7-f922-4ee3-b311-f199da9b352b
// IMPLEMENTED — the mana ability, with its printed spend restriction attached
// to the mana (`Effect::restricted`, restriction only, no rider).

use baylee_cards_dsl::prelude::*;

/// A spell with two or more colors (CR 202.2c) — the printed "multicolored".
///
/// Spelled as the two negations rather than as an `Or` over the ten color
/// pairs, and the colorless half is load-bearing: `Not(&Monocolored)` on its
/// own matches a colorless artifact spell, which is exactly the cast this
/// mana may not pay for.
static MULTICOLORED_SPELL: Filter = Filter::And(&[
    Filter::Not(&Filter::IsColorless),
    Filter::Not(&Filter::Monocolored),
]);

card!(
    index = index::PILLAR_OF_THE_PARUNS,
    oracle_id = "677b8ce7-f922-4ee3-b311-f199da9b352b",
    scryfall_id = "4c66bacf-ad9a-40c3-a2b7-464be8d4dd24",
    faces = &[face!(name = "Pillar of the Paruns", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[
        Effect::mana_of_any_color().restricted(&MULTICOLORED_SPELL, SpendRider::None)
    ])],
);
