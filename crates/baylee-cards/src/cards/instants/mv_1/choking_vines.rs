//! Choking Vines — {X}{G} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: X target attacking creatures become blocked. Choking Vines deals 1 damage to each of those creatures. (This spell works on creatures that can't be blocked.)
//! Set: WTH #123 — Weatherlight | Scryfall ID: 6cc4a7ee-f6f0-454a-9074-5988fdee1f34 | Oracle ID: 3b6b565e-c0b5-48f9-b3e0-4d5d4b065402
// NOT SUPPORTED: Making attacking creatures become blocked without a blocker (CR 509.1h, triggering "becomes blocked" abilities and changing their blocked status for filters) is not yet a dedicated `Effect` variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHOKING_VINES,
    oracle_id = "3b6b565e-c0b5-48f9-b3e0-4d5d4b065402",
    scryfall_id = "6cc4a7ee-f6f0-454a-9074-5988fdee1f34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Making attacking creatures become blocked without a blocker (CR 509.1h, triggering 'becomes blocked' abilities and changing their blocked status for filters) is not yet a dedicated `Effect` variant; in play, Choking Vines can only be cast during the declare blockers step targeting X attacking creatures, causes each of those creatures to assign no combat damage this turn (`Modifier::AssignsNoCombatDamage`, matching CR 510.1c for a blocked creature with no blockers), and deals 1 damage to each of those creatures.",
    ),
    faces = &[face!(
        name = "Choking Vines",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AssignsNoCombatDamage,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE,
        ))),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers)),
    )],
);
