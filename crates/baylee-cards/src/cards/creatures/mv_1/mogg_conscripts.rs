//! Mogg Conscripts — {R} — Creature — Goblin
//! Oracle: This creature can't attack unless you've cast a creature spell this turn.
//! Set: TPR #143 — Tempest Remastered | Scryfall ID: c67c26d7-f752-40a5-b79e-1f4c531cbef5 | Oracle ID: b015686d-07e1-46c7-8e51-2bec646860d6
// PARTIAL: can't attack unless you've cast a spell this turn; checking specifically for a creature spell cast this turn is not in Condition.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_CONSCRIPTS,
    oracle_id = "b015686d-07e1-46c7-8e51-2bec646860d6",
    scryfall_id = "c67c26d7-f752-40a5-b79e-1f4c531cbef5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "can't attack unless you've cast a spell this turn; checking specifically for a creature spell cast this turn is not in Condition",
    ),
    faces = &[face!(
        name = "Mogg Conscripts",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantAttack,
        condition = Some(Condition::YouCastNoSpellThisTurn),
    )],
);
