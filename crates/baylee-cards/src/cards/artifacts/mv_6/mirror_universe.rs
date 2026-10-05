//! Mirror Universe — {6} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Exchange life totals with target opponent. Activate only during your upkeep.
//! Set: ME1 #159 — Masters Edition | Scryfall ID: 0f9cbeaf-3456-4a87-ac75-e7658ccbd97f | Oracle ID: 8d231d2f-5274-4b55-8262-9eb95654d183
// PARTIAL — the exchange ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRROR_UNIVERSE,
    oracle_id = "8d231d2f-5274-4b55-8262-9eb95654d183",
    scryfall_id = "0f9cbeaf-3456-4a87-ac75-e7658ccbd97f",
    faces = &[face!(
        name = "Mirror Universe",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Effect exchanges two players' life totals, so the exchange cannot \
         be stated"
    ),
    // NOT SUPPORTED: "{T}, Sacrifice this artifact: Exchange life totals
    // with target opponent. Activate only during your upkeep." — the cost
    // (`cost!(TapSelf, SacrificeSelf)`), the target and the "only during
    // your upkeep" gate (`Condition::All(&[YourTurn, DuringStep(
    // StepKind::Upkeep)])`) are all sayable, but no `Effect` exchanges life
    // totals.
);
