//! Mortuary — {3}{B} — Enchantment
//! Oracle: Whenever a creature is put into your graveyard from the battlefield, put that card on top of your library.
//! Set: STH #66 — Stronghold | Scryfall ID: 860f46ea-6fd0-456d-971d-8d1eccc7fa60 | Oracle ID: 8bd2af55-5a48-4300-a893-ae6cd548db24
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static YOUR_OWNED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::OwnedByYou]);

card!(
    index = index::MORTUARY,
    oracle_id = "8bd2af55-5a48-4300-a893-ae6cd548db24",
    scryfall_id = "860f46ea-6fd0-456d-971d-8d1eccc7fa60",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mortuary",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&YOUR_OWNED_CREATURE),
        &[Effect::GraveyardToTop {
            target: TargetSpec::EventObject,
        }],
    ),],
);
