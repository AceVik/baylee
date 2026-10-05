//! Diamond Valley — (no cost) — Land
//! Oracle: {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
//! Set: ME1 #175 — Masters Edition | Scryfall ID: 37ccc3ab-9875-4bd2-bdbd-e3af5e01d682 | Oracle ID: 84cef34a-c3e1-4059-b4cd-c481938a53a5
// PARTIAL — the sacrifice ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIAMOND_VALLEY,
    oracle_id = "84cef34a-c3e1-4059-b4cd-c481938a53a5",
    scryfall_id = "37ccc3ab-9875-4bd2-bdbd-e3af5e01d682",
    faces = &[face!(name = "Diamond Valley", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Amount variant reads the toughness of the permanent a cost sacrificed"
    ),
    // NOT SUPPORTED: "{T}, Sacrifice a creature: You gain life equal to the
    // sacrificed creature's toughness." — the cost is payable
    // (`cost!(TapSelf, Sacrifice(&Filter::CREATURE))`), but the amount is the
    // toughness of the permanent that paid it and no `Amount` can reach a
    // sacrificed object: `Amount::TargetPower` reads the first *target* and
    // this ability names none (making the creature a target would be a
    // different card — the printed sentence sacrifices at activation, so
    // hexproof never answers it), `Amount::SacrificedManaValue` reads a mana
    // value rather than a toughness, and no `Amount` reads toughness at all.
    // The ability comes off the card rather than gaining the wrong life.
);
