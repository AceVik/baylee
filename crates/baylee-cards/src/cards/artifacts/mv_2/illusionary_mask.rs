//! Illusionary Mask — {2} — Artifact
//! Oracle: {X}: You may choose a creature card in your hand whose mana cost could be paid by some amount of, or all of, the mana you spent on {X}. If you do, you may cast that card face down as a 2/2 creature spell without paying its mana cost. If the creature that spell becomes as it resolves has not been turned face up and would assign or deal damage, be dealt damage, or become tapped, instead it's turned face up and assigns or deals damage, is dealt damage, or becomes tapped. Activate only as a sorcery.
//! Set: ME3 #197 — Masters Edition III | Scryfall ID: 937e977f-5e97-4e00-8a5a-42982862b997 | Oracle ID: 05ac866d-0405-4d25-986a-c10fcfc097e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUSIONARY_MASK,
    oracle_id = "05ac866d-0405-4d25-986a-c10fcfc097e6",
    scryfall_id = "937e977f-5e97-4e00-8a5a-42982862b997",
    faces = &[face!(
        name = "Illusionary Mask",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
