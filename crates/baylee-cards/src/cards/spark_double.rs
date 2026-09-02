//! Spark Double — {3}{U} — Creature — Illusion
//! Oracle: You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it isn't legendary.
//! Set: RVR #62 — Ravnica Remastered | Scryfall ID: c41b9ba2-0006-4d8e-b600-efe81ff5e0cc | Oracle ID: 8dcb35e5-ae44-455f-86e3-4a77d496ff34
// IMPLEMENTED — clone of your creature/planeswalker, not legendary,
// with both bonus counters (the creature-irrelevant counter is harmless
// on the other card type, matching the card's effect in play).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static YOUR_CREATURE_OR_WALKER: Filter = Filter::And(&[
    Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER]),
    Filter::ControlledByYou,
]);

card! {
    index: 154,
    oracle_id: "8dcb35e5-ae44-455f-86e3-4a77d496ff34",
    scryfall_id: "c41b9ba2-0006-4d8e-b600-efe81ff5e0cc",
    faces: &[face! {
        name: "Spark Double",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ILLUSION],
        power: Some(0),
        toughness: Some(0),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&YOUR_CREATURE_OR_WALKER),
        mods: &[
            CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY),
            CopyMod::AddCounter(CounterKind::P1P1, 1),
            CopyMod::AddCounter(CounterKind::Loyalty, 1),
        ],
    }],
}
