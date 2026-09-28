//! Phyrexian Fleshgorger — {7} — Artifact Creature — Phyrexian Wurm
//! Oracle: Prototype {1}{B}{B} — 3/3 (You may cast this spell with different mana cost, color, and size. It keeps its abilities and types.)
//! Oracle: Menace, lifelink
//! Oracle: Ward—Pay life equal to this creature's power.
//! Set: BRO #121 — The Brothers' War | Scryfall ID: 62d37423-3445-412a-9abd-0480da404637 | Oracle ID: d3a5a830-cd14-49da-9412-c50049c74c92
// PARTIAL — the {7} body, menace, lifelink and ward life payment work;
// prototype remains unsupported. Ward is tested in engine::ward_tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Prototype {1}{B}{B} — 3/3 (You may cast this spell with
// different mana cost, color, and size.)" — no DSL variant casts a spell for
// an alternative cost that also changes its color and its power/toughness.
// `AlternativeCost` swaps the mana paid and nothing else; `SpellMode`'s
// `cost_override` belongs to a modal spell and carries no characteristics
// either.

card!(
    index = index::PHYREXIAN_FLESHGORGER,
    oracle_id = "d3a5a830-cd14-49da-9412-c50049c74c92",
    scryfall_id = "62d37423-3445-412a-9abd-0480da404637",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Phyrexian Fleshgorger",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::WURM],
        power = Some(7),
        toughness = Some(5),
        keywords = KeywordSet::MENACE.union(KeywordSet::LIFELINK),
    ),],
    abilities = &[triggered!(
        Trigger::Ward,
        &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::ControllerOfTarget,
            life: Amount::SourcePower,
            effect: &Effect::CounterTargetSpellOrAbility,
        }],
    )],
    coverage = Coverage::Partial(
        "prototype (an alternative cast with a different mana cost, color and size)"
    ),
);
