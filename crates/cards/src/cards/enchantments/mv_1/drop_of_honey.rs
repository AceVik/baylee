//! Drop of Honey — {G} — Enchantment
//! Oracle: At the beginning of your upkeep, destroy the creature with the least power. It can't be regenerated. If two or more creatures are tied for least power, you choose one of them.
//! Oracle: When there are no creatures on the battlefield, sacrifice this enchantment.
//! Set: ME4 #150 — Masters Edition IV | Scryfall ID: 588f1bc5-2230-4437-8d5e-a18f6e55b390 | Oracle ID: 383e9005-5869-4d1d-917d-30e5f214fbd9
use baylee_cards_dsl::prelude::*;

card!(
    index = index::DROP_OF_HONEY,
    oracle_id = "383e9005-5869-4d1d-917d-30e5f214fbd9",
    scryfall_id = "588f1bc5-2230-4437-8d5e-a18f6e55b390",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Drop of Honey",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::DestroyWeakest {
                filter: &Filter::CREATURE,
                no_regen: true
            }]
        ),
        triggered!(
            Trigger::State(&Condition::BattlefieldCountAtMost(&Filter::CREATURE, 0)),
            &[Effect::SacrificeSelf]
        )
    ],
);
