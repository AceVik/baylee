//! Mist Dragon — {4}{U}{U} — Creature — Dragon
//! Oracle: {0}: This creature gains flying. (This effect lasts indefinitely.)
//! Oracle: {0}: This creature loses flying. (This effect lasts indefinitely.)
//! Oracle: {3}{U}{U}: This creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: MIR #79 — Mirage | Scryfall ID: 3b9b4be4-74c8-4fe5-a5e4-de57c11e8ec1 | Oracle ID: 448f5c55-7334-4fa4-9d90-0e57bae19444
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIST_DRAGON,
    oracle_id = "448f5c55-7334-4fa4-9d90-0e57bae19444",
    scryfall_id = "3b9b4be4-74c8-4fe5-a5e4-de57c11e8ec1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mist Dragon",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        activated!(
            Cost::FREE,
            &[Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                duration: Duration::Indefinitely,
            }],
        ),
        activated!(
            Cost::FREE,
            &[Effect::ModifySource {
                modifier: Modifier::RemoveKeyword(KeywordSet::FLYING),
                duration: Duration::Indefinitely,
            }],
        ),
        activated!(cost!("{3}{U}{U}"), &[Effect::PhaseOut { target: None }]),
    ],
);
