//! Ring of Ma'rûf — {5} — Artifact
//! Oracle: {5}, {T}, Exile this artifact: The next time you would draw a card this turn, instead put a card you own from outside the game into your hand.
//! Set: ME1 #163 — Masters Edition | Scryfall ID: fa740755-244f-4658-a9e2-aa4cf6742808 | Oracle ID: c79b9187-cbfe-43a0-bdc8-4f7e0d215607
// PARTIAL — the ability is off the card: nothing replaces the draw it was
// meant to replace.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RING_OF_MA_RUF,
    oracle_id = "c79b9187-cbfe-43a0-bdc8-4f7e0d215607",
    scryfall_id = "fa740755-244f-4658-a9e2-aa4cf6742808",
    faces = &[face!(
        name = "Ring of Ma'rûf",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the next draw cannot be replaced: Effect::WishToHand exists but \
         resolves immediately, handing over the card whether or not a draw \
         follows"
    ),
    // NOT SUPPORTED: "{5}, {T}, Exile this artifact: The next time you would
    // draw a card this turn, instead put a card you own from outside the game
    // into your hand." — the cost is sayable (`cost!("{5}", TapSelf,
    // ExileSelf)`) and `Effect::WishToHand` reaches the sideboard, but what
    // makes the card is the replacement: no `ReplacementRule` and no effect
    // says "instead of drawing". `Effect::WishToHand` alone would put the
    // card into hand as the ability resolves, whether or not another draw
    // happens this turn, which is strictly stronger than the printed Ring.
);
