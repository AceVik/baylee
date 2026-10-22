//! Reparations — {1}{W}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell that targets you or a creature you control, you may draw a card.
//! Set: MIR #278 — Mirage | Scryfall ID: 0a9edf28-79c0-42a5-af0f-6df9c3a1f546 | Oracle ID: 3cb86deb-f4e5-41c5-bfcd-614a24f3499c
// PARTIAL: Trigger::SpellCast has no filter for spells that target you or a creature you control; triggers whenever an opponent casts a spell and lets you optionally draw a card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPARATIONS,
    oracle_id = "3cb86deb-f4e5-41c5-bfcd-614a24f3499c",
    scryfall_id = "0a9edf28-79c0-42a5-af0f-6df9c3a1f546",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial(
        "Trigger::SpellCast has no filter for spells that target you or a creature you control; triggers whenever an opponent casts a spell and lets you optionally draw a card",
    ),
    faces = &[face!(
        name = "Reparations",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByOpponent),
        &[Effect::MayDo {
            effects: &[Effect::draw(1)],
        }],
    )],
);
