//! Life Chisel — {4} — Artifact
//! Oracle: Sacrifice a creature: You gain life equal to the sacrificed creature's toughness. Activate only during your upkeep.
//! Set: ME3 #199 — Masters Edition III | Scryfall ID: 6d06edbe-2278-4818-b476-75bc02958418 | Oracle ID: 2493fe65-2ecb-418f-8e4e-797f83475c73
// PARTIAL — the whole ability is off the card: the amount is not
// expressible, see the NOT SUPPORTED line above `abilities`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_CHISEL,
    oracle_id = "2493fe65-2ecb-418f-8e4e-797f83475c73",
    scryfall_id = "6d06edbe-2278-4818-b476-75bc02958418",
    faces = &[face!(
        name = "Life Chisel",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Amount variant reads the toughness of a creature sacrificed to pay \
         an activation cost, so \"life equal to the sacrificed creature's \
         toughness\" cannot be stated"
    ),
    // NOT SUPPORTED: "Sacrifice a creature: You gain life equal to the
    // sacrificed creature's toughness. Activate only during your upkeep." —
    // the sacrifice cost (`cost!(Sacrifice(&Filter::CREATURE))`), the life
    // gain (`Effect::GainLife`), and the upkeep window
    // (`Condition::All(&[YourTurn, DuringStep(StepKind::Upkeep)])`) are each
    // sayable, but the amount is not: `Amount` carries
    // `Amount::SacrificedManaValue` and no sacrificed-toughness twin, and the
    // sacrificed creature is not a target, so `Amount::TargetPower` (the only
    // printed-power reader) cannot reach it either.
);
