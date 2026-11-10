//! Elvish Scout — {G} — Creature — Elf Scout
//! Oracle: {G}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by it this turn.
//! Set: FEM #68a — Fallen Empires | Scryfall ID: 689cd2ed-be81-4769-a8ec-287946301396 | Oracle ID: d39a492e-a599-4283-bfc1-8e99065a9303
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Attacking]);

card!(
    index = index::ELVISH_SCOUT,
    oracle_id = "d39a492e-a599-4283-bfc1-8e99065a9303",
    scryfall_id = "689cd2ed-be81-4769-a8ec-287946301396",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Scout",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{G}", TapSelf),
        &[
            Effect::UntapTarget,
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        target = Some(TargetSpec::Object(&YOUR_ATTACKING_CREATURE)),
    ),],
);
