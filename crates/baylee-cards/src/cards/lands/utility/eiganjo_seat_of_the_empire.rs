//! Eiganjo, Seat of the Empire — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: Channel — {2}{W}, Discard this card: It deals 4 damage to target attacking or blocking creature. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #268 — Kamigawa: Neon Dynasty | Scryfall ID: c375a022-5b57-496d-a802-e4ea8376e9e4 | Oracle ID: 7edb3d15-4f70-4ebe-8c5e-caf6a225076d
// PARTIAL — the mana ability is built; the channel ability is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EIGANJO_SEAT_OF_THE_EMPIRE,
    oracle_id = "7edb3d15-4f70-4ebe-8c5e-caf6a225076d",
    scryfall_id = "c375a022-5b57-496d-a802-e4ea8376e9e4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eiganjo, Seat of the Empire",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "Channel: `Filter` has no `Blocking`, so a creature that is attacking or blocking \
         cannot be named, and nothing in `Cost`/`CostReduction` makes an activation cheaper",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: Channel — {2}{W}, Discard this card: It deals 4 damage to target attacking or blocking creature. This ability costs {1} less to activate for each legendary creature you control.
    ],
);
