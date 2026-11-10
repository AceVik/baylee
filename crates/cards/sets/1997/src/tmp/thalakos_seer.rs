//! Thalakos Seer — {U}{U} — Creature — Thalakos Wizard
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: When this creature leaves the battlefield, draw a card.
//! Set: TPR #72 — Tempest Remastered | Scryfall ID: 50210dd8-83c6-4e64-a488-f8e7fd85d992 | Oracle ID: 1baec770-0785-4cbc-be18-83572fd8e9c1

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THALAKOS_SEER,
    oracle_id = "1baec770-0785-4cbc-be18-83572fd8e9c1",
    scryfall_id = "50210dd8-83c6-4e64-a488-f8e7fd85d992",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thalakos Seer",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THALAKOS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::LeavesBattlefield(&Filter::This),
        &[Effect::draw(1)],
    )],
);
