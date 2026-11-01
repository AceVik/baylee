//! Kjeldoran Home Guard — {3}{W} — Creature — Human Soldier
//! Oracle: At end of combat, if this creature attacked or blocked this combat, put a -0/-1 counter on this creature and create a 0/1 white Deserter creature token.
//! Set: ME2 #22 — Masters Edition II | Scryfall ID: af8fd3bc-2244-45c7-a271-ec982fbc13ff | Oracle ID: 5f7ae611-a78d-4535-9d76-755001bda34c
use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KJELDORAN_HOME_GUARD,
    oracle_id = "5f7ae611-a78d-4535-9d76-755001bda34c",
    scryfall_id = "af8fd3bc-2244-45c7-a271-ec982fbc13ff",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Home Guard",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer,
        },
        &[
            Effect::AddCounterSource {
                kind: CounterKind::Minus {
                    power: 0,
                    toughness: 1,
                },
                amount: Amount::Fixed(1),
            },
            Effect::CreateToken {
                token: &generated_tokens::DESERTER_0_1_WHITE,
            },
        ],
        condition = Some(Condition::AttackedOrBlockedThisCombat),
    )],
);
