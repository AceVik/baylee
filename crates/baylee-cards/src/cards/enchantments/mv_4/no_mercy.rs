//! No Mercy — {2}{B}{B} — Enchantment
//! Oracle: Whenever a creature deals damage to you, destroy it.
//! Set: DMR #95 — Dominaria Remastered | Scryfall ID: 63c5e1dd-2fe6-4734-8406-7bcdf17aa60c | Oracle ID: b9538d53-480b-481a-abbf-83ab17e1a45b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NO_MERCY,
    oracle_id = "b9538d53-480b-481a-abbf-83ab17e1a45b",
    scryfall_id = "63c5e1dd-2fe6-4734-8406-7bcdf17aa60c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "No Mercy",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::CREATURE,
            player: PlayerRel::You,
        },
        &[Effect::destroy(TargetSpec::EventObject)],
    )],
);
