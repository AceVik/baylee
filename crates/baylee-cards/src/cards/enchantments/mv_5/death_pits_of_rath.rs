//! Death Pits of Rath — {3}{B}{B} — Enchantment
//! Oracle: Whenever a creature is dealt damage, destroy it. It can't be regenerated.
//! Set: TPR #99 — Tempest Remastered | Scryfall ID: ad19107e-9b31-40c1-9b5d-22f4727d4d2a | Oracle ID: aead53ef-ffcf-4ae2-ab81-40aa9627adc9
// PARTIAL

use baylee_cards_dsl::prelude::*;

static DAMAGED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::DealtDamageThisTurn]);

card!(
    index = index::DEATH_PITS_OF_RATH,
    oracle_id = "aead53ef-ffcf-4ae2-ab81-40aa9627adc9",
    scryfall_id = "ad19107e-9b31-40c1-9b5d-22f4727d4d2a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Whenever a creature is dealt damage, destroys creatures that have been dealt damage this turn (they can't be regenerated); Trigger::DealtDamage does not yet bind the specific damaged creature as the event object."
    ),
    faces = &[face!(
        name = "Death Pits of Rath",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::CREATURE),
        &[Effect::destroy_all_no_regen(&DAMAGED_CREATURE)],
    )],
);
