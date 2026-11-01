//! Fumarole — {3}{B}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, pay 3 life.
//! Oracle: Destroy target creature and target land.
//! Set: ME2 #194 — Masters Edition II | Scryfall ID: 66ce0f61-3c5b-4490-9e9b-8e30432a346a | Oracle ID: a9e33138-e1c0-4b69-b868-1ce8493c073d
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FUMAROLE,
    oracle_id = "a9e33138-e1c0-4b69-b868-1ce8493c073d",
    scryfall_id = "66ce0f61-3c5b-4490-9e9b-8e30432a346a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fumarole",
        mana_cost = mana!("{3}{B}{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::PayLife(3)],
    ),],
    abilities = &[spell!(
        &[Effect::DestroyTargetSlots {
            targets: &[TargetSlot::First, TargetSlot::Second],
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    )],
);
