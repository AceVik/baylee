//! Storm World — {R} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is 4 minus the number of cards in their hand.
//! Set: ME3 #111 — Masters Edition III | Scryfall ID: ad43358a-6369-4b8b-a0b7-f8ba07c1bf39 | Oracle ID: 868f4ab2-a846-4ad0-8720-95fd234dd36b
// PARTIAL — the printed amount ("4 minus the number of cards in their hand")
// has no Amount, so the upkeep trigger is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORM_WORLD,
    oracle_id = "868f4ab2-a846-4ad0-8720-95fd234dd36b",
    scryfall_id = "ad43358a-6369-4b8b-a0b7-f8ba07c1bf39",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Amount computes a constant minus a counted quantity: \
         Amount::SaturatingSub subtracts a constant from an amount, and \
         Amount::Plus over a negative base is refused by amount_sign_tests"
    ),
    faces = &[face!(
        name = "Storm World",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// NOT SUPPORTED: "At the beginning of each player's upkeep, this enchantment
// deals X damage to that player, where X is 4 minus the number of cards in
// their hand." — `Amount::SaturatingSub` is `count - 4` (Black Vise's
// direction, The Rack's missing reverse) and a constant minus a count floored
// at zero has no variant, so the trigger comes off the card rather than
// dealing the wrong direction.
