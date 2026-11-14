use super::{cases, execution, observations, pacing, recipes};
use observations::{Observation, Observations, Outcome};
use recipes::{Input, Piece, Rarity};

fn observed() -> Observations {
    let suite = cases::suite();
    Observations {
        schema: 1,
        contract_revision: cases::CONTRACT_REVISION,
        implementation: "synthetic-comparator-self-test".into(),
        cases: suite
            .fusions
            .iter()
            .map(|case| Observation {
                id: case.id.clone(),
                result: Outcome::Fusion(case.expected.clone()),
            })
            .chain(suite.resales.iter().map(|case| Observation {
                id: case.id.clone(),
                result: Outcome::Resale(case.expected),
            }))
            .chain(suite.executions.iter().map(|case| Observation {
                id: case.id.clone(),
                result: Outcome::Execution(case.expected),
            }))
            .collect(),
    }
}

#[test]
fn reference_cases_include_success_and_refusal_and_match_the_oracles() {
    let suite = cases::suite();
    suite.check().unwrap();
    assert!(suite.fusions.iter().any(|c| c.expected.is_some()));
    assert!(suite.fusions.iter().any(|c| c.expected.is_none()));
    assert!(suite.executions.iter().any(|c| c.expected.is_some()));
    assert!(suite.executions.iter().any(|c| c.expected.is_none()));
    let mut broken = suite.clone();
    broken.fusions[0].expected.as_mut().unwrap().rarity = Rarity::Rare;
    assert!(broken.check().is_err());
    broken = suite.clone();
    broken.executions[0].id.clone_from(&suite.fusions[0].id);
    assert!(broken.check().is_err());
}

fn ingredients(rarity: Rarity, copies: usize, basis: u32, potions: Vec<u32>) -> Input {
    Input {
        pieces: vec![
            Piece {
                family: "heartwood".into(),
                definition: 1,
                rarity,
                gold_basis: basis
            };
            copies
        ],
        potion_bases: potions,
    }
}

#[test]
fn nine_common_path_preserves_paid_basis_and_a_sale_never_refunds_more() {
    let uncommon = recipes::quote(&ingredients(Rarity::Common, 3, 4, vec![])).unwrap();
    let rare = recipes::quote(&ingredients(
        uncommon.rarity,
        3,
        uncommon.gold_basis,
        vec![],
    ))
    .unwrap();
    let mythic = recipes::quote(&ingredients(rare.rarity, 1, rare.gold_basis, vec![24])).unwrap();
    assert_eq!(mythic.rarity, Rarity::Mythic);
    assert_eq!(mythic.gold_basis, 60);
    assert_eq!(recipes::resale(mythic.gold_basis, 100), 30);
    assert_eq!(recipes::resale(mythic.gold_basis, 25), 25);
    assert_eq!(recipes::resale(0, 100), 0);
    for basis in 0..=1000 {
        for cap in [0, 1, 25, 1000] {
            assert!(recipes::resale(basis, cap) <= basis / 2);
            assert!(recipes::resale(basis, cap) <= cap);
        }
    }
}

#[test]
fn all_small_ingredient_counts_preserve_recipe_value_and_required_potion() {
    let units = |r| match r {
        Rarity::Common => 1,
        Rarity::Uncommon => 3,
        Rarity::Rare | Rarity::Mythic => 9,
    };
    let mut accepted = 0;
    for rarity in [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Mythic,
    ] {
        for copies in 0..=9 {
            for potions in 0..=2 {
                let input = ingredients(rarity, copies, 4, vec![24; potions]);
                if let Some(output) = recipes::quote(&input) {
                    accepted += 1;
                    assert_eq!(units(output.rarity), units(rarity) * copies);
                    assert_eq!(output.gold_basis, (copies * 4 + potions * 24) as u32);
                    assert_eq!(potions == 1, output.rarity == Rarity::Mythic);
                }
            }
        }
    }
    assert_eq!(accepted, 3);
}

#[test]
fn invalid_recipe_identity_and_sum_overflow_refuse_without_mutation() {
    let mut input = ingredients(Rarity::Rare, 1, u32::MAX, vec![1]);
    let before = serde_json::to_value(&input).unwrap();
    assert!(recipes::quote(&input).is_none());
    assert_eq!(serde_json::to_value(&input).unwrap(), before);
    input.pieces[0].gold_basis = 26;
    input.pieces[0].definition = 0;
    assert!(recipes::quote(&input).is_none());
    input.pieces[0].definition = 1;
    input.pieces[0].family = "x".repeat(81);
    assert!(recipes::quote(&input).is_none());
}

#[test]
fn execution_selection_is_order_independent_and_never_uses_a_different_contract() {
    let suite = cases::suite();
    for case in suite.executions {
        let mut reversed = case.input.clone();
        reversed.installed.reverse();
        assert_eq!(execution::select(&reversed), case.expected);
    }
    let mut input = cases::suite().executions[3].input.clone();
    input.installed[2].contract.build = "new-build".into();
    assert_eq!(execution::select(&input), None);
    input
        .installed
        .push(cases::suite().executions[3].input.installed[2].clone());
    assert_eq!(execution::select(&input), Some(execution::Variant::Tower));
}

#[test]
fn self_test_observations_round_trip_and_compare_in_any_order() {
    let mut result = observed();
    result.cases.reverse();
    let bytes = serde_json::to_vec(&result).unwrap();
    let count = result.cases.len();
    assert_eq!(
        observations::compare(&cases::suite(), &observations::decode(&bytes).unwrap()).unwrap(),
        count
    );
}

#[test]
fn missing_extra_duplicate_and_wrong_observations_cannot_pass() {
    let suite = cases::suite();
    let original = observed();
    let mut changed = original.clone();
    changed.cases.pop();
    assert!(observations::compare(&suite, &changed).is_err());
    let mut changed = original.clone();
    changed.cases.push(original.cases[0].clone());
    assert!(observations::compare(&suite, &changed).is_err());
    let mut changed = original.clone();
    changed.cases[0] = original.cases[1].clone();
    assert!(observations::compare(&suite, &changed).is_err());
    let mut changed = original.clone();
    changed.cases[0].result = Outcome::Fusion(None);
    assert!(observations::compare(&suite, &changed).is_err());
    let mut changed = original;
    changed.cases[0].result = Outcome::Execution(None);
    assert!(observations::compare(&suite, &changed).is_err());
}

#[test]
fn unknown_cases_and_versions_fail_without_echoing_payload() {
    let mut result = observed();
    result.cases[0].id = "PRIVATE_PAYLOAD".into();
    let error = observations::compare(&cases::suite(), &result).unwrap_err();
    assert!(!format!("{error:#}").contains("PRIVATE_PAYLOAD"));
    let mut result = observed();
    result.contract_revision += 1;
    assert!(observations::compare(&cases::suite(), &result).is_err());
    let mut result = observed();
    result.schema = 2;
    assert!(observations::compare(&cases::suite(), &result).is_err());
}

#[test]
fn malformed_unknown_and_oversized_json_is_bounded_and_redacted() {
    for bytes in [
        b"".as_slice(),
        b"not JSON",
        &[0xff],
        &vec![b' '; 1024 * 1024 + 1],
    ] {
        assert!(observations::decode(bytes).is_err());
    }
    let mut value = serde_json::to_value(observed()).unwrap();
    value["PRIVATE_PAYLOAD"] = serde_json::json!("secret");
    let error = observations::decode(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert!(!format!("{error:#}").contains("PRIVATE_PAYLOAD"));
    value.as_object_mut().unwrap().remove("PRIVATE_PAYLOAD");
    value["cases"][0]["result"]["kind"] = serde_json::json!("PRIVATE_PAYLOAD");
    let error = observations::decode(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert!(!format!("{error:#}").contains("PRIVATE_PAYLOAD"));
}

#[test]
fn pacing_matches_independent_binomial_endpoints_and_stops_at_terminals() {
    let current = pacing::calculate(8, 5, 500).unwrap();
    let previous = pacing::calculate(10, 4, 500).unwrap();
    assert!((current.conquest_probability - 794.0 / 4096.0).abs() < 1e-12);
    assert!((previous.conquest_probability - 378.0 / 8192.0).abs() < 1e-12);
    assert_eq!(current.max_duels, 12);
    assert_eq!(current.stages[6].gross_gold_before_duel, 90);
    let certain = pacing::calculate(8, 5, 1000).unwrap();
    assert!((certain.expected_duels - 8.0).abs() < 1e-12);
    assert!(certain.stages[8].probability_of_reaching.abs() < 1e-12);
    assert!((pacing::calculate(8, 5, 0).unwrap().expected_duels - 5.0).abs() < 1e-12);
}

#[test]
fn pacing_probability_mass_reach_and_bounds_hold_across_inputs() {
    for wins in [1, 5, 8, 32] {
        for defeats in [1, 4, 5, 32] {
            let mut previous = 0.0;
            for chance in [0, 100, 500, 900, 1000] {
                let result = pacing::calculate(wins, defeats, chance).unwrap();
                assert!(
                    (result.conquest_probability + result.exhaustion_probability - 1.0).abs()
                        < 1e-12
                );
                assert!(result.conquest_probability + 1e-12 >= previous);
                previous = result.conquest_probability;
                assert!(result.expected_duels <= f64::from(result.max_duels) + 1e-12);
                for pair in result.stages.windows(2) {
                    assert!(
                        pair[1].probability_of_reaching <= pair[0].probability_of_reaching + 1e-12
                    );
                }
            }
        }
    }
    for (wins, defeats, chance) in [(0, 5, 500), (8, 0, 500), (33, 5, 500), (8, 5, 1001)] {
        assert!(pacing::calculate(wins, defeats, chance).is_err());
    }
}
