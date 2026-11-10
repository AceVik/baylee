//! Command Beacon — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Put your commander into your hand from the command zone.
//! Set: TDC #352 — Tarkir: Dragonstorm Commander | Scryfall ID: 435e9678-9ff3-4e5d-8061-3f806e1c2ed2 | Oracle ID: 7e8c2a18-e404-40ff-a9e0-ec3eeb6d576e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMMAND_BEACON,
    oracle_id = "7e8c2a18-e404-40ff-a9e0-ec3eeb6d576e",
    scryfall_id = "435e9678-9ff3-4e5d-8061-3f806e1c2ed2",
    faces = &[face!(name = "Command Beacon", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(cost!(TapSelf, SacrificeSelf), &[Effect::CommanderToHand]),
    ],
);
