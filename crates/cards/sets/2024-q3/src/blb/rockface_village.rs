//! Rockface Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {R}. Spend this mana only to cast a creature spell.
//! Oracle: {R}, {T}: Target Lizard, Mouse, Otter, or Raccoon you control gets +1/+0 and gains haste until end of turn. Activate only as a sorcery.
//! Set: BLB #259 — Bloomburrow | Scryfall ID: 62799d24-39a6-4e66-8ac3-7cafa99e6e6d | Oracle ID: 7e103748-3f76-42ce-a063-d0256b2dce2b
// IMPLEMENTED — {C}; {R} restricted to creature spells; and a sorcery-speed
// {R},{T} pump granting +1/+0 and haste to a Lizard, Mouse, Otter or
// Raccoon you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::ROCKFACE_VILLAGE,
    oracle_id = "7e103748-3f76-42ce-a063-d0256b2dce2b",
    scryfall_id = "62799d24-39a6-4e66-8ac3-7cafa99e6e6d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Rockface Village", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::Red, 1).restricted(&Filter::CREATURE, SpendRider::None)
        ]),
        activated!(
            cost!("{R}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::Or(&[
                    Filter::HasSubtype(creature::LIZARD),
                    Filter::HasSubtype(creature::MOUSE),
                    Filter::HasSubtype(creature::OTTER),
                    Filter::HasSubtype(creature::RACCOON),
                ]),
                Filter::ControlledByYou,
            ]))),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
