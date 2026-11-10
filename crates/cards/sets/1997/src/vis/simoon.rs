//! Simoon — {R}{G} — Instant
//! Oracle: Simoon deals 1 damage to each creature target opponent controls.
//! Set: INV #272 — Invasion | Scryfall ID: 84b1930d-2e4b-472f-98a9-008fd632f3be | Oracle ID: 128e201a-f520-4917-a1a4-2f3836f1f92d
// PARTIAL: with exactly one opponent, the targeted opponent's creatures are dealt 1 damage each; with two or more opponents it deals no damage, because damage to "each creature target opponent controls" cannot be scoped to the targeted opponent alone (the available filter covers every opponent's creatures, which would be wider than printed).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIMOON,
    oracle_id = "128e201a-f520-4917-a1a4-2f3836f1f92d",
    scryfall_id = "84b1930d-2e4b-472f-98a9-008fd632f3be",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Partial(
        "with exactly one opponent, the targeted opponent's creatures are dealt 1 damage each; with two or more opponents it deals no damage, because damage to \"each creature target opponent controls\" cannot be scoped to the targeted opponent alone (the available filter covers every opponent's creatures, which would be wider than printed)",
    ),
    faces = &[face!(
        name = "Simoon",
        mana_cost = mana!("{R}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfCondition {
            // NOT SUPPORTED: scoping the damage to the creatures of the targeted opponent alone.
            // With one opponent, that opponent is the target, so the creatures of "each opponent"
            // are exactly the printed ones (2 minus the number of opponents is at least 1 only
            // with one or none); with more, the spell would hit creatures it was not aimed at.
            condition: Condition::AmountAtLeast(
                Amount::ConstantMinus {
                    constant: 2,
                    subtract: &Amount::Players(PlayerRel::EachOpponent),
                },
                1,
            ),
            then: &[Effect::damage_each(1, &Filter::OPPONENT_CREATURE)],
            otherwise: &[],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
