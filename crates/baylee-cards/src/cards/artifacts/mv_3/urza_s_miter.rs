//! Urza's Miter — {3} — Artifact
//! Oracle: Whenever an artifact you control is put into a graveyard from the battlefield, if it wasn't sacrificed, you may pay {3}. If you do, draw a card.
//! Set: ME4 #237 — Masters Edition IV | Scryfall ID: 23270d99-5a25-4647-95f8-64da9b8e8831 | Oracle ID: 57f0c89c-79ad-4786-9497-c7e668620fc0
// PARTIAL — the trigger is off the card: no vocabulary asks whether the artifact was sacrificed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_MITER,
    oracle_id = "57f0c89c-79ad-4786-9497-c7e668620fc0",
    scryfall_id = "23270d99-5a25-4647-95f8-64da9b8e8831",
    faces = &[face!(
        name = "Urza's Miter",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Trigger or Condition can ask whether the object that left the \
         battlefield was sacrificed; Trigger::Dies fires for a sacrificed \
         artifact too"
    ),
    // NOT SUPPORTED: "Whenever an artifact you control is put into a graveyard
    // from the battlefield, if it wasn't sacrificed, you may pay {3}. If you
    // do, draw a card." — `Trigger::Dies(&Filter::YOUR_ARTIFACT)` is the zone
    // change and `Effect::PlayerMayPayThen` is the payment and the draw, but
    // the intervening "if it wasn't sacrificed" has no vocabulary:
    // `Condition` has no clause about how a permanent left the battlefield,
    // and the engine's `Cause` on the zone change is read by no `Filter`.
    // Without it the Miter pays out on every sacrifice, the one case the
    // clause exists to stop, so the trigger comes off the card.
    abilities = &[],
);
