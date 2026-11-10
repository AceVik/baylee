//! Mariposa Military Base — (no cost) — Land
//! Oracle: You may have this land enter tapped. If you do, you get two rad counters.
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}: Draw a card. This ability costs {1} less to activate for each rad counter you have.
//! Set: PIP #151 — Fallout | Scryfall ID: cd7be9d4-f9fa-44a3-9902-7e9e226ccf56 | Oracle ID: f1e03d99-024a-430b-9342-ffd2268bd103
// IMPLEMENTED — optional rad-bearing entry and a reduction fixed at activation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARIPOSA_MILITARY_BASE,
    oracle_id = "f1e03d99-024a-430b-9342-ffd2268bd103",
    scryfall_id = "cd7be9d4-f9fa-44a3-9902-7e9e226ccf56",
    faces = &[face!(
        name = "Mariposa Military Base",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedForPlayerCounters {
            kind: CounterKind::Rad,
            amount: 2
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::draw(1)],
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountersOnYou(CounterKind::Rad),
                each: 1,
            }),
        ),
    ],
);
