//! Interplanar Beacon — (no cost) — Land
//! Oracle: Whenever you cast a planeswalker spell, you gain 1 life.
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add two mana of different colors. Spend this mana only to cast planeswalker spells.
//! Set: CMM #1004 — Commander Masters | Scryfall ID: bc1bed72-2440-4364-a69f-a9d7c4fe3fea | Oracle ID: 073169f2-da3a-4a93-8c01-b3fd8558d225
// PARTIAL — the life trigger and both mana abilities are built; the third
// one cannot require its two mana to be of different colors.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTERPLANAR_BEACON,
    oracle_id = "073169f2-da3a-4a93-8c01-b3fd8558d225",
    scryfall_id = "bc1bed72-2440-4364-a69f-a9d7c4fe3fea",
    faces = &[face!(name = "Interplanar Beacon", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"Add two mana of different colors\" is not sayable: the only \
         multi-mana line the DSL has picks each mana independently, so the \
         ability may add {W}{W}"
    ),
    abilities = &[
        triggered!(
            Trigger::SpellCast(&f!(your PLANESWALKER)),
            &[Effect::gain_life(1)]
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Add two mana of different colors" — no ManaSource
        // can require the two picks to differ, so this may add {W}{W}. The
        // spending restriction is the tail of the same line and is written
        // where the card writes it.
        mana_ability!(
            cost!("{1}", TapSelf),
            &[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(2))
                .restricted(&Filter::PLANESWALKER, SpendRider::None)]
        ),
    ],
);
