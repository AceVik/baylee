//! Soldevi Sage — {1}{U} — Creature — Human Wizard
//! Oracle: {T}, Sacrifice two lands: Draw three cards, then discard one of them.
//! Set: 6ED #99 — Classic Sixth Edition | Scryfall ID: 268c3726-0e2d-40df-811d-2cdf6b328ea3 | Oracle ID: 1f612df3-53b6-4317-9d63-1f903ee3f0c4

// PARTIAL — a vanilla 1/1: the activated ability is not played.
// NOT SUPPORTED: "{T}, Sacrifice two lands: Draw three cards, then discard one of them." The discard must be one of the three cards just drawn; DiscardForPlayers takes any card from the hand, a wider choice than printed (a card held before could be discarded in place of a drawn one), so the ability is off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDEVI_SAGE,
    oracle_id = "1f612df3-53b6-4317-9d63-1f903ee3f0c4",
    scryfall_id = "268c3726-0e2d-40df-811d-2cdf6b328ea3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Soldevi Sage",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "\"Draw three cards, then discard one of them\" cannot name the discard among the cards drawn this way: the only discard effect takes any card from the hand, which is more than printed; Soldevi Sage enters as a 1/1 with no abilities."
    ),
);
