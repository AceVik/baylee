//! Krovikan Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature.
//! Set: ME2 #102 — Masters Edition II | Scryfall ID: fa1e779a-7075-49f0-b62e-936ad554d036 | Oracle ID: 8fa93c01-f9ea-4d48-a040-418d1ca6be19
// NOT SUPPORTED: puts a creature dealt damage by Krovikan Vampire this turn onto the battlefield under your control when it dies rather than at the beginning of each end step, and does not sacrifice it when you lose control of Krovikan Vampire.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROVIKAN_VAMPIRE,
    oracle_id = "8fa93c01-f9ea-4d48-a040-418d1ca6be19",
    scryfall_id = "fa1e779a-7075-49f0-b62e-936ad554d036",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "puts a creature dealt damage by Krovikan Vampire this turn onto the battlefield under your control when it dies rather than at the beginning of each end step, and does not sacrifice it when you lose control of Krovikan Vampire",
    ),
    faces = &[face!(
        name = "Krovikan Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::EventObject,
            owner_control: false,
            counters: None,
        }],
    )],
);
