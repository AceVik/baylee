//! Elder Spawn — {4}{U}{U}{U} — Creature — Spawn
//! Oracle: At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature and it deals 6 damage to you.
//! Oracle: This creature can't be blocked by red creatures.
//! Set: LEG #52 — Legends | Scryfall ID: 99cc045e-01a8-4f14-a86d-0a67ec35d6b7 | Oracle ID: 363b2213-4c70-4cb0-b895-81393c1082a7
// IMPLEMENTED — upkeep Island sacrifice or the Spawn dies and deals 6 to you; red creatures can't block it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELDER_SPAWN,
    oracle_id = "363b2213-4c70-4cb0-b895-81393c1082a7",
    scryfall_id = "99cc045e-01a8-4f14-a86d-0a67ec35d6b7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elder Spawn",
        mana_cost = mana!("{4}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPAWN],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Sacrifice(&Filter::And(&[
                    Filter::HasSubtype(subtypes::land::ISLAND),
                    Filter::ControlledByYou
                ])),
                effect: &Effect::Sequence(&[
                    Effect::SacrificeSelf,
                    Effect::DealDamage {
                        amount: Amount::Fixed(6),
                        target: TargetSpec::Player(PlayerRel::You)
                    }
                ])
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::CantBeBlockedBy(&Filter::And(&[
                Filter::CREATURE,
                Filter::HasColor(ColorSet::from_slice(&[Color::Red]))
            ]))
        ),
    ],
);
