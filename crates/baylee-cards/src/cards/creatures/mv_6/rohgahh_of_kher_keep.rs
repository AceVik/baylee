//! Rohgahh of Kher Keep — {2}{B}{B}{R}{R} — Legendary Creature — Kobold
//! Oracle: At the beginning of your upkeep, you may pay {R}{R}{R}. If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.
//! Oracle: Creatures you control named Kobolds of Kher Keep get +2/+2.
//! Set: ME3 #172 — Masters Edition III | Scryfall ID: 1429cc3d-f6a2-4011-b716-5c3e0cf251d0 | Oracle ID: 44412804-aa69-4e62-b118-123741d91914
// PARTIAL — the upkeep tax and the anthem are written; the control change is
// off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static KOBOLDS_OF_KHER_KEEP: Filter = Filter::Named("Kobolds of Kher Keep");

static ROHGAHH_AND_KOBOLDS: Filter = Filter::Or(&[
    Filter::This,
    Filter::And(&[Filter::CREATURE, KOBOLDS_OF_KHER_KEEP]),
]);

card!(
    index = index::ROHGAHH_OF_KHER_KEEP,
    oracle_id = "44412804-aa69-4e62-b118-123741d91914",
    scryfall_id = "1429cc3d-f6a2-4011-b716-5c3e0cf251d0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rohgahh of Kher Keep",
        mana_cost = mana!("{2}{B}{B}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::KOBOLD],
        power = Some(5),
        toughness = Some(5),
    ),],
    coverage = Coverage::Partial(
        "no effect changes control of every permanent a filter matches — \
         `Effect::ChangeController` hands over one object (the first target or \
         the source) to a `PlayerRel`, so \"then an opponent gains control of \
         them\" is off the card",
    ),
    // NOT SUPPORTED: "…then an opponent gains control of them." —
    // `Effect::ChangeController` changes one permanent, never every permanent
    // a filter matches, and it cannot ask which opponent; the tap half of the
    // sentence is written below, and the unpaid branch therefore leaves the
    // board tapped without the printed hand-over.
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{R}{R}{R}"),
                effect: &Effect::TapAll {
                    filter: &ROHGAHH_AND_KOBOLDS,
                },
            }],
        ),
        static_ability!(
            Filter::And(&[Filter::YOUR_CREATURE, KOBOLDS_OF_KHER_KEEP]),
            Modifier::ModifyPT(2, 2)
        ),
    ],
);
