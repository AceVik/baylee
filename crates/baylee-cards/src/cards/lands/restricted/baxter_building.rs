//! Baxter Building — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Add four mana in any combination of colors.
//! Oracle: {4}, {T}: Draw a card. Activate only if you control a creature with toughness 4 or greater.
//! Set: MSH #261 — Marvel Super Heroes | Scryfall ID: 1abc652f-8e9d-4df7-bc4e-d8b515a40fec | Oracle ID: 71bc69a5-7cec-4abd-b97d-13f8e1f9afac
// IMPLEMENTED — {C}, the {4}, {T} four mana in any combination, and the
// {4}, {T} draw behind the toughness gate.

use baylee_cards_dsl::prelude::*;

/// Toughness and not power: the two are one word apart on the card and a
/// whole board apart in play, and this is the only land in the pool that
/// gates on the defensive half.
static TOUGH_CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::ToughnessAtLeast(4)]);

card!(
    index = index::BAXTER_BUILDING,
    oracle_id = "71bc69a5-7cec-4abd-b97d-13f8e1f9afac",
    scryfall_id = "1abc652f-8e9d-4df7-bc4e-d8b515a40fec",
    faces = &[face!(name = "Baxter Building", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{4}", TapSelf),
            &[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(4))],
        ),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::draw(1)],
            condition = Some(Condition::ControlCount(&TOUGH_CREATURE_YOU_CONTROL, 1)),
        ),
    ],
);
