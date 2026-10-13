//! Backdraft — {1}{R} — Instant
//! Oracle: Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.
//! Set: LEG #132 — Legends | Scryfall ID: 58d5b9fe-b66a-48c9-94c4-db783e605f37 | Oracle ID: 80734bb0-5032-4355-9304-49f4a4557aba
// IMPLEMENTED — the controller picks a player who cast a sorcery this turn
// (and one of that player's sorceries); half its damage, rounded down, goes
// to that player.

use baylee_cards_dsl::prelude::*;

static SORCERY_SPELL: Filter = Filter::HasType(TypeSet::SORCERY);

card!(
    index = index::BACKDRAFT,
    oracle_id = "80734bb0-5032-4355-9304-49f4a4557aba",
    scryfall_id = "58d5b9fe-b66a-48c9-94c4-db783e605f37",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Backdraft",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::DealHalfOfSpellDamage {
        spell: &SORCERY_SPELL,
    }])],
);
