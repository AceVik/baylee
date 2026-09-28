//! Huntmaster of the Fells // Ravager of the Fells — {2}{R}{G} — Creature — Human Werewolf // Creature — Werewolf
//! Oracle: Whenever this creature enters or transforms into Huntmaster of the Fells, create a 2/2 green Wolf creature token and you gain 2 life.
//! Oracle: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
//! Oracle: Trample
//! Oracle: Whenever this creature transforms into Ravager of the Fells, it deals 2 damage to target opponent or planeswalker and 2 damage to up to one target creature that player or that planeswalker's controller controls.
//! Oracle: At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
//! Set: INR #241 — Innistrad Remastered | Scryfall ID: b3819a11-2f3e-4304-a1b0-6abf893c89c5 | Oracle ID: 582328cd-660d-47a4-bb23-e91e80b9a907
//! Face: Huntmaster of the Fells — {2}{R}{G} — Creature — Human Werewolf
//! Face: Ravager of the Fells —  — Creature — Werewolf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTMASTER_OF_THE_FELLS,
    oracle_id = "582328cd-660d-47a4-bb23-e91e80b9a907",
    scryfall_id = "b3819a11-2f3e-4304-a1b0-6abf893c89c5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(
            name = "Huntmaster of the Fells",
            mana_cost = mana!("{2}{R}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WEREWOLF],
            power = Some(2),
            toughness = Some(2),
        ),
        face!(
            name = "Ravager of the Fells",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(4),
            toughness = Some(4),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
