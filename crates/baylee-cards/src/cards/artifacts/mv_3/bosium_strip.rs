//! Bösium Strip — {3} — Artifact
//! Oracle: {3}, {T}: Until end of turn, you may cast instant and sorcery spells from the top of your graveyard. If a spell cast this way would be put into a graveyard, exile it instead.
//! Set: WTH #147 — Weatherlight | Scryfall ID: 3884bede-df28-42e8-9ac9-ae03118b1985 | Oracle ID: 2fa6f1b1-00af-433f-b8e1-36db99cd9bba
// NOT SUPPORTED: casting instant and sorcery spells from the top of your graveyard until end of turn and exiling them if they would be put into a graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOSIUM_STRIP,
    oracle_id = "2fa6f1b1-00af-433f-b8e1-36db99cd9bba",
    scryfall_id = "3884bede-df28-42e8-9ac9-ae03118b1985",
    coverage = Coverage::Partial(
        "activates for {3}, {T}; casting instant and sorcery spells from the top of your graveyard until end of turn and exiling them if they would be put into a graveyard is not yet modeled",
    ),
    faces = &[face!(
        name = "Bösium Strip",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(cost!("{3}", TapSelf), &[])],
);
