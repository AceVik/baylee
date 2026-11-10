//! Blast Zone — (no cost) — Land
//! Oracle: This land enters with a charge counter on it.
//! Oracle: {T}: Add {C}.
//! Oracle: {X}{X}, {T}: Put X charge counters on this land.
//! Oracle: {3}, {T}, Sacrifice this land: Destroy each nonland permanent with mana value equal to the number of charge counters on this land.
//! Set: CMM #987 — Commander Masters | Scryfall ID: cdad14f1-d541-4e58-af9f-f8e587fca05f | Oracle ID: 393a254f-be31-431a-9341-a51286f8cbce

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLAST_ZONE,
    oracle_id = "393a254f-be31-431a-9341-a51286f8cbce",
    scryfall_id = "cdad14f1-d541-4e58-af9f-f8e587fca05f",
    faces = &[face!(
        name = "Blast Zone",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{X}{X}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::X,
            }]
        ),
        activated!(
            cost!("{3}", TapSelf, SacrificeSelf),
            &[Effect::DestroyAllWithManaValue {
                filter: &Filter::Not(&Filter::LAND),
                value: Amount::CountersOnSource(CounterKind::Charge),
            }]
        ),
    ],
);
