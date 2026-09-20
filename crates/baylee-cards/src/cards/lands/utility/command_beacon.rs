//! Command Beacon — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Put your commander into your hand from the command zone.
//! Set: TDC #352 — Tarkir: Dragonstorm Commander | Scryfall ID: 435e9678-9ff3-4e5d-8061-3f806e1c2ed2 | Oracle ID: 7e8c2a18-e404-40ff-a9e0-ec3eeb6d576e
// PARTIAL — {T}: Add {C} is built; the second ability has no DSL vocabulary
// at all (no Effect reaches the command zone), so the land is playable as a
// colorless mana source and nothing more.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMMAND_BEACON,
    oracle_id = "7e8c2a18-e404-40ff-a9e0-ec3eeb6d576e",
    scryfall_id = "435e9678-9ff3-4e5d-8061-3f806e1c2ed2",
    faces = &[face!(name = "Command Beacon", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the command-zone ability is inexpressible: no Effect variant moves a card out of the \
         command zone (WishToHand reaches outside-the-game and face-up exile only) and no \
         TargetSpec names a card there"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}, Sacrifice this land: Put your commander into
        // your hand from the command zone." — the cost is expressible
        // (`cost!(TapSelf, SacrificeSelf)`) but nothing the ability could do
        // is: `ZoneRef::Command` exists only as a filter zone, and every
        // zone-moving effect in the vocabulary (GraveyardToHand,
        // ReturnChosenToHand, WishToHand, SearchLibrary) reads a library, a
        // graveyard, a hand, exile or the battlefield.
    ],
);
