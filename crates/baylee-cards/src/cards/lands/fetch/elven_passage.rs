//! Elven Passage — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. You may behold an Elf. If you do, untap that land. (To behold an Elf, choose an Elf you control or reveal an Elf card from your hand.)
//! Set: HOB #181 — The Hobbit | Scryfall ID: dd1fd2ab-2565-4798-a832-fc849df82f74 | Oracle ID: 97a2cd39-6b54-496b-b3ac-dab9dfed7edc
// PARTIAL — the fetch half of the activated ability is built: {T}, pay 1 life,
// sacrifice this land, find a basic land and put it onto the battlefield
// tapped.
// NOT SUPPORTED: "You may behold an Elf. If you do, untap that land." — the DSL has no behold action and no way to untap the card this search found.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELVEN_PASSAGE,
    oracle_id = "97a2cd39-6b54-496b-b3ac-dab9dfed7edc",
    scryfall_id = "dd1fd2ab-2565-4798-a832-fc849df82f74",
    faces = &[face!(name = "Elven Passage", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the \"You may behold an Elf. If you do, untap that land.\" clause — no behold action, and nothing untaps the land the search found"
    ),
    abilities = &[activated!(
        cost!(TapSelf, PayLife(1), SacrificeSelf),
        &[Effect::SearchLibrary {
            filter: &Filter::BASIC_LAND,
            finds: &[Find::BATTLEFIELD_TAPPED],
            optional: false,
        }]
    )],
);
