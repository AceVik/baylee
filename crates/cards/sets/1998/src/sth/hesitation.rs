//! Hesitation — {1}{U} — Enchantment
//! Oracle: When a player casts a spell, sacrifice this enchantment and counter that spell.
//! Set: STH #33 — Stronghold | Scryfall ID: e3ce5dbc-3597-42e8-859a-cac105c8102e | Oracle ID: 11eb31d9-cbcb-4ec5-abab-dfa646addbba
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HESITATION,
    oracle_id = "11eb31d9-cbcb-4ec5-abab-dfa646addbba",
    scryfall_id = "e3ce5dbc-3597-42e8-859a-cac105c8102e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hesitation",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::Any),
        &[Effect::SacrificeSelf, Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);
