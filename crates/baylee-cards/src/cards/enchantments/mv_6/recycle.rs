//! Recycle — {4}{G}{G} — Enchantment
//! Oracle: Skip your draw step.
//! Oracle: Whenever you play a card, draw a card.
//! Oracle: Your maximum hand size is two.
//! Set: TPR #189 — Tempest Remastered | Scryfall ID: 35698c86-4552-4c04-8f51-099bb2dbce6d | Oracle ID: 8cef3ce7-8fbc-4e68-93f9-d6adfc2c2cbf
// PARTIAL: draws a card whenever you play a land or cast a spell; skipping your draw step and setting your maximum hand size to two are not in Modifier.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECYCLE,
    oracle_id = "8cef3ce7-8fbc-4e68-93f9-d6adfc2c2cbf",
    scryfall_id = "35698c86-4552-4c04-8f51-099bb2dbce6d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "draws a card whenever you play a land or cast a spell; skipping your draw step and setting your maximum hand size to two are not in Modifier",
    ),
    faces = &[face!(
        name = "Recycle",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(Trigger::PlaysLand(PlayerRel::You), &[Effect::draw(1)]),
        triggered!(
            Trigger::SpellCast(&Filter::ControlledByYou),
            &[Effect::draw(1)],
        ),
    ],
);
