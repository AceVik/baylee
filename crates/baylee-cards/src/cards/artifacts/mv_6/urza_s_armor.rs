//! Urza's Armor — {6} — Artifact
//! Oracle: If a source would deal damage to you, prevent 1 of that damage.
//! Set: USG #313 — Urza's Saga | Scryfall ID: df901d78-e8fb-4adf-bafe-99a567a375c1 | Oracle ID: e5fa232a-c8f2-4532-88bf-60d1223181b0
// PARTIAL: preventing 1 damage each time any source would deal damage to you is not in DamageAmountReplacement; caps damage dealt to you by any source at 1.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_ARMOR,
    oracle_id = "e5fa232a-c8f2-4532-88bf-60d1223181b0",
    scryfall_id = "df901d78-e8fb-4adf-bafe-99a567a375c1",
    coverage = Coverage::Partial(
        "preventing 1 damage each time any source would deal damage to you is not in DamageAmountReplacement; caps damage dealt to you by any source at 1",
    ),
    faces = &[face!(
        name = "Urza's Armor",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::ReplacePlayerDamage {
            sources: &Filter::Any,
            recipients: PlayerRel::You,
            combat: None,
            replacement: DamageAmountReplacement::Cap(1),
        }
    ),],
);
