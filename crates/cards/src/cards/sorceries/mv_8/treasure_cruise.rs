//! Treasure Cruise — {7}{U} — Sorcery
//! Oracle: Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)
//! Oracle: Draw three cards.
//! Set: SOC #205 — Secrets of Strixhaven Commander | Scryfall ID: 42c45880-15c7-4259-8066-c04d031d8216 | Oracle ID: 5b6bdf5a-2742-4851-92cd-a857a3852836
// IMPLEMENTED — graveyard cards exiled at cast pay the generic half, then
// draw three.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TREASURE_CRUISE,
    oracle_id = "5b6bdf5a-2742-4851-92cd-a857a3852836",
    scryfall_id = "42c45880-15c7-4259-8066-c04d031d8216",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Treasure Cruise",
        mana_cost = mana!("{7}{U}"),
        types = TypeSet::SORCERY,
        delve = true,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::draw(3)])],
);

// Engine-level coverage: the delve half is played in
// `convoke_tests::delve_makes_a_spell_castable_that_the_pool_alone_could_not_pay`
// and its bound in `the_delve_question_stops_at_the_generic_half_of_the_cost`;
// drawing is covered by the s4 scenario tests.
