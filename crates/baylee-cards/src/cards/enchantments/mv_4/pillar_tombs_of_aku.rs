//! Pillar Tombs of Aku — {2}{B}{B} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, that player may sacrifice a creature of their choice. If that player doesn't, they lose 5 life and you sacrifice this enchantment.
//! Set: VIS #67 — Visions | Scryfall ID: 153f93fd-4f2c-4dce-a774-4483031ed532 | Oracle ID: b8cdf79f-b247-4346-8122-9a2d7d23b3c3
// PARTIAL: at the beginning of each player's upkeep, that player may sacrifice a creature or lose 5 life; sacrificing Pillar Tombs of Aku when they don't sacrifice a creature is not yet supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: also sacrificing Pillar Tombs of Aku in the unpaid branch of PlayerMayPayCostOr.
card!(
    index = index::PILLAR_TOMBS_OF_AKU,
    oracle_id = "b8cdf79f-b247-4346-8122-9a2d7d23b3c3",
    scryfall_id = "153f93fd-4f2c-4dce-a774-4483031ed532",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "at the beginning of each player's upkeep, that player may sacrifice a creature or lose 5 life; sacrificing Pillar Tombs of Aku when they don't sacrifice a creature is not yet supported"
    ),
    faces = &[face!(
        name = "Pillar Tombs of Aku",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::ActivePlayer,
            cost: &CostPart::Sacrifice(&Filter::CREATURE),
            effect: &Effect::LoseLife {
                amount: Amount::Fixed(5),
                target: PlayerRel::ActivePlayer,
            },
        }],
    )],
);
