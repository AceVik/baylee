//! Iron Maiden — {3} — Artifact
//! Oracle: At the beginning of each opponent's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4.
//! Set: ULG #127 — Urza's Legacy | Scryfall ID: ad925fb0-1d5c-44a0-8347-202a38c23107 | Oracle ID: e0351638-9224-4779-8f7c-f2b11d134a5f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IRON_MAIDEN,
    oracle_id = "e0351638-9224-4779-8f7c-f2b11d134a5f",
    scryfall_id = "ad925fb0-1d5c-44a0-8347-202a38c23107",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Iron Maiden",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::Opponent,
        },
        &[Effect::DealDamage {
            amount: Amount::SaturatingSub {
                base: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandActivePlayer,
                },
                subtract: 4,
            },
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
    )],
);
