//! Glissa Sunseeker — {2}{G}{G} — Legendary Creature — Elf Warrior
//! Oracle: First strike
//! Oracle: {T}: Destroy target artifact if its mana value is equal to the amount of unspent mana you have.
//! Set: MRD #120 — Mirrodin | Scryfall ID: 670c3106-71fc-464e-8c94-81bf7fafc3e6 | Oracle ID: bf4b9330-72df-426f-820c-77e37f7e9130
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLISSA_SUNSEEKER,
    oracle_id = "bf4b9330-72df-426f-820c-77e37f7e9130",
    scryfall_id = "670c3106-71fc-464e-8c94-81bf7fafc3e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FIRST_STRIKE,
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Glissa Sunseeker",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::IfTargetMatches {
            filter: &Filter::CmcEqualsYourUnspentMana,
            then: &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        }],
        target = Some(TargetSpec::Object(&Filter::ARTIFACT)),
    )],
);
