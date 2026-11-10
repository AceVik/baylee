//! Kookus — {3}{R}{R} — Creature — Djinn
//! Oracle: Trample
//! Oracle: At the beginning of your upkeep, if you don't control a creature named Keeper of Kookus, this creature deals 3 damage to you and attacks this turn if able.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: VIS #86 — Visions | Scryfall ID: 8fb90922-99d2-4b36-9039-bb806fd01756 | Oracle ID: 4c8196e0-c392-4402-b77c-e3d009e9899c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_KEEPER_OF_KOOKUS: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::Named("Keeper of Kookus")]);

card!(
    index = index::KOOKUS,
    oracle_id = "4c8196e0-c392-4402-b77c-e3d009e9899c",
    scryfall_id = "8fb90922-99d2-4b36-9039-bb806fd01756",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kookus",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(3),
        toughness = Some(5),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::You),
                },
                Effect::ModifySource {
                    modifier: Modifier::AttacksEachCombat,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            condition = Some(Condition::ControlCountAtMost(&YOUR_KEEPER_OF_KOOKUS, 0)),
        ),
        activated!(
            cost!("{R}"),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 0),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
