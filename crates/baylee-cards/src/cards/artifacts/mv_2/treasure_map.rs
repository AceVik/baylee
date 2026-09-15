//! Treasure Map // Treasure Cove — {2} — Artifact // Land
//! Oracle: {1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: (Transforms from Treasure Map.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a Treasure: Draw a card.
//! Set: LCI #267 — The Lost Caverns of Ixalan | Scryfall ID: a924fe1e-a85e-4e14-88d2-ac55130638ab | Oracle ID: 0b55eac6-a745-4bf4-8926-5ce83bc38d7d
//! Face: Treasure Map — {2} — Artifact
//! Face: Treasure Cove —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 17329,
    oracle_id = "0b55eac6-a745-4bf4-8926-5ce83bc38d7d",
    scryfall_id = "a924fe1e-a85e-4e14-88d2-ac55130638ab",
    faces = &[
        face!(
            name = "Treasure Map",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(name = "Treasure Cove", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
