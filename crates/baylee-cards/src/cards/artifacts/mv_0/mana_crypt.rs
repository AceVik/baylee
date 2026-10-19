//! Mana Crypt — {0} — Artifact
//! Oracle: At the beginning of your upkeep, flip a coin. If you lose the flip, this artifact deals 3 damage to you.
//! Oracle: {T}: Add {C}{C}.
//! Set: 2XM #270 — Double Masters | Scryfall ID: 4d960186-4559-4af0-bd22-63baa15f8939 | Oracle ID: 2c63e4e1-89d2-4bc6-a232-94e75c4b1c8a

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_CRYPT,
    oracle_id = "2c63e4e1-89d2-4bc6-a232-94e75c4b1c8a",
    scryfall_id = "4d960186-4559-4af0-bd22-63baa15f8939",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mana Crypt",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::FlipCoin {
                won: &[],
                lost: &[Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::You),
                }],
            }],
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
