//! Ali from Cairo — {2}{R}{R} — Creature — Human
//! Oracle: Damage that would reduce your life total to less than 1 reduces it to 1 instead.
//! Set: ME4 #107 — Masters Edition IV | Scryfall ID: 2719516a-7a47-413e-b1a3-15543d229e08 | Oracle ID: 6517edb3-30e2-40ba-b6e4-4554b4bbb342
// PARTIAL — the damage replacement is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALI_FROM_CAIRO,
    oracle_id = "6517edb3-30e2-40ba-b6e4-4554b4bbb342",
    scryfall_id = "2719516a-7a47-413e-b1a3-15543d229e08",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ali from Cairo",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no replacement or modifier floors a life total at 1: Modifier::CantLoseLife \
         stops the loss entirely and Modifier::NoLossForZeroLife only lifts the \
         zero-life state-based loss, so the damage-reduction sentence is off the card",
    ),
    // NOT SUPPORTED: "Damage that would reduce your life total to less than 1
    // reduces it to 1 instead." — the two life modifiers reach past and short
    // of the sentence: Modifier::CantLoseLife (Everybody Lives!) prevents the
    // loss outright, so damage that would leave 5 life still leaves 5, and
    // Modifier::NoLossForZeroLife (Lich) lets the total go to 0 or less
    // without changing how much damage is dealt. Neither replaces a loss that
    // would drop the total below 1 with a loss that drops it to exactly 1.
);
