//! Brushwagg — {1}{G}{G} — Creature — Brushwagg
//! Oracle: Whenever this creature blocks or becomes blocked, it gets -2/+2 until end of turn.
//! Set: MIR #208 — Mirage | Scryfall ID: 6c20edc3-5ad0-42c1-a5ec-3e680fb03297 | Oracle ID: 33a981ac-878d-4257-936f-55b4e6543c12
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRUSHWAGG,
    oracle_id = "33a981ac-878d-4257-936f-55b4e6543c12",
    scryfall_id = "6c20edc3-5ad0-42c1-a5ec-3e680fb03297",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brushwagg",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BRUSHWAGG],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(-2, 2),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
