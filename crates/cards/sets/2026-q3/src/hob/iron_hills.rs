//! Iron Hills — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Oracle: {2}{R}{W}, {T}, Sacrifice this land: Put two +1/+1 counters on target Dwarf you control. Activate only as a sorcery.
//! Set: HOB #185 — The Hobbit | Scryfall ID: 78045c43-5cbe-48ff-837d-e7c6baac2937 | Oracle ID: a71e8d07-1a49-47a0-834e-de87d750a200
// IMPLEMENTED — enters tapped, {T} for {R} or {W}, and {2}{R}{W}, {T}, sac puts two +1/+1 counters on target Dwarf you control at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::IRON_HILLS,
    oracle_id = "a71e8d07-1a49-47a0-834e-de87d750a200",
    scryfall_id = "78045c43-5cbe-48ff-837d-e7c6baac2937",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Iron Hills",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::White])]),
        activated!(
            cost!("{2}{R}{W}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(
                &f!(your Filter::HasSubtype(creature::DWARF))
            )),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
