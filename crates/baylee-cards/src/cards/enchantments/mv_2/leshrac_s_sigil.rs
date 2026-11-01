//! Leshrac's Sigil — {B}{B} — Enchantment
//! Oracle: Whenever an opponent casts a green spell, you may pay {B}{B}. If you do, look at that player's hand and choose a card from it. The player discards that card.
//! Oracle: {B}{B}: Return this enchantment to its owner's hand.
//! Set: ICE #144 — Ice Age | Scryfall ID: ad5ba7ee-d6df-4b62-a8a1-c81e6fca392a | Oracle ID: 98388eba-31ab-4302-be9e-ea67ea2b8c1a
// NOT SUPPORTED: triggered ability when an opponent casts a green spell to pay {B}{B}, look at their hand, and choose a card for them to discard is not supported; only the {B}{B} ability returning this enchantment to its owner's hand is implemented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LESHRAC_S_SIGIL,
    oracle_id = "98388eba-31ab-4302-be9e-ea67ea2b8c1a",
    scryfall_id = "ad5ba7ee-d6df-4b62-a8a1-c81e6fca392a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "triggered ability when an opponent casts a green spell to pay {B}{B}, look at their hand, and choose a card for them to discard is not supported; only the {B}{B} ability returning this enchantment to its owner's hand is implemented",
    ),
    faces = &[face!(
        name = "Leshrac's Sigil",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{B}{B}"),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
