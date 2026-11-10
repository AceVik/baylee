//! Stone Calendar — {5} — Artifact
//! Oracle: Spells you cast cost {1} less to cast.
//! Set: ME1 #167 — Masters Edition | Scryfall ID: e682d19b-7936-4942-b42c-c836d6741a45 | Oracle ID: 301a59c1-e372-4c31-a2a8-88c5ba7d822a

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STONE_CALENDAR,
    oracle_id = "301a59c1-e372-4c31-a2a8-88c5ba7d822a",
    scryfall_id = "e682d19b-7936-4942-b42c-c836d6741a45",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Stone Calendar",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::SpellsCostLess(1),
    )],
);
