//! Ghazbán Ogre — {G} — Creature — Ogre
//! Oracle: At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.
//! Set: ME1 #120 — Masters Edition | Scryfall ID: 1e14cf3a-3c5a-4c22-88d1-1b19660b2e2a | Oracle ID: d361bdd4-afb8-493d-9091-ebe22f215834
// IMPLEMENTED — at your upkeep, if one player has the most life
// (Condition::APlayerHasMostLife, CR 603.4), that player gains control of the
// Ogre (PlayerRel::MostLife).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GHAZBAN_OGRE,
    oracle_id = "d361bdd4-afb8-493d-9091-ebe22f215834",
    scryfall_id = "1e14cf3a-3c5a-4c22-88d1-1b19660b2e2a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ghazbán Ogre",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::ChangeController {
            new_controller: PlayerRel::MostLife,
        }],
        condition = Some(Condition::APlayerHasMostLife),
    )],
);
