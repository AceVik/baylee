//! Sindbad — {1}{U} — Creature — Human
//! Oracle: {T}: Draw a card and reveal it. If it isn't a land card, discard it.
//! Set: TSB #31 — Time Spiral Timeshifted | Scryfall ID: 6a6372ec-1ae2-4806-bb31-78f9ea6259df | Oracle ID: dc81069b-b2cf-44b3-98fc-45bb24b815cb
// IMPLEMENTED — Effect::DrawRevealDiscardUnless over land cards.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SINDBAD,
    oracle_id = "dc81069b-b2cf-44b3-98fc-45bb24b815cb",
    scryfall_id = "6a6372ec-1ae2-4806-bb31-78f9ea6259df",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sindbad",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::DrawRevealDiscardUnless {
            keep: &Filter::LAND
        }]
    )],
);
