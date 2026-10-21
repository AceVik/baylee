//! Winter's Chill — {X}{U} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: X can't be greater than the number of snow lands you control.
//! Oracle: Choose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat.
//! Set: ICE #107 — Ice Age | Scryfall ID: a779aca7-ff2c-48d8-9484-6ad04b2c6bcb | Oracle ID: ae78d498-11c7-427f-9d13-3f1b7096f803
// NOT SUPPORTED: capping X at the number of snow lands you control and having each target's controller choose whether to pay {1}, {2}, or neither to avoid end-of-combat destruction are not supported; prevents all combat damage that would be dealt to and dealt by the X target attacking creatures this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTER_S_CHILL,
    oracle_id = "ae78d498-11c7-427f-9d13-3f1b7096f803",
    scryfall_id = "a779aca7-ff2c-48d8-9484-6ad04b2c6bcb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "capping X at the number of snow lands you control and having each target's controller choose whether to pay {1}, {2}, or neither to avoid end-of-combat destruction are not supported; prevents all combat damage that would be dealt to and dealt by the X target attacking creatures this turn",
    ),
    faces = &[face!(
        name = "Winter's Chill",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        ))),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BeforeStep(StepKind::DeclareBlockers),
        ]))
    )],
);
