//! Albino Troll — {1}{G} — Creature — Troll
//! Oracle: Echo {1}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: {1}{G}: Regenerate this creature.
//! Set: GVL #3 — Duel Decks Anthology: Garruk vs. Liliana | Scryfall ID: 114fefca-8472-4c09-ae61-75d300d6e681 | Oracle ID: 02079e18-ac14-4255-95c4-741eb76c4799
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALBINO_TROLL,
    oracle_id = "02079e18-ac14-4255-95c4-741eb76c4799",
    scryfall_id = "114fefca-8472-4c09-ae61-75d300d6e681",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Albino Troll",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TROLL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{1}{G}"),
        },
        activated!(
            cost!("{1}{G}"),
            &[Effect::regenerate(TargetSpec::ThisObject)],
        ),
    ],
);
