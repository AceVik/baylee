//! Choking Vines — {X}{G} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: X target attacking creatures become blocked. Choking Vines deals 1 damage to each of those creatures. (This spell works on creatures that can't be blocked.)
//! Set: WTH #123 — Weatherlight | Scryfall ID: 6cc4a7ee-f6f0-454a-9074-5988fdee1f34 | Oracle ID: 3b6b565e-c0b5-48f9-b3e0-4d5d4b065402
// PARTIAL — X target attacking creatures take 1 damage during the declare blockers step; making them become blocked is not supported and omitted.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHOKING_VINES,
    oracle_id = "3b6b565e-c0b5-48f9-b3e0-4d5d4b065402",
    scryfall_id = "6cc4a7ee-f6f0-454a-9074-5988fdee1f34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "X target attacking creatures take 1 damage during the declare blockers step; making them become blocked is not supported and omitted."
    ),
    faces = &[face!(
        name = "Choking Vines",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            // NOT SUPPORTED: X target attacking creatures become blocked.
            // Suppressing combat damage would also stop trample, unlike becoming blocked.
            Effect::DealDamageToMatchingTargets {
                amount: Amount::Fixed(1),
                filter: &Filter::ATTACKING_CREATURE,
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE,
        ))),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers)),
    )],
);
