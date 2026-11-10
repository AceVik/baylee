//! Crypt Rats — {2}{B} — Creature — Rat
//! Oracle: {X}: This creature deals X damage to each creature and each player. Spend only black mana on X.
//! Set: MH1 #84 — Modern Horizons | Scryfall ID: 96916db2-5121-4ff1-880c-369744f11ecf | Oracle ID: 104095ed-55e3-408e-bf70-4fe06bb16d2f
// PARTIAL — the 1/1 body only: the {X} damage ability is not played, since "Spend only black mana on X" cannot be said for an activated ability, and with any mana paying X it would be stronger than printed.
// NOT SUPPORTED: "{X}: This creature deals X damage to each creature and each player. Spend only black mana on X." An activated ability's X cannot be limited to black mana (`x_mana_color` limits the X of a cast spell only), so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRYPT_RATS,
    oracle_id = "104095ed-55e3-408e-bf70-4fe06bb16d2f",
    scryfall_id = "96916db2-5121-4ff1-880c-369744f11ecf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the 1/1 body only: the {X} damage ability is left off because \"spend only black mana on X\" cannot be said for an activated ability, and with any mana paying X it would be stronger than printed",
    ),
    faces = &[face!(
        name = "Crypt Rats",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(1),
        toughness = Some(1),
    ),],
);
