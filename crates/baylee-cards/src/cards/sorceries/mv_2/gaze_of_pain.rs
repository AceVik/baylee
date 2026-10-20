//! Gaze of Pain — {1}{B} — Sorcery
//! Oracle: Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature. If you do, it assigns no combat damage this turn.
//! Set: ICE #128 — Ice Age | Scryfall ID: 48401643-ec4b-444a-8f9a-1a5ea471ff4a | Oracle ID: b3a479d4-3e36-496b-abf8-b83231628d76
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Until end of turn, whenever a creature you control attacks and isn't blocked" —
// modeled by granting the unblocked-attacker trigger to creatures you control on resolution.
card!(
    index = index::GAZE_OF_PAIN,
    oracle_id = "b3a479d4-3e36-496b-abf8-b83231628d76",
    scryfall_id = "48401643-ec4b-444a-8f9a-1a5ea471ff4a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "granted unblocked-attacker trigger locked to creatures controlled on resolution"
    ),
    faces = &[face!(
        name = "Gaze of Pain",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::YOUR_CREATURE,
        Modifier::GrantTriggered {
            trigger: Trigger::AttacksAndIsntBlocked(&Filter::This),
            effects: &[Effect::MayDo {
                effects: &[
                    Effect::EventObjectDealsDamageEqualToPower {
                        target: TargetSpec::Object(&Filter::CREATURE),
                    },
                    Effect::ModifySource {
                        modifier: Modifier::AssignsNoCombatDamage,
                        duration: Duration::UntilEndOfTurn,
                    },
                ],
            }],
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
        },
        Duration::UntilEndOfTurn,
    )])],
);
