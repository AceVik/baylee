//! The Rack — {1} — Artifact
//! Oracle: As this artifact enters, choose an opponent.
//! Oracle: At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand.
//! Set: TSB #113 — Time Spiral Timeshifted | Scryfall ID: 696d1e25-5c25-4522-b085-90d49fe23a18 | Oracle ID: 3d873e1d-4fac-42c4-bb31-77e76099e1ef
// PARTIAL — the whole ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_RACK,
    oracle_id = "3d873e1d-4fac-42c4-bb31-77e76099e1ef",
    scryfall_id = "696d1e25-5c25-4522-b085-90d49fe23a18",
    faces = &[face!(
        name = "The Rack",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Amount computes a constant minus a counted quantity, so the \
         trigger has no amount it can carry"
    ),
    // NOT SUPPORTED: "As this artifact enters, choose an opponent. At the
    // beginning of the chosen player's upkeep, this artifact deals X damage
    // to that player, where X is 3 minus the number of cards in their hand."
    // — the choice and the trigger exist (`EnterModifier::ChooseOpponent`,
    // `Trigger::StepBeginChosenOpponent`), but every Amount reads
    // left-to-right: `CountOf` minus a constant (`SaturatingSub`) is Black
    // Vise's `hand - 4`, and the reverse — a constant minus a count, floored
    // at zero — has no variant, so the trigger comes off the card instead of
    // dealing the wrong direction. See TODO.md.
);
