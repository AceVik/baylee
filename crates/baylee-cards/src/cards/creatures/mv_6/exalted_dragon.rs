//! Exalted Dragon — {4}{W}{W} — Creature — Dragon
//! Oracle: Flying
//! Oracle: This creature can't attack unless you sacrifice a land. (This cost is paid as attackers are declared.)
//! Set: TPR #12 — Tempest Remastered | Scryfall ID: 0cf0866d-4544-4074-bf55-fcac0799c764 | Oracle ID: c901805e-c196-4b9d-8d5a-e17367fb37bc
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXALTED_DRAGON,
    oracle_id = "c901805e-c196-4b9d-8d5a-e17367fb37bc",
    scryfall_id = "0cf0866d-4544-4074-bf55-fcac0799c764",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Exalted Dragon",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantAttackUnlessYouSacrifice {
            count: 1,
            filter: &Filter::LAND,
        }
    )],
);
