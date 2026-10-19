//! Giant Badger — {1}{G}{G} — Creature — Badger
//! Oracle: Whenever this creature blocks, it gets +2/+2 until end of turn.
//! Set: 8ED #253 — Eighth Edition | Scryfall ID: 10e9dd66-8580-4c96-9ab9-cc392dc6ee74 | Oracle ID: a4109138-5d69-4510-a228-7c1f88c0c8c0

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_BADGER,
    oracle_id = "a4109138-5d69-4510-a228-7c1f88c0c8c0",
    scryfall_id = "10e9dd66-8580-4c96-9ab9-cc392dc6ee74",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Giant Badger",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BADGER],
        power = Some(2),
        toughness = Some(2),
        abilities = &[triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(2, 2),
                duration: Duration::UntilEndOfTurn,
            }],
        )],
    ),],
);
