//! Thalakos Scout — {2}{U} — Creature — Thalakos Soldier Scout
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Discard a card: Return this creature to its owner's hand.
//! Set: TPR #71 — Tempest Remastered | Scryfall ID: 1594d0e2-0d98-4cd5-bcd2-cddbe7bc6a78 | Oracle ID: c8877da5-7008-4b47-955e-d205220c91e6
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THALAKOS_SCOUT,
    oracle_id = "c8877da5-7008-4b47-955e-d205220c91e6",
    scryfall_id = "1594d0e2-0d98-4cd5-bcd2-cddbe7bc6a78",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thalakos Scout",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::THALAKOS,
            subtypes::creature::SOLDIER,
            subtypes::creature::SCOUT
        ],
        power = Some(2),
        toughness = Some(1),
        keywords = KeywordSet::SHADOW,
    ),],
    abilities = &[activated!(
        cost!(Discard(&Filter::Any)),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
