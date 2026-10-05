//! Shimian Night Stalker — {3}{B}{B} — Creature — Nightstalker
//! Oracle: {B}, {T}: All damage that would be dealt to you this turn by target attacking creature is dealt to this creature instead.
//! Set: CHR #36 — Chronicles | Scryfall ID: 9caf87f7-36d5-478b-9836-52043833290f | Oracle ID: 09b9e6fd-7a61-4ed4-a121-61b64fbf03f4
// PARTIAL — the whole ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIMIAN_NIGHT_STALKER,
    oracle_id = "09b9e6fd-7a61-4ed4-a121-61b64fbf03f4",
    scryfall_id = "9caf87f7-36d5-478b-9836-52043833290f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shimian Night Stalker",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTSTALKER],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "no filter can name the ability's chosen target as the damage source \
         of a RedirectDamageToYou, and the redirection verbs the DSL has \
         either move damage toward a player or run the other way"
    ),
    // NOT SUPPORTED: "{B}, {T}: All damage that would be dealt to you this
    // turn by target attacking creature is dealt to this creature instead."
    // — the standing shape is `Modifier::RedirectDamageToYou(&from)`
    // (Veteran Bodyguard), whose `from` filter is asked of the damage source,
    // but no `Filter` names an ability's target: the nearest,
    // `Filter::ATTACKING_CREATURE`, would redirect every attacking creature's
    // damage, not the chosen one. `Effect::RedirectNextFromChosenSource`
    // moves damage the other way (a chosen source's damage to a target
    // creature, redirected to you), and `Effect::RedirectNextDamage`'s
    // destination is a player, never a permanent.
);
