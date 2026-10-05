//! Haunting Wind — {3}{B} — Enchantment
//! Oracle: Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost, this enchantment deals 1 damage to that artifact's controller.
//! Set: ATQ #17 — Antiquities | Scryfall ID: a2f6ef2f-a3a2-4e1f-b7eb-59abc8414114 | Oracle ID: cbd2d38b-6402-44b7-ba27-d854268e31d0
// PARTIAL — the "becomes tapped" half is written; the "activates an ability
// without {T}" half is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAUNTING_WIND,
    oracle_id = "cbd2d38b-6402-44b7-ba27-d854268e31d0",
    scryfall_id = "a2f6ef2f-a3a2-4e1f-b7eb-59abc8414114",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no trigger hears an activated ability being activated: only \
         `Trigger::TappedForMana` observes an activation, and only of a mana \
         ability that tapped its source, so the {T}-less-activation half is off"
    ),
    faces = &[face!(
        name = "Haunting Wind",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "or a player activates an artifact's ability without {T}
    // in its activation cost" — no `Trigger` variant hears an activated
    // ability; `Trigger::TappedForMana` is mana abilities only and requires the
    // activation to have tapped the source.
    abilities = &[triggered!(
        Trigger::BecomesTapped(&Filter::ARTIFACT),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::ControllerOfEvent)
        }]
    )],
);
