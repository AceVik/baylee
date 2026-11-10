//! Krark-Clan Ironworks — {4} — Artifact
//! Oracle: Sacrifice an artifact: Add {C}{C}.
//! Set: 5DN #134 — Fifth Dawn | Scryfall ID: c60174d6-1f9d-4870-b3db-34d6fcb3f6ab | Oracle ID: 68e1f7e0-a9b3-437f-8086-0c0cb85f2880
// IMPLEMENTED — an artifact sacrifice outlet that turns each one into {C}{C}
// without using the stack (CR 605.1). No tap, so it eats the whole board a
// turn, and "an artifact" includes the Ironworks itself (CR 701.21a limits
// the choice to permanents you control, which `Filter::YOUR_ARTIFACT` says).
// Which artifact is a question `engine::cost_wizard` asks as the ability is
// activated, and the Ironworks is on its own menu.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KRARK_CLAN_IRONWORKS,
    oracle_id = "68e1f7e0-a9b3-437f-8086-0c0cb85f2880",
    scryfall_id = "c60174d6-1f9d-4870-b3db-34d6fcb3f6ab",
    faces = &[face!(
        name = "Krark-Clan Ironworks",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(Sacrifice(&Filter::YOUR_ARTIFACT)),
        &[Effect::mana(ManaColor::Colorless, 2)]
    ),],
);
