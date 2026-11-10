//! Tolarian Entrancer — {1}{U} — Creature — Human Wizard
//! Oracle: Whenever this creature becomes blocked by a creature, gain control of that creature at end of combat.
//! Set: WTH #56 — Weatherlight | Scryfall ID: c29dd04a-b3aa-48b6-beef-3314344b84a6 | Oracle ID: 50c8c7e2-1b45-44b2-a50b-3b514d4c4c42
// IMPLEMENTED — Gains control of each creature that blocks it at end of combat.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOLARIAN_ENTRANCER,
    oracle_id = "50c8c7e2-1b45-44b2-a50b-3b514d4c4c42",
    scryfall_id = "c29dd04a-b3aa-48b6-beef-3314344b84a6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tolarian Entrancer",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::ModifyEvent {
                modifier: Modifier::GainControl,
                duration: Duration::Indefinitely,
            }],
        }],
    ),],
);
