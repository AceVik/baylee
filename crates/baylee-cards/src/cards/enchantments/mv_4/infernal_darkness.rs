//! Infernal Darkness — {2}{B}{B} — Enchantment
//! Oracle: Cumulative upkeep—Pay {B} and 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: If a land is tapped for mana, it produces {B} instead of any other type.
//! Set: ME2 #97 — Masters Edition II | Scryfall ID: 0230e9bf-da21-4014-a834-d4fd2f5b31df | Oracle ID: dd821fca-79a0-48a9-b1cf-5a496bc04f65
// PARTIAL — "{B} and 1 life" for each age counter is two payments in a row (the {B} for each counter, then the life for each counter), because no single payment names both: mana is spent even when the life is then refused, and Braidwood Cup, Drought and Adarkar Unicorn do not read this as a cumulative upkeep (Effect::is_cumulative_upkeep).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFERNAL_DARKNESS,
    oracle_id = "dd821fca-79a0-48a9-b1cf-5a496bc04f65",
    scryfall_id = "0230e9bf-da21-4014-a834-d4fd2f5b31df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "cumulative upkeep asks for the {B} per age counter and then the 1 life per age counter as two payments in a row, so mana is spent even when the life is refused, and Braidwood Cup, Drought and Adarkar Unicorn do not see it as a cumulative upkeep",
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
                    paid: &[Effect::PlayerMayPayLifeOr {
                        player: PlayerRel::You,
                        life: Amount::CountersOnSource(counters::AGE),
                        effect: &Effect::SacrificeSelf,
                    }],
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
