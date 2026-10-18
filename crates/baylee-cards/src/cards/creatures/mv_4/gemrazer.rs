//! Gemrazer — {3}{G} — Creature — Beast
//! Oracle: Mutate {1}{G}{G} (If you cast this spell for its mutate cost, put it over or under target non-Human creature you own. They mutate into the creature on top plus all abilities from under it.)
//! Oracle: Reach, trample
//! Oracle: Whenever this creature mutates, destroy target artifact or enchantment an opponent controls.
//! Set: IKO #155 — Ikoria: Lair of Behemoths | Scryfall ID: 0095245c-a30e-4e2a-88c9-632c678e9f03 | Oracle ID: 3dfb0c0a-b68f-43b9-8475-28d0192fc4ed

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMRAZER,
    oracle_id = "3dfb0c0a-b68f-43b9-8475-28d0192fc4ed",
    scryfall_id = "0095245c-a30e-4e2a-88c9-632c678e9f03",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::REACH.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gemrazer",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
        mutate = Some(mana!("{1}{G}{G}")),
    ),],
    abilities = &[triggered!(
        Trigger::Mutates,
        &[Effect::destroy(TargetSpec::Object(&Filter::And(&[
            Filter::ARTIFACT_OR_ENCHANTMENT,
            Filter::ControlledByOpponent,
        ])))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::ARTIFACT_OR_ENCHANTMENT,
            Filter::ControlledByOpponent,
        ]))))
    )],
);
