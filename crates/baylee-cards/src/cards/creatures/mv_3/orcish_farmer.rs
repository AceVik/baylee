//! Orcish Farmer — {1}{R}{R} — Creature — Orc
//! Oracle: {T}: Target land becomes a Swamp until its controller's next untap step.
//! Set: ME2 #141 — Masters Edition II | Scryfall ID: 2563a5b5-ea7a-4400-9e40-292623db2f96 | Oracle ID: c3039d19-8c98-4953-8943-9922b6ab45ef
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// IMPLEMENTED — {T} makes the target land a Swamp for Duration::UntilAffectedUntapSteps(1): through the land's controller's next untap step, whoever controls the land.

card!(
    index = index::ORCISH_FARMER,
    oracle_id = "c3039d19-8c98-4953-8943-9922b6ab45ef",
    scryfall_id = "2563a5b5-ea7a-4400-9e40-292623db2f96",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orcish Farmer",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetLandType(subtypes::land::SWAMP),
            Duration::UntilAffectedUntapSteps(1),
        )],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
