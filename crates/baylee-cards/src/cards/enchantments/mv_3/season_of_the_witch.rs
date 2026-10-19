//! Season of the Witch — {B}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay 2 life.
//! Oracle: At the beginning of the end step, destroy all untapped creatures that didn't attack this turn, except for creatures that couldn't attack.
//! Set: DRK #52 — The Dark | Scryfall ID: 06900a71-34ca-48c6-94ac-fca744356829 | Oracle ID: 307813c5-c3a8-46c5-8ed4-3b244dd169dc

use baylee_cards_dsl::prelude::*;

static UNTAPPED_DIDNT_ATTACK_COULD_HAVE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Untapped,
    Filter::Not(&Filter::AttackedThisTurn),
    Filter::CouldHaveAttackedThisTurn,
]);

card!(
    index = index::SEASON_OF_THE_WITCH,
    oracle_id = "307813c5-c3a8-46c5-8ed4-3b244dd169dc",
    scryfall_id = "06900a71-34ca-48c6-94ac-fca744356829",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Season of the Witch",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayLifeOr {
                player: PlayerRel::You,
                life: Amount::Fixed(2),
                effect: &Effect::SacrificeSelf,
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::DestroyAll {
                filter: &UNTAPPED_DIDNT_ATTACK_COULD_HAVE,
                no_regen: false,
            }]
        ),
    ],
);
