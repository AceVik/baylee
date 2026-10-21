//! Reality Twist — {U}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep {1}{U}{U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If tapped for mana, Plains produce {R}, Swamps produce {G}, Mountains produce {W}, and Forests produce {B} instead of any other type.
//! Set: ICE #94 — Ice Age | Scryfall ID: 1b7e955c-3de2-430c-93b9-0b39ccea5420 | Oracle ID: 2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: On a land with multiple matching basic land types, mana_rules::replace applies matching ReplaceTappedMana effects in registration order rather than letting the affected controller choose which replacement applies (CR 616.1).
card!(
    index = index::REALITY_TWIST,
    oracle_id = "2c6f9fdb-ee27-4cd8-9d4d-1ea8b2830f73",
    scryfall_id = "1b7e955c-3de2-430c-93b9-0b39ccea5420",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White,
    ]),
    coverage = Coverage::Partial(
        "on a land with multiple matching basic land types, the engine applies the last registered mana-type replacement instead of letting the controller choose which replacement applies",
    ),
    faces = &[face!(
        name = "Reality Twist",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
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
                    each: mana!("{1}{U}{U}"),
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
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::SWAMP),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Green,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::White,
            }
        ),
        static_ability!(
            Filter::HasSubtype(subtypes::land::MOUNTAIN),
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::White,
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
