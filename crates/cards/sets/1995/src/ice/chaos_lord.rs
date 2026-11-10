//! Chaos Lord — {4}{R}{R}{R} — Creature — Human
//! Oracle: First strike
//! Oracle: At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.
//! Oracle: This creature can attack as though it had haste unless it entered this turn.
//! Set: ICE #178 — Ice Age | Scryfall ID: ee245922-b380-4b2e-a43f-ab1ba8078943 | Oracle ID: 16d0aea2-4ca4-4a9d-8674-2863e085f514
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAOS_LORD,
    oracle_id = "16d0aea2-4ca4-4a9d-8674-2863e085f514",
    scryfall_id = "ee245922-b380-4b2e-a43f-ab1ba8078943",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chaos Lord",
        mana_cost = mana!("{4}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(7),
        toughness = Some(7),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            // The "if" follows the effect, so this is no intervening if (CR
            // 603.4): the ability always goes on the stack and targets, and
            // the permanents are counted only as it resolves.
            &[Effect::IfCondition {
                condition: Condition::AmountIsEven(Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::Battlefield,
                }),
                then: &[Effect::ChangeController {
                    new_controller: PlayerRel::TargetPlayerOrController,
                }],
                otherwise: &[],
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        ),
        // Only the attack restriction is relaxed. This is deliberately not
        // the Haste keyword, which would also enable activated {T} abilities.
        static_ability!(
            Filter::And(&[Filter::This, Filter::Not(&Filter::EnteredThisTurn)]),
            Modifier::AttacksAsThoughHaste
        ),
    ],
);
