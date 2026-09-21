//! Forsaken City — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may exile a card from your hand. If you do, untap this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: PLS #139 — Planeshift | Scryfall ID: 676703fe-bd80-413c-8704-1da5d3248b7e | Oracle ID: 6bb00a28-8b5a-4049-93b7-3db02de88aeb
// PARTIAL — the untap suppression (a `Layer::Text` rules modifier) and the
// any-color mana ability are built; the upkeep clause is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORSAKEN_CITY,
    oracle_id = "6bb00a28-8b5a-4049-93b7-3db02de88aeb",
    scryfall_id = "676703fe-bd80-413c-8704-1da5d3248b7e",
    faces = &[face!(name = "Forsaken City", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the upkeep clause exiles a card from your hand, which no Effect can \
         say: CostPart::ExileFromHand is paid only in the cast wizard, and \
         PlayerMayPayCostOr takes a price named by an object",
    ),
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        mana_ability!(&[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "At the beginning of your upkeep, you may exile a card
        // from your hand. If you do, untap this land." — there is no
        // `Effect::ExileFromHand`, and no wrapper carries a cost into a "may";
        // writing only `MayDo { UntapSelf }` would be a free untap every turn.
    ],
);
