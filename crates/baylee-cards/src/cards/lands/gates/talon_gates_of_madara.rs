//! Talon Gates of Madara — (no cost) — Land — Gate
//! Oracle: When this land enters, up to one target creature phases out.
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}: Put this card from your hand onto the battlefield.
//! Set: M3C #82 — Modern Horizons 3 Commander | Scryfall ID: de92facf-762b-4a23-8d5e-bb673b0500c0 | Oracle ID: 8c45bf9d-a017-43bf-9e32-67810a8a217b
// PARTIAL — ETB phase out up to one target creature, {T}: Add {C}, and {1}, {T}: Add one mana of any color are built; {4} activation from hand is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TALON_GATES_OF_MADARA,
    oracle_id = "8c45bf9d-a017-43bf-9e32-67810a8a217b",
    scryfall_id = "de92facf-762b-4a23-8d5e-bb673b0500c0",
    faces = &[face!(
        name = "Talon Gates of Madara",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE],
    ),],
    coverage = Coverage::Partial(
        "putting the card from hand onto the battlefield via an activated ability is not expressible in the DSL (no Effect variant moves source from hand to battlefield)"
    ),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PhaseOut {
                target: Some(TargetSpec::Object(&Filter::CREATURE)),
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "{4}: Put this card from your hand onto the battlefield."
    ],
);
