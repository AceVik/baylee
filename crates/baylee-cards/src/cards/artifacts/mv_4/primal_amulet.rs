//! Primal Amulet // Primal Wellspring — {4} — Artifact // Land
//! Oracle: Instant and sorcery spells you cast cost {1} less to cast.
//! Oracle: Whenever you cast an instant or sorcery spell, put a charge counter on this artifact. Then if there are four or more charge counters on it, you may remove those counters and transform it.
//! Oracle: (Transforms from Primal Amulet.)
//! Oracle: {T}: Add one mana of any color. When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose new targets for the copy.
//! Set: XLN #243 — Ixalan | Scryfall ID: d4d379b5-7f56-4a7d-a4ac-131fc3d579c6 | Oracle ID: 8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49
//! Face: Primal Amulet — {4} — Artifact
//! Face: Primal Wellspring —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 874,
    oracle_id = "8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49",
    scryfall_id = "d4d379b5-7f56-4a7d-a4ac-131fc3d579c6",
    faces = &[
        face!(
            name = "Primal Amulet",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(name = "Primal Wellspring", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
