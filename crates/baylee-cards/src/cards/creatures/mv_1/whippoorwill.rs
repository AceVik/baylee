//! Whippoorwill — {G} — Creature — Bird
//! Oracle: {G}{G}, {T}: Target creature can't be regenerated this turn. Damage that would be dealt to that creature this turn can't be prevented or dealt instead to another permanent or player. When the creature dies this turn, exile the creature.
//! Set: DRK #91 — The Dark | Scryfall ID: e56146bf-5db0-4bef-83bb-efa5ebec6684 | Oracle ID: 84050a10-e1f1-413e-aa21-5c1f47bb2a64

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIPPOORWILL,
    oracle_id = "84050a10-e1f1-413e-aa21-5c1f47bb2a64",
    scryfall_id = "e56146bf-5db0-4bef-83bb-efa5ebec6684",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Whippoorwill",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{G}{G}", TapSelf),
        &[
            Effect::CantBeRegeneratedThisTurn {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::ModifyTargets {
                modifier: Modifier::DamageToItCantBePreventedOrRedirected,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ExileIfDiesThisTurn {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
