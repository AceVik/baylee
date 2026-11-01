//! Mounted Archers — {3}{W} — Creature — Human Soldier Archer
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: {W}: This creature can block an additional creature this turn.
//! Set: TPR #19 — Tempest Remastered | Scryfall ID: 801e7b20-9359-4157-87a5-6354efea929a | Oracle ID: 22517690-3ec1-49c9-951f-93eb3114423a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOUNTED_ARCHERS,
    oracle_id = "22517690-3ec1-49c9-951f-93eb3114423a",
    scryfall_id = "801e7b20-9359-4157-87a5-6354efea929a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mounted Archers",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::ARCHER,
        ],
        power = Some(2),
        toughness = Some(3),
        keywords = KeywordSet::REACH,
    ),],
    abilities = &[activated!(
        cost!("{W}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::CanBlockAdditional(1),
            Duration::UntilEndOfTurn,
        )],
    )],
);
