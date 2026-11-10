//! Interplanar Beacon — (no cost) — Land
//! Oracle: Whenever you cast a planeswalker spell, you gain 1 life.
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add two mana of different colors. Spend this mana only to cast planeswalker spells.
//! Set: CMM #1004 — Commander Masters | Scryfall ID: bc1bed72-2440-4364-a69f-a9d7c4fe3fea | Oracle ID: 073169f2-da3a-4a93-8c01-b3fd8558d225
// Batch 46: every printed ability is implemented; package validation is pending.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTERPLANAR_BEACON,
    oracle_id = "073169f2-da3a-4a93-8c01-b3fd8558d225",
    scryfall_id = "bc1bed72-2440-4364-a69f-a9d7c4fe3fea",
    faces = &[face!(name = "Interplanar Beacon", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::SpellCast(&f!(your PLANESWALKER)),
            &[Effect::gain_life(1)]
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{1}", TapSelf),
            &[
                Effect::mana_different_colors(ALL_MANA_COLORS, Amount::Fixed(2))
                    .restricted(&Filter::PLANESWALKER, SpendRider::None)
            ]
        ),
    ],
);
