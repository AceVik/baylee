//! Silhouette — {1}{U} — Instant
//! Oracle: Choose target creature. If a spell or ability that targets that creature would cause a source to deal damage to that creature this turn, prevent that damage.
//! Set: LEG #77 — Legends | Scryfall ID: e6d6fac6-9a23-465f-a813-92e1ed1cd742 | Oracle ID: 5a074b0a-3a4d-4c6f-b161-5d1281f6136d
// PARTIAL — the card's one sentence is a prevention shield the DSL cannot
// describe, so the instant carries no ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SILHOUETTE,
    oracle_id = "5a074b0a-3a4d-4c6f-b161-5d1281f6136d",
    scryfall_id = "e6d6fac6-9a23-465f-a813-92e1ed1cd742",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no prevention effect takes the damage's cause as a condition: \
         Modifier::PreventDamageToIt is combat-only and names no cause, \
         Effect::PreventNextDamage is a fixed amount with no cause filter, \
         and Effect::PreventNextFromChosenSource shields the ability's \
         controller rather than the chosen creature and takes a source the \
         player chooses rather than \"a spell or ability that targets it\""
    ),
    faces = &[face!(
        name = "Silhouette",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Choose target creature. If a spell or ability that
    // targets that creature would cause a source to deal damage to that
    // creature this turn, prevent that damage." — the prevention exists but
    // no shield carries the printed cause condition. The nearest pieces each
    // miss the sentence: `Modifier::PreventDamageToIt` is combat-only and
    // unfiltered, `Effect::PreventNextDamage` is a fixed amount that stops
    // the next damage whatever caused it, and
    // `Effect::PreventNextFromChosenSource` names the controller as the
    // protected player and asks for a source of the player's choice, not for
    // a spell or ability that targets the creature.
);
