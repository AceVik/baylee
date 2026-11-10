//! Cursed Rack — {4} — Artifact
//! Oracle: As this artifact enters, choose an opponent.
//! Oracle: The chosen player's maximum hand size is four.
//! Set: ME1 #155 — Masters Edition | Scryfall ID: 26aa48e6-f23b-4fe1-8cf7-65a430ffba61 | Oracle ID: 4f04603f-8f91-405b-b6ac-d2b66f05e32f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CURSED_RACK,
    oracle_id = "4f04603f-8f91-405b-b6ac-d2b66f05e32f",
    scryfall_id = "26aa48e6-f23b-4fe1-8cf7-65a430ffba61",
    faces = &[face!(
        name = "Cursed Rack",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::ChosenPlayerMaxHandSize(4)
    )],
);
