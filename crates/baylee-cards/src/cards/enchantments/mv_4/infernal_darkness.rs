//! Infernal Darkness — {2}{B}{B} — Enchantment
//! Oracle: Cumulative upkeep—Pay {B} and 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If a land is tapped for mana, it produces {B} instead of any other type.
//! Set: ME2 #97 — Masters Edition II | Scryfall ID: 0230e9bf-da21-4014-a834-d4fd2f5b31df | Oracle ID: dd821fca-79a0-48a9-b1cf-5a496bc04f65
// NOT SUPPORTED: cumulative upkeep pays {B} per age counter without also paying 1 life per age counter.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFERNAL_DARKNESS,
    oracle_id = "dd821fca-79a0-48a9-b1cf-5a496bc04f65",
    scryfall_id = "0230e9bf-da21-4014-a834-d4fd2f5b31df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "cumulative upkeep pays {B} per age counter without also paying 1 life per age counter",
    ),
    faces = &[face!(
        name = "Infernal Darkness",
        mana_cost = mana!("{2}{B}{B}"),
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
                    each: mana!("{B}"),
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
                to: ManaColor::Black,
            },
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Blue,
                to: ManaColor::Black,
            },
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Red,
                to: ManaColor::Black,
            },
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Green,
                to: ManaColor::Black,
            },
        ),
        static_ability!(
            Filter::LAND,
            Modifier::ReplaceTappedMana {
                from: ManaColor::Colorless,
                to: ManaColor::Black,
            },
        ),
    ],
);
