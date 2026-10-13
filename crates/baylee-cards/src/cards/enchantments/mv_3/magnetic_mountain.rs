//! Magnetic Mountain — {1}{R}{R} — Enchantment
//! Oracle: Blue creatures don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player may choose any number of tapped blue creatures they control and pay {4} for each creature chosen this way. If the player does, untap those creatures.
//! Set: 4ED #209 — Fourth Edition | Scryfall ID: 993a14d5-a33d-426e-ab49-c3226a6fcdca | Oracle ID: 5c28fc22-7da6-4f15-b02b-6ff10981466c
// IMPLEMENTED — blue creatures don't untap; each upkeep that player may pay
// {4} per tapped blue creature they choose to untap it.

use baylee_cards_dsl::prelude::*;

static BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::of(Color::Blue)),
]);

card!(
    index = index::MAGNETIC_MOUNTAIN,
    oracle_id = "5c28fc22-7da6-4f15-b02b-6ff10981466c",
    scryfall_id = "993a14d5-a33d-426e-ab49-c3226a6fcdca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Magnetic Mountain",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(BLUE_CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer
            },
            &[Effect::UntapChosenPayingEach {
                who: PlayerRel::ActivePlayer,
                filter: &BLUE_CREATURE,
                each: mana!("{4}")
            }]
        ),
    ],
);
