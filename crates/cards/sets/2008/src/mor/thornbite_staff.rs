//! Thornbite Staff — {2} — Kindred Artifact — Shaman Equipment
//! Oracle: Equipped creature has "{2}, {T}: This creature deals 1 damage to any target" and "Whenever a creature dies, untap this creature."
//! Oracle: Whenever a Shaman creature enters, you may attach this Equipment to it.
//! Oracle: Equip {4}
//! Set: MOR #145 — Morningtide | Scryfall ID: c1ab3225-64a9-411e-b22b-1869e958b8e5 | Oracle ID: dae4815e-9025-4993-ab46-52a3f1a7219e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static THORNBITE_STAFF_PING: AbilityDef = activated!(
    cost!("{2}", TapSelf),
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

static SHAMAN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::SHAMAN),
]);

card!(
    index = index::THORNBITE_STAFF,
    oracle_id = "dae4815e-9025-4993-ab46-52a3f1a7219e",
    scryfall_id = "c1ab3225-64a9-411e-b22b-1869e958b8e5",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thornbite Staff",
        mana_cost = mana!("{2}"),
        types = TypeSet::KINDRED.union(TypeSet::ARTIFACT),
        subtypes = &[subtypes::creature::SHAMAN, subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{4}")),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&THORNBITE_STAFF_PING)
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::Dies(&Filter::CREATURE),
                effects: &[Effect::UntapSelf],
                target: None,
            }
        ),
        triggered!(
            Trigger::EntersBattlefield(&SHAMAN_CREATURE),
            &[Effect::MayDo {
                effects: &[Effect::AttachSelf {
                    target: TargetSpec::EventObject,
                }],
            }]
        ),
    ],
);
