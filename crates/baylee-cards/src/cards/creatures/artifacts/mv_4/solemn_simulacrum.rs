//! Solemn Simulacrum — {4} — Artifact Creature — Golem
//! Oracle: When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: When this creature dies, you may draw a card.
//! Set: MSC #215 — Marvel Super Heroes Commander | Scryfall ID: daafd816-f7c1-4630-9e5c-a1e5db570a35 | Oracle ID: 00c0543c-2a1f-4425-8283-4062d74a1637
// IMPLEMENTED — ETB ramp + dies cantrip.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static BASIC_LAND: Filter = Filter::And(&[Filter::HasSupertype(SupertypeSet::BASIC), Filter::LAND]);

card! {
    index: 151,
    oracle_id: "00c0543c-2a1f-4425-8283-4062d74a1637",
    scryfall_id: "daafd816-f7c1-4630-9e5c-a1e5db570a35",
    faces: &[face! {
        name: "Solemn Simulacrum",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::GOLEM],
        power: Some(2),
        toughness: Some(2),
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::SearchLibrary {
                filter: &BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: true,
            }]),
        triggered!(Trigger::Dies(&Filter::This), &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }]),
    ],
}
