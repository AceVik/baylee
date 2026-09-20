//! Lupinflower Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W}. Spend this mana only to cast a creature spell.
//! Oracle: {1}{W}, {T}, Sacrifice this land: Look at the top six cards of your library. You may reveal a Bat, Bird, Mouse, or Rabbit card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.
//! Set: BLB #256 — Bloomburrow | Scryfall ID: 8ab9d56f-9178-4ec9-a5f6-b934f50d8d9d | Oracle ID: b6c7c708-5212-4100-b954-b77855b27915
// PARTIAL — both mana abilities are built ({C}, and {W} restricted to creature
// spells); the {1}{W} look-at-six comes off the card, see the NOT SUPPORTED line.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LUPINFLOWER_VILLAGE,
    oracle_id = "b6c7c708-5212-4100-b954-b77855b27915",
    scryfall_id = "8ab9d56f-9178-4ec9-a5f6-b934f50d8d9d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Lupinflower Village", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {1}{W} activated ability: Effect::LookAtTopPick carries a count and \
         a pick and nothing else — no filter for the four creature types, no \
         \"you may\", and no statement about the order the rest go back in",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::White, 1).restricted(&Filter::CREATURE, SpendRider::None)
        ]),
        // NOT SUPPORTED: "{1}{W}, {T}, Sacrifice this land: Look at the top six
        // cards of your library. You may reveal a Bat, Bird, Mouse, or Rabbit
        // card from among them and put it into your hand. Put the rest on the
        // bottom of your library in a random order." — the nearest variant,
        // Effect::LookAtTopPick { count: 6, pick: 1 }, would let the player keep
        // any of the six cards and would force the choice, which is a card that
        // plays a different game rather than a card that is short one rider.
    ],
);
