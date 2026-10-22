//! Shallow Grave — {1}{B} — Instant
//! Oracle: Return the top creature card of your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: MIR #141 — Mirage | Scryfall ID: d5c782cc-c951-4c6f-a93f-774ae6c1c214 | Oracle ID: 9843d8a6-bcbf-40c2-8c41-316e97db9c62
// PARTIAL: ReturnFromGraveyardOf lets you choose any creature card in your graveyard rather than automatically selecting the top creature card, and does not grant haste until end of turn or exile it at the beginning of the next end step; returns a creature card from your graveyard to the battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHALLOW_GRAVE,
    oracle_id = "9843d8a6-bcbf-40c2-8c41-316e97db9c62",
    scryfall_id = "d5c782cc-c951-4c6f-a93f-774ae6c1c214",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "ReturnFromGraveyardOf lets you choose any creature card in your graveyard rather than automatically selecting the top creature card, and does not grant haste until end of turn or exile it at the beginning of the next end step; returns a creature card from your graveyard to the battlefield",
    ),
    faces = &[face!(
        name = "Shallow Grave",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnFromGraveyardOf {
        whose: PlayerRel::You,
        filter: &Filter::CREATURE,
    }])],
);
