//! No Quarter — {3}{R} — Enchantment
//! Oracle: Whenever a creature becomes blocked by a creature with lesser power, destroy the blocking creature.
//! Oracle: Whenever a creature blocks a creature with lesser power, destroy the attacking creature.
//! Set: TMP #193 — Tempest | Scryfall ID: 7c317ee7-ed92-4e3e-92bb-502099caccf8 | Oracle ID: d238a5c1-f992-4f89-851a-de75b59f6793
// PARTIAL: grants creatures triggered abilities that destroy any creature with lesser power that blocks or is blocked by them; triggering from No Quarter itself rather than as a granted ability on the creatures is not in Trigger.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NO_QUARTER,
    oracle_id = "d238a5c1-f992-4f89-851a-de75b59f6793",
    scryfall_id = "7c317ee7-ed92-4e3e-92bb-502099caccf8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "grants creatures triggered abilities that destroy any creature with lesser power that blocks or is blocked by them; triggering from No Quarter itself rather than as a granted ability on the creatures is not in Trigger",
    ),
    faces = &[face!(
        name = "No Quarter",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(
            Filter::CREATURE,
            Modifier::GrantTriggered {
                trigger: Trigger::BecomesBlockedBy(&Filter::PowerLessThanSourcePower),
                effects: &[Effect::destroy(TargetSpec::EventObject)],
                target: None,
            },
        ),
        static_ability!(
            Filter::CREATURE,
            Modifier::GrantTriggered {
                trigger: Trigger::BlocksCreature(&Filter::PowerLessThanSourcePower),
                effects: &[Effect::destroy(TargetSpec::EventObject)],
                target: None,
            },
        ),
    ],
);
