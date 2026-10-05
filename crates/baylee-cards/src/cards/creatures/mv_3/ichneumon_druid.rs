//! Ichneumon Druid — {1}{G}{G} — Creature — Human Druid
//! Oracle: Whenever an opponent casts an instant spell other than the first instant spell that player casts each turn, this creature deals 4 damage to that player.
//! Set: LEG #191 — Legends | Scryfall ID: cf2313bb-6f9b-49d6-b069-9f3b77b6e107 | Oracle ID: 6aba2a04-0117-429e-a5df-6ba4645129d8
// PARTIAL — the damage trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICHNEUMON_DRUID,
    oracle_id = "6aba2a04-0117-429e-a5df-6ba4645129d8",
    scryfall_id = "cf2313bb-6f9b-49d6-b069-9f3b77b6e107",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ichneumon Druid",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "the whole trigger is off the card: no `Trigger` fires on an \
         opponent's instant spell *other than the first that player casts \
         each turn* (`Trigger::SpellCast` fires on the first one too, and \
         `Trigger::NthSpellCast` counts the source controller's spells of \
         every type), and no `Condition` counts spells cast by the event's \
         player"
    ),
    // NOT SUPPORTED: "Whenever an opponent casts an instant spell other than
    // the first instant spell that player casts each turn, this creature deals
    // 4 damage to that player." — `Trigger::SpellCast(&Filter::HasType(
    // TypeSet::INSTANT))` would also fire on the first instant, and
    // `Trigger::NthSpellCast { n: 2, .. }` counts the ability controller's
    // spells (all types) rather than the caster's instants, with no player
    // relation at all; writing either would deal 4 damage on triggers the
    // card does not print.
    abilities = &[],
);
