//! Forethought Amulet — {5} — Artifact
//! Oracle: At the beginning of your upkeep, sacrifice this artifact unless you pay {3}.
//! Oracle: If an instant or sorcery source would deal 3 or more damage to you, it deals 2 damage to you instead.
//! Set: LEG #277 — Legends | Scryfall ID: 700f53d3-0a84-4c55-8495-786f0f0783db | Oracle ID: 0edf0988-9ed8-4fe6-aa47-4921870f05a8
// IMPLEMENTED — upkeep payment or sacrifice, plus a damage replacement
// capping instant and sorcery damage to you at two.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORETHOUGHT_AMULET,
    oracle_id = "0edf0988-9ed8-4fe6-aa47-4921870f05a8",
    scryfall_id = "700f53d3-0a84-4c55-8495-786f0f0783db",
    faces = &[face!(
        name = "Forethought Amulet",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ReplacePlayerDamage {
                sources: &Filter::INSTANT_OR_SORCERY,
                recipients: PlayerRel::You,
                combat: None,
                replacement: DamageAmountReplacement::Cap(2),
            }
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Fixed(3),
                effect: &Effect::SacrificeSelf
            }]
        ),
    ],
);
