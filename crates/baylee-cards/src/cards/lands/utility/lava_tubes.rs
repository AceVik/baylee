//! Lava Tubes — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {B} or {R}. Put a depletion counter on this land.
//! Set: ICE #358 — Ice Age | Scryfall ID: 5e7c2cf6-f36f-451b-bba5-19a82c659c4c | Oracle ID: 2ffb2647-60bb-4916-b919-cbcf18e5e424
// PARTIAL — the {T} ability is built (a {B}/{R} choice plus the depletion counter);
// the untap lock and the upkeep removal have no construct in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAVA_TUBES,
    oracle_id = "2ffb2647-60bb-4916-b919-cbcf18e5e424",
    scryfall_id = "5e7c2cf6-f36f-451b-bba5-19a82c659c4c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Lava Tubes", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no static ability carries a condition, and no effect removes a counter",
    ),
    abilities = &[
        // NOT SUPPORTED: "This land doesn't untap during your untap step if it has a depletion counter on it." — Modifier::DoesNotUntap takes no condition, and Condition::CountersOnSelf is read only on an activation or an intervening `if`.
        // NOT SUPPORTED: "At the beginning of your upkeep, remove a depletion counter from this land." — Effect has no remove-a-counter variant; a counter leaves only as CostPart::RemoveCounterSelf, which is a cost and not an effect.
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Black, ManaColor::Red]),
            Effect::AddCounter {
                kind: counters::DEPLETION,
                amount: Amount::Fixed(1),
            },
        ]),
    ],
);
