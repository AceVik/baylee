//! Phyrexian Dreadnought — {1} — Artifact Creature — Phyrexian Dreadnought
//! Oracle: Trample
//! Oracle: When this creature enters, sacrifice it unless you sacrifice any number of creatures with total power 12 or greater.
//! Set: MIR #315 — Mirage | Scryfall ID: 7b8197b9-0cd1-4fa1-9668-d1b5f1759151 | Oracle ID: a7950edb-5670-46fd-a30e-0fe888a5a32b
// PARTIAL: The engine has no multi-creature total-power sacrifice cost for "unless you sacrifice any number of creatures with total power 12 or greater"; this creature has trample and sacrifices itself when its enters-the-battlefield trigger resolves

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_DREADNOUGHT,
    oracle_id = "a7950edb-5670-46fd-a30e-0fe888a5a32b",
    scryfall_id = "7b8197b9-0cd1-4fa1-9668-d1b5f1759151",
    coverage = Coverage::Partial(
        "The engine has no multi-creature total-power sacrifice cost for \"unless you sacrifice any number of creatures with total power 12 or greater\"; this creature has trample and sacrifices itself when its enters-the-battlefield trigger resolves"
    ),
    faces = &[face!(
        name = "Phyrexian Dreadnought",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::DREADNOUGHT
        ],
        power = Some(12),
        toughness = Some(12),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[triggered!(Trigger::ETB, &[Effect::SacrificeSelf])],
);
