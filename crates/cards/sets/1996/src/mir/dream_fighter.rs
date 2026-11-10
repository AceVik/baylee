//! Dream Fighter — {2}{U} — Creature — Human Soldier
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, this creature and that creature phase out. (While they're phased out, they're treated as though they don't exist. Each one phases in before its controller untaps during their next untap step.)
//! Set: MIR #63 — Mirage | Scryfall ID: aec06bc9-553c-4e01-8b43-a4eeaa511b4d | Oracle ID: ea8d1c55-e045-4a3f-92f0-4ecd86362b97
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "This creature and that creature": the source and the creature the
/// trigger's event named (the one it blocks or is blocked by).
static THIS_AND_THAT_CREATURE: Filter = Filter::Or(&[Filter::This, Filter::Reference]);

card!(
    index = index::DREAM_FIGHTER,
    oracle_id = "ea8d1c55-e045-4a3f-92f0-4ecd86362b97",
    scryfall_id = "aec06bc9-553c-4e01-8b43-a4eeaa511b4d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dream Fighter",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::PhaseOutFilter(&THIS_AND_THAT_CREATURE)],
    )],
);
