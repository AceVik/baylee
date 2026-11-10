//! Balamb Garden, SeeD Academy // Balamb Garden, Airborne — (no cost) — Land — Town // Legendary Artifact — Vehicle
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Oracle: {5}{G}{U}, {T}: Transform this land. This ability costs {1} less to activate for each other Town you control.
//! Oracle: Flying
//! Oracle: Whenever Balamb Garden attacks, draw a card.
//! Oracle: Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)
//! Set: FIN #272 — Final Fantasy | Scryfall ID: 001e9f20-5b15-41cb-bf82-46172decc235 | Oracle ID: 8b84fec5-617c-4088-8250-2ba1f1f9479a
//! Face: Balamb Garden, SeeD Academy —  — Land — Town
//! Face: Balamb Garden, Airborne —  — Legendary Artifact — Vehicle

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_ABILITIES: &[AbilityDef] = &[
    triggered!(Trigger::Attacks(&Filter::This), &[Effect::draw(1)]),
    crew!(1),
];

card!(
    index = index::BALAMB_GARDEN_SEE_D_ACADEMY,
    oracle_id = "8b84fec5-617c-4088-8250-2ba1f1f9479a",
    scryfall_id = "001e9f20-5b15-41cb-bf82-46172decc235",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Balamb Garden, SeeD Academy",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Balamb Garden, Airborne",
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::artifact::VEHICLE],
            power = Some(5),
            toughness = Some(4),
            keywords = KeywordSet::FLYING,
            abilities = BACK_ABILITIES,
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::Blue])]),
        activated!(
            cost!("{5}{G}{U}", TapSelf),
            &[Effect::TransformSource],
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::Another,
                        Filter::ControlledByYou,
                        Filter::HasSubtype(subtypes::land::TOWN)
                    ]),
                    zone: ZoneSel::Battlefield
                },
                each: 1
            }),
        ),
    ],
);
