//! Mariposa Military Base — (no cost) — Land
//! Oracle: You may have this land enter tapped. If you do, you get two rad counters.
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}: Draw a card. This ability costs {1} less to activate for each rad counter you have.
//! Set: PIP #151 — Fallout | Scryfall ID: cd7be9d4-f9fa-44a3-9902-7e9e226ccf56 | Oracle ID: f1e03d99-024a-430b-9342-ffd2268bd103
// PARTIAL — {T}: Add {C} and {5}, {T}: Draw a card are built; the optional
// tapped entry with its rad counters and the cost reduction are not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARIPOSA_MILITARY_BASE,
    oracle_id = "f1e03d99-024a-430b-9342-ffd2268bd103",
    scryfall_id = "cd7be9d4-f9fa-44a3-9902-7e9e226ccf56",
    faces = &[face!(
        name = "Mariposa Military Base",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial(
        "the optional tapped entry (and the two rad counters it grants) and the \
         {1}-less-per-rad-counter cost reduction are not expressible",
    ),
    // NOT SUPPORTED: "You may have this land enter tapped. If you do, you get
    // two rad counters." — no EnterModifier offers an *optional* tapped entry
    // that runs an effect (TappedOrPayLife charges life), and no effect puts
    // counters on a player.
    // NOT SUPPORTED: "This ability costs {1} less to activate for each rad
    // counter you have." — CostReduction holds only NotStartingPlayer and
    // nothing reads a counter on a player, so the ability is built at its
    // full printed {5}.
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(cost!("{5}", TapSelf), &[Effect::draw(1)]),
    ],
);
