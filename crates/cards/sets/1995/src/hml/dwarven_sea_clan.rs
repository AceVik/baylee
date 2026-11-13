//! Dwarven Sea Clan — {2}{R} — Creature — Dwarf
//! Oracle: {T}: Choose target attacking or blocking creature whose controller controls an Island. This creature deals 2 damage to that creature at end of combat. Activate only before the end of combat step.
//! Set: HML #71 — Homelands | Scryfall ID: 4cb722d9-1998-4912-a6f2-4ffa8d21311a | Oracle ID: 55ae9711-65ac-4166-8a51-1deeb10a8904
// IMPLEMENTED — {T}: 2 damage to the chosen attacking or blocking creature (its controller controls an Island) at end of combat, activated only before the end of combat step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
static TARGET_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
    Filter::ControllerControls(&ISLAND),
]);

card!(
    index = index::DWARVEN_SEA_CLAN,
    oracle_id = "55ae9711-65ac-4166-8a51-1deeb10a8904",
    scryfall_id = "4cb722d9-1998-4912-a6f2-4ffa8d21311a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Sea Clan",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::Object(&TARGET_CREATURE),
            effects: &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::EventObject,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&TARGET_CREATURE))),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BeforeStep(StepKind::CombatEnd),
        ]))
    )],
);
