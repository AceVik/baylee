//! Commander Greven il-Vec — {3}{B}{B}{B} — Legendary Creature — Phyrexian Human Warrior
//! Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: When Commander Greven il-Vec enters, sacrifice a creature.
//! Set: TPR #88 — Tempest Remastered | Scryfall ID: 8a8ddd65-c482-426c-b4cd-9ad0206942ae | Oracle ID: 08fa1bee-cedf-4a29-9b02-cbdcc0926f8f
// IMPLEMENTED — Fear and enters-the-battlefield trigger sacrificing a creature you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COMMANDER_GREVEN_IL_VEC,
    oracle_id = "08fa1bee-cedf-4a29-9b02-cbdcc0926f8f",
    scryfall_id = "8a8ddd65-c482-426c-b4cd-9ad0206942ae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FEAR,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Commander Greven il-Vec",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::HUMAN,
            subtypes::creature::WARRIOR
        ],
        power = Some(7),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        }]
    )],
);
