//! Chaos Moon — {3}{R} — Enchantment
//! Oracle: At the beginning of each upkeep, count the number of permanents. If the number is odd, until end of turn, red creatures get +1/+1 and whenever a player taps a Mountain for mana, that player adds an additional {R}. If the number is even, until end of turn, red creatures get -1/-1 and if a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type.
//! Set: ICE #179 — Ice Age | Scryfall ID: aae0543f-7f8b-4327-b735-ac21244e9936 | Oracle ID: 728e2960-1d85-4b3f-8b5b-915f9b9ffd2f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const RED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);
const MOUNTAIN: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::MOUNTAIN)]);
const EVEN_PERMANENT_COUNT: Condition = Condition::AmountIsEven(Amount::CountOf {
    filter: &Filter::Any,
    zone: ZoneSel::Battlefield,
});

const EVEN_EFFECTS: &[Effect] = &[
    Effect::PumpFilter {
        filter: &RED_CREATURE,
        controlled_by: None,
        power: Amount::NegXFixed(1),
        toughness: Amount::NegXFixed(1),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    },
    Effect::continuous(
        &MOUNTAIN,
        Modifier::ReplaceTappedMana {
            from: ManaColor::White,
            to: ManaColor::Colorless,
        },
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &MOUNTAIN,
        Modifier::ReplaceTappedMana {
            from: ManaColor::Blue,
            to: ManaColor::Colorless,
        },
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &MOUNTAIN,
        Modifier::ReplaceTappedMana {
            from: ManaColor::Black,
            to: ManaColor::Colorless,
        },
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &MOUNTAIN,
        Modifier::ReplaceTappedMana {
            from: ManaColor::Red,
            to: ManaColor::Colorless,
        },
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &MOUNTAIN,
        Modifier::ReplaceTappedMana {
            from: ManaColor::Green,
            to: ManaColor::Colorless,
        },
        Duration::UntilEndOfTurn,
    ),
];

const ODD_EFFECTS: &[Effect] = &[
    Effect::PumpFilter {
        filter: &RED_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(1),
        toughness: Amount::Fixed(1),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    },
    Effect::WheneverTappedForManaThisTurn {
        by: PlayerRel::EachPlayer,
        filter: &MOUNTAIN,
        effects: &[Effect::AddManaFor {
            who: PlayerRel::EventPlayer,
            color: ManaColor::Red,
            amount: 1,
        }],
    },
];

card!(
    index = index::CHAOS_MOON,
    oracle_id = "728e2960-1d85-4b3f-8b5b-915f9b9ffd2f",
    scryfall_id = "aae0543f-7f8b-4327-b735-ac21244e9936",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chaos Moon",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        // This is a resolution-time choice, not an intervening-if: the upkeep
        // trigger always goes on the stack and counts permanents as it resolves.
        &[Effect::IfCondition {
            condition: EVEN_PERMANENT_COUNT,
            then: EVEN_EFFECTS,
            otherwise: ODD_EFFECTS,
        }],
    )],
);
