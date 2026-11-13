//! Reverent Silence — {3}{G} — Sorcery
//! Oracle: If you control a Forest, rather than pay this spell's mana cost, you may have each other player gain 6 life.
//! Oracle: Destroy all enchantments.
//! Set: NEM #111 — Nemesis | Scryfall ID: b82d3432-2167-4a65-8221-cb7b338e60d0 | Oracle ID: e670d1c9-5e28-4df8-aad9-d08d19dc74e9
// NOT SUPPORTED: alternative cost of having each other player gain 6 life when you control a Forest is not supported
// PARTIAL — base {3}{G} destroy all enchantments implemented; alternative cost of having each other player gain 6 life when you control a Forest is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERENT_SILENCE,
    oracle_id = "e670d1c9-5e28-4df8-aad9-d08d19dc74e9",
    scryfall_id = "b82d3432-2167-4a65-8221-cb7b338e60d0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "alternative cost of having each other player gain 6 life when you control a Forest is not supported",
    ),
    faces = &[face!(
        name = "Reverent Silence",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyAll {
        filter: &Filter::ENCHANTMENT,
        no_regen: false,
    }]),],
);
