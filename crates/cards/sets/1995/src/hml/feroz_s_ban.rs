//! Feroz's Ban — {6} — Artifact
//! Oracle: Creature spells cost {2} more to cast.
//! Set: 7ED #295 — Seventh Edition | Scryfall ID: 188b3863-fa7d-4fbb-b061-97580d394017 | Oracle ID: 445dd350-5165-4516-8669-9233f5558605
// IMPLEMENTED — creature spells cost {2} more to cast.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FEROZ_S_BAN,
    oracle_id = "445dd350-5165-4516-8669-9233f5558605",
    scryfall_id = "188b3863-fa7d-4fbb-b061-97580d394017",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Feroz's Ban",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::SpellsCostMore(2)
    )],
);
