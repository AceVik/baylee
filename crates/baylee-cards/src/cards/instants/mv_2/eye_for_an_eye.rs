//! Eye for an Eye — {W}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage to you this turn, instead that source deals that much damage to you and Eye for an Eye deals that much damage to that source's controller.
//! Set: ME4 #12 — Masters Edition IV | Scryfall ID: 7b4b8de1-a548-4398-887e-f95b4c15590c | Oracle ID: 22647b1a-5a7c-41b5-b820-b2e9f49c7aad
// PARTIAL — the redirection is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EYE_FOR_AN_EYE,
    oracle_id = "22647b1a-5a7c-41b5-b820-b2e9f49c7aad",
    scryfall_id = "7b4b8de1-a548-4398-887e-f95b4c15590c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eye for an Eye",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "nothing reflects the next damage from a chosen source back at that \
         source's controller; the closest piece, \
         Effect::PreventNextFromChosenSource, chooses the source but \
         prevents the damage instead"
    ),
    // NOT SUPPORTED: "The next time a source of your choice would deal
    // damage to you this turn, instead that source deals that much damage
    // to you and Eye for an Eye deals that much damage to that source's
    // controller." — `Effect::PreventNextFromChosenSource` does let the
    // controller choose a source and wait for its next damage, but its
    // shield prevents the damage rather than letting it be dealt again,
    // and nothing adds the reflected damage: no `PlayerRel` names the
    // controller of a replaced damage event's source, and
    // `Modifier::RedirectDamageToYou` moves damage to a permanent instead
    // of duplicating it. So the spell comes off the card.
);
