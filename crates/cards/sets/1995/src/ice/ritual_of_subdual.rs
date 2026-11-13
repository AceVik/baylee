//! Ritual of Subdual — {4}{G}{G} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If a land is tapped for mana, it produces colorless mana instead of any other type.
//! Set: ME2 #174 — Masters Edition II | Scryfall ID: 3ca0d060-7bf7-4311-92f4-6cdfef6f85df | Oracle ID: e0587218-206e-41ed-af3c-f06b7a668e90
use baylee_cards_dsl::prelude::*;

card!(
    index = index::RITUAL_OF_SUBDUAL,
    oracle_id = "e0587218-206e-41ed-af3c-f06b7a668e90",
    scryfall_id = "3ca0d060-7bf7-4311-92f4-6cdfef6f85df",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ritual of Subdual",
        mana_cost = mana!("{4}{G}{G}"),
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
                    each: mana!("{2}"),
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
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Colorless,
            }
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Colorless,
            }
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Black,
                to: ManaColor::Colorless,
            }
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Colorless,
            }
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::Colorless,
            }
        ),
    ],
);
