//! Naked Singularity — {5} — Artifact
//! Oracle: Cumulative upkeep {3} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If tapped for mana, Plains produce {R}, Islands produce {G}, Swamps produce {W}, Mountains produce {U}, and Forests produce {B} instead of any other type.
//! Set: ME4 #216 — Masters Edition IV | Scryfall ID: 5edddcbb-0e01-4473-b7a8-1e59cf7ee506 | Oracle ID: fb5ec55d-4a35-432a-be6e-c295f1b2e603
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: On a land with multiple basic land types, mana_rules::replace applies matching ReplaceTappedMana effects in registration order rather than letting the affected controller choose which replacement applies (CR 616.1).
card!(
    index = index::NAKED_SINGULARITY,
    oracle_id = "fb5ec55d-4a35-432a-be6e-c295f1b2e603",
    scryfall_id = "5edddcbb-0e01-4473-b7a8-1e59cf7ee506",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White,
    ]),
    coverage = Coverage::Partial(
        "on a land with multiple basic land types, the engine applies the last registered mana-type replacement instead of letting the controller choose which replacement applies",
    ),
    faces = &[face!(
        name = "Naked Singularity",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{3}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::PLAINS),
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Red,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::PLAINS),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Red,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::PLAINS),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::Red,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::PLAINS),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::Red,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::PLAINS),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Red,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::ISLAND),
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::ISLAND),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::ISLAND),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::ISLAND),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::ISLAND),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Blue,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::Blue,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Blue,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::Blue,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Blue,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::FOREST),
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Black,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::FOREST),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Black,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::FOREST),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Black,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::FOREST),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::Black,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::FOREST),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Black,
            }
        ),
    ],
);
