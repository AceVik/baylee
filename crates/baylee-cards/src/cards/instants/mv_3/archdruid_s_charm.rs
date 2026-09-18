//! Archdruid's Charm — {G}{G}{G} — Instant
//! Oracle: Choose one —
//! Oracle: • Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land card. Otherwise, put it into your hand. Then shuffle.
//! Oracle: • Put a +1/+1 counter on target creature you control. It deals damage equal to its power to target creature you don't control.
//! Oracle: • Exile target artifact or enchantment.
//! Set: MKM #151 — Murders at Karlov Manor | Scryfall ID: 5caae5ae-845f-42c2-b1ae-956df2739433 | Oracle ID: 3c1ef404-e2c6-486d-a5a2-d5779c71d498
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARCHDRUID_S_CHARM,
    oracle_id = "3c1ef404-e2c6-486d-a5a2-d5779c71d498",
    scryfall_id = "5caae5ae-845f-42c2-b1ae-956df2739433",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Archdruid's Charm",
        mana_cost = mana!("{G}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
