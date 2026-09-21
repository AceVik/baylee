//! Karn's Bastion — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Proliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)
//! Set: EOC #163 — Edge of Eternities Commander | Scryfall ID: 22017ec2-3552-4865-af76-dba042b141f5 | Oracle ID: 9fb8cd81-403a-4988-8f1c-b8eccf8abd9c
// PARTIAL — the {T} mana ability is built; the proliferate activation is
// dropped, because the DSL has no effect that reads the counters an object
// already wears.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KARN_S_BASTION,
    oracle_id = "9fb8cd81-403a-4988-8f1c-b8eccf8abd9c",
    scryfall_id = "22017ec2-3552-4865-af76-dba042b141f5",
    faces = &[face!(name = "Karn's Bastion", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "proliferate: no effect gives an object another counter of each kind already on it, and none asks the player to choose any number of permanents and/or players"
    ),
    // NOT SUPPORTED: "{4}, {T}: Proliferate. (Choose any number of
    // permanents and/or players, then give each another counter of each kind
    // already there.)" — `Effect::AddCounter` names one target and one
    // counter kind and `Effect::AddCounterFilter` one kind over a filter;
    // neither reads back the kinds an object already wears, and nothing in
    // `Effect` puts a chosen set of permanents *and* players in front of a
    // player.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
