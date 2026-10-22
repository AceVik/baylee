//! Tariff — {1}{W} — Sorcery
//! Oracle: Each player sacrifices the creature they control with the greatest mana value unless they pay that creature's mana cost. If two or more creatures a player controls are tied for greatest, that player chooses one.
//! Set: 6ED #47 — Classic Sixth Edition | Scryfall ID: 61094858-b8a8-425f-9c96-fac4fc6ecae8 | Oracle ID: e96b0560-1d91-48df-b42f-3b7d9c6d94bd
// PARTIAL — Each player sacrifices a creature they control; restricting the choice to the creature with the greatest mana value and allowing each player to pay that creature's mana cost instead is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TARIFF,
    oracle_id = "e96b0560-1d91-48df-b42f-3b7d9c6d94bd",
    scryfall_id = "61094858-b8a8-425f-9c96-fac4fc6ecae8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Each player sacrifices a creature they control; restricting the choice to the creature with the greatest mana value and allowing each player to pay that creature's mana cost instead is not yet modeled.",
    ),
    faces = &[face!(
        name = "Tariff",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::SacrificeFilter {
        who: PlayerRel::EachPlayer,
        filter: &Filter::CREATURE,
    }]),],
);
