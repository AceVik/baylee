//! Chaos Moon — {3}{R} — Enchantment
//! Oracle: At the beginning of each upkeep, count the number of permanents. If the number is odd, until end of turn, red creatures get +1/+1 and whenever a player taps a Mountain for mana, that player adds an additional {R}. If the number is even, until end of turn, red creatures get -1/-1 and if a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type.
//! Set: ICE #179 — Ice Age | Scryfall ID: aae0543f-7f8b-4327-b735-ac21244e9936 | Oracle ID: 728e2960-1d85-4b3f-8b5b-915f9b9ffd2f
// PARTIAL: always plays the odd branch's +1/+1, whatever the count; no -1/-1 and no Mountain mana changes.

use baylee_cards_dsl::prelude::*;

const RED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

// NOT SUPPORTED: "count the number of permanents. If the number is odd … If the number is even …": no condition reads the parity of a count, so every upkeep plays the odd branch's +1/+1 and never the even branch's -1/-1.
// NOT SUPPORTED: "whenever a player taps a Mountain for mana, that player adds an additional {R}" and "that Mountain produces colorless mana instead of any other type" until end of turn: neither Mountain rider is played.
card!(
    index = index::CHAOS_MOON,
    oracle_id = "728e2960-1d85-4b3f-8b5b-915f9b9ffd2f",
    scryfall_id = "aae0543f-7f8b-4327-b735-ac21244e9936",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "every upkeep red creatures get +1/+1 until end of turn whatever the number of permanents (no condition reads a count's parity), never -1/-1, and neither Mountain mana rider is played"
    ),
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
        &[Effect::PumpFilter {
            filter: &RED_CREATURE,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
