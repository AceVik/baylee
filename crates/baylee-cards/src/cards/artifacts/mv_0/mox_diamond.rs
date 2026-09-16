//! Mox Diamond — {0} — Artifact
//! Oracle: If this artifact would enter, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add one mana of any color.
//! Set: TPR #228 — Tempest Remastered | Scryfall ID: bf9fecfd-d122-422f-bd0a-5bf69b434dfe | Oracle ID: f3c5978a-70fa-431f-933b-b954bd0db0ea
// PARTIAL — {T}: Add one mana of any color is built; the enter replacement is not, so it is Partial.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOX_DIAMOND,
    oracle_id = "f3c5978a-70fa-431f-933b-b954bd0db0ea",
    scryfall_id = "bf9fecfd-d122-422f-bd0a-5bf69b434dfe",
    faces = &[face!(
        name = "Mox Diamond",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the enter-the-battlefield replacement: discard a land or go to the graveyard"
    ),
    abilities = &[mana_ability!(&[Effect::mana_of_any_color()])],
);

// NOT SUPPORTED: "If this artifact would enter, you may discard a land card
// instead. If you do, put this artifact onto the battlefield. If you don't,
// put it into its owner's graveyard." — FaceDef::enter_modifiers carries only
// entry *states* (Tapped, TappedUnless, TappedUnlessCount, TappedOrPayLife,
// ChooseSubtype) and ReplacementRule only token/counter doubling and trigger
// multipliers, so nothing can make the entry conditional on a discard; the
// artifact enters and its controller keeps the land.
