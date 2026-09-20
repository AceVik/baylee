//! Blast Zone — (no cost) — Land
//! Oracle: This land enters with a charge counter on it.
//! Oracle: {T}: Add {C}.
//! Oracle: {X}{X}, {T}: Put X charge counters on this land.
//! Oracle: {3}, {T}, Sacrifice this land: Destroy each nonland permanent with mana value equal to the number of charge counters on this land.
//! Set: CMM #987 — Commander Masters | Scryfall ID: cdad14f1-d541-4e58-af9f-f8e587fca05f | Oracle ID: 393a254f-be31-431a-9341-a51286f8cbce
// PARTIAL — enters with a charge counter (EnterModifier::WithCounters), {T}:
// Add {C}, and {X}{X}, {T} for X more counters are built; the {3} sacrifice
// clause is not expressible.

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
    coverage = Coverage::Partial(
        "the {3}, {T}, Sacrifice clause destroys each nonland permanent with \
         mana value equal to the charge counters on this land, and no Filter \
         variant compares a mana value against a counter count on the source \
         (DestroyAll takes a static filter; a variant such as \
         Filter::CmcEqualToCountersOnSource { kind } would be needed)"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{X}{X}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::X,
            }]
        ),
        // NOT SUPPORTED: {3}, {T}, Sacrifice this land: Destroy each nonland
        // permanent with mana value equal to the number of charge counters on
        // this land.
    ],
);
