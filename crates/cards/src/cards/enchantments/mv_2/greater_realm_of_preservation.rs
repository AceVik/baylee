//! Greater Realm of Preservation — {1}{W} — Enchantment
//! Oracle: {1}{W}: The next time a black or red source of your choice would deal damage to you this turn, prevent that damage.
//! Set: ME1 #13 — Masters Edition | Scryfall ID: 07cef166-aefe-4586-b30e-4decb127851c | Oracle ID: b03eb0c4-89a4-420d-8a23-1a868a07d9cf
// IMPLEMENTED — {1}{W}: choose a black or red source; the next damage it would deal to you this turn is prevented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREATER_REALM_OF_PRESERVATION,
    oracle_id = "b03eb0c4-89a4-420d-8a23-1a868a07d9cf",
    scryfall_id = "07cef166-aefe-4586-b30e-4decb127851c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Greater Realm of Preservation",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}{W}"),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::HasColor(ColorSet::from_slice(&[Color::Black, Color::Red])),
            combat_only: false,
            all_but: 0,
            gain_life: false
        }]
    ),],
);
