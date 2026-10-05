//! Knowledge Vault — {4} — Artifact
//! Oracle: {2}, {T}: Exile the top card of your library face down.
//! Oracle: {0}: Sacrifice this artifact. If you do, discard your hand, then put all cards exiled with this artifact into their owner's hand.
//! Oracle: When this artifact leaves the battlefield, put all cards exiled with it into their owner's graveyard.
//! Set: ME3 #198 — Masters Edition III | Scryfall ID: c416fca6-e201-4dbc-b9c7-248d70967746 | Oracle ID: 77d3cc31-837a-408a-b7f8-5aa9de78e1b7
// PARTIAL — nothing is built: every ability turns on exiling the top card
// of a library face down and returning the cards exiled with this artifact,
// and neither half is expressible.
// NOT SUPPORTED: "{2}, {T}: Exile the top card of your library face down." —
// no `Effect` exiles the top card of a library (`Effect::ExileTopMayCast`
// grants a cast permission for it and `Effect::Mill` sends cards to a
// graveyard), and nothing tracks a face-down exile.
// NOT SUPPORTED: "{0}: Sacrifice this artifact. If you do, discard your
// hand, then put all cards exiled with this artifact into their owner's
// hand." — `Effect::SacrificeSelf` and `Effect::DiscardHand` exist, but no
// effect returns the cards exiled with the source to a hand
// (`Effect::ReturnLinkedToBattlefield` is the only linked return).
// NOT SUPPORTED: "When this artifact leaves the battlefield, put all cards
// exiled with it into their owner's graveyard." — no effect moves the cards
// exiled with the source to a graveyard.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KNOWLEDGE_VAULT,
    oracle_id = "77d3cc31-837a-408a-b7f8-5aa9de78e1b7",
    scryfall_id = "c416fca6-e201-4dbc-b9c7-248d70967746",
    faces = &[face!(
        name = "Knowledge Vault",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no effect exiles the top card of a library face down, and no effect \
         returns the cards exiled with the source to a hand or graveyard",
    ),
);
