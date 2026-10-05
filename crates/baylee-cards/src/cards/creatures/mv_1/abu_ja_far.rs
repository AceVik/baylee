//! Abu Ja'far — {W} — Creature — Human
//! Oracle: When this creature dies, destroy all creatures blocking or blocked by it. They can't be regenerated.
//! Set: CHR #1 — Chronicles | Scryfall ID: 023b5e6f-10de-422d-8431-11f1fdeca246 | Oracle ID: a2404d88-0621-49ae-9908-052c23a96ac6
// PARTIAL — the dies trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABU_JA_FAR,
    oracle_id = "a2404d88-0621-49ae-9908-052c23a96ac6",
    scryfall_id = "023b5e6f-10de-422d-8431-11f1fdeca246",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Abu Ja'far",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no filter names the creatures blocking or blocked by this one, so \
         the dies trigger has no effect it can carry"
    ),
    // NOT SUPPORTED: "When this creature dies, destroy all creatures
    // blocking or blocked by it. They can't be regenerated." — both halves
    // are sayable (`Trigger::Dies`, `Effect::destroy_all_no_regen`), but no
    // `Filter` names the combat partners of the source: `Filter::Blocking`
    // and `Filter::Unblocked` test a creature's own combat state, not a
    // relationship to another object. So the trigger comes off the card
    // instead of shipping as a no-op.
);
