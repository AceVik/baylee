//! HELIOS One — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: You get {E} (an energy counter).
//! Oracle: {3}, {T}, Pay X {E}, Sacrifice this land: Destroy target nonland permanent with mana value X. Activate only as a sorcery.
//! Set: PIP #149 — Fallout | Scryfall ID: 4102d28e-437b-440b-bf9b-8b4f6fb85a6c | Oracle ID: cfb1a656-0bf1-484d-b099-33087914250b

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELIOS_ONE,
    oracle_id = "cfb1a656-0bf1-484d-b099-33087914250b",
    scryfall_id = "4102d28e-437b-440b-bf9b-8b4f6fb85a6c",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "HELIOS One", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::Energy {
                who: PlayerRel::You,
                amount: Amount::Fixed(1)
            }]
        ),
        activated!(
            cost!("{3}", TapSelf, PayEnergyX, SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(&Filter::NONLAND))],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::NONLAND,
                Filter::CmcExactlyX
            ]))),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
