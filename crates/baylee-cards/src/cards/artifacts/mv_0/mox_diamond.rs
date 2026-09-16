//! Mox Diamond — {0} — Artifact
//! Oracle: If this artifact would enter, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add one mana of any color.
//! Set: TPR #228 — Tempest Remastered | Scryfall ID: bf9fecfd-d122-422f-bd0a-5bf69b434dfe | Oracle ID: f3c5978a-70fa-431f-933b-b954bd0db0ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOX_DIAMOND,
    oracle_id = "f3c5978a-70fa-431f-933b-b954bd0db0ea",
    scryfall_id = "bf9fecfd-d122-422f-bd0a-5bf69b434dfe",
    faces = &[face!(
        name = "Mox Diamond",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
