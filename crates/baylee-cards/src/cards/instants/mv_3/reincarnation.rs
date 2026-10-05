//! Reincarnation — {1}{G}{G} — Instant
//! Oracle: Choose target creature. When that creature dies this turn, return a creature card from its owner's graveyard to the battlefield under the control of that creature's owner.
//! Set: C13 #166 — Commander 2013 | Scryfall ID: 7ee379cd-b3fb-487b-846c-eab02902de79 | Oracle ID: d6bf5e22-8d33-43a9-8824-435068e0a87a
// PARTIAL — nothing is implemented: no effect registers the delayed
// "when that creature dies this turn" trigger the spell creates.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REINCARNATION,
    oracle_id = "d6bf5e22-8d33-43a9-8824-435068e0a87a",
    scryfall_id = "7ee379cd-b3fb-487b-846c-eab02902de79",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no effect registers the printed delayed dies trigger; the nearest shape, granting \
         the creature Trigger::Dies, would force the return's graveyard card to be an \
         announced target chosen by that creature's controller, where the card's delayed \
         trigger is the caster's and chooses on resolution from that creature's owner's \
         graveyard"
    ),
    faces = &[face!(
        name = "Reincarnation",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "When that creature dies this turn, return a creature card
    // from its owner's graveyard to the battlefield under the control of that
    // creature's owner." — no effect registers a delayed dies trigger, and
    // Effect::GraveyardToBattlefield reads its card from announced targets.
);
