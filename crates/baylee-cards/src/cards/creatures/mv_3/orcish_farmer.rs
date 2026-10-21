//! Orcish Farmer — {1}{R}{R} — Creature — Orc
//! Oracle: {T}: Target land becomes a Swamp until its controller's next untap step.
//! Set: ME2 #141 — Masters Edition II | Scryfall ID: 2563a5b5-ea7a-4400-9e40-292623db2f96 | Oracle ID: c3039d19-8c98-4953-8943-9922b6ab45ef
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Duration::UntilAffectedUntapSteps is only decremented for untap-suppression effects during the untap step, not for general continuous effects on a single target, so the land-type change lasts until end of turn instead of until its controller's next untap step.
card!(
    index = index::ORCISH_FARMER,
    oracle_id = "c3039d19-8c98-4953-8943-9922b6ab45ef",
    scryfall_id = "2563a5b5-ea7a-4400-9e40-292623db2f96",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "the target land becomes a Swamp until end of turn rather than until its controller's next untap step",
    ),
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
            Duration::UntilEndOfTurn,
        )],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
