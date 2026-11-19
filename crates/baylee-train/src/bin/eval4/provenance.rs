// The collector and converter must agree on these exact producer fields.
fn collector_policy_metadata(
    mode: PolicyMode4,
    temperature: f32,
    thinking_steps: u8,
    family: &str,
) -> Value {
    json!({"policy_mode":mode,"policy_contract":mode.contract(),"policy_provenance_version":2,
        "model_family":family,"temperature":temperature,"thinking_steps":thinking_steps,"sampling_version":2})
}

#[cfg(test)]
#[path = "../convert4/policy_provenance.rs"]
mod converter_policy_contract;

#[cfg(test)]
mod producer_tests {
    use super::*;
    #[test]
    fn actual_producer_fields_are_accepted_by_converter() {
        for mode in [PolicyMode4::Raw, PolicyMode4::Tactical] {
            let source = collector_policy_metadata(mode, 0.8, 2, "v5");
            assert!(converter_policy_contract::certified_run(&source));
            assert_eq!(source["temperature"], json!(0.8_f32));
            assert_eq!(source["thinking_steps"], 2);
            assert_eq!(source["sampling_version"], 2);
        }
        let invalid = collector_policy_metadata(PolicyMode4::Raw, 0.2, 2, "v5");
        assert!(!converter_policy_contract::certified_run(&invalid));
        let mut searched = collector_policy_metadata(PolicyMode4::Raw, 0.8, 2, "v5");
        searched["search"] = SearchConfig4::new(4, 2, 0).unwrap().metadata();
        assert!(
            !converter_policy_contract::certified_run(&searched),
            "a run that names its search certifies no policy-gradient dataset"
        );
    }

    /// The layer's answer as a search's rival: only a fair measurement of
    /// two candidates beside `--distill-labels`, on the raw net, and only
    /// then may labels and a search run together.
    #[test]
    fn the_layer_is_a_rival_only_in_a_fair_measurement_of_two() {
        let parse = |extra: &[&str]| {
            let mut argv = vec!["eval4", "--model", "m", "--out", "o", "--record-out", "r"];
            argv.extend_from_slice(extra);
            Args::try_parse_from(argv).unwrap()
        };
        let probe = [
            "--raw-policy",
            "--temperature",
            "0",
            "--distill-labels",
            "react",
            "--search",
            "2,6,96",
            "--search-fair",
            "--search-turns",
            "2",
            "--search-house-free",
            "--search-select",
            "measure",
            "--search-rival",
            "distill",
        ];
        let args = parse(&probe);
        assert_eq!(rival_refusal(&args), None);
        assert!(distill_probe(&args));
        let config = search_config(&args).unwrap().unwrap();
        assert_eq!(config.rival(), baylee_train::netplay4::search::Rival4::Distill);
        assert_eq!(config.selection(), Selection4::Measure);
        let metadata = config.metadata();
        // What convert4 admits a searched distillation run by
        // (`a_searched_run_is_admitted_only_as_a_fair_probe_of_the_layer`).
        assert_eq!(metadata["rival"], "distill");
        assert_eq!(metadata["selection"]["rule"], "measure");
        assert_eq!(
            metadata["decks"],
            "public_prior_from_the_seats_view_and_own_list_never_another_players_list"
        );
        assert_eq!(metadata["provenance"], "the_policys_own");

        let without = |flag: &str, value: bool| -> Vec<&str> {
            let mut out = Vec::new();
            let mut skip = false;
            for (at, arg) in probe.iter().enumerate() {
                if skip {
                    skip = false;
                    continue;
                }
                if *arg == flag {
                    skip = value && at + 1 < probe.len();
                    continue;
                }
                out.push(*arg);
            }
            out
        };
        assert!(rival_refusal(&parse(&without("--distill-labels", true))).is_some());
        assert!(rival_refusal(&parse(&without("--search-fair", false))).is_some());
        let paired = parse(&[without("--search-select", true), vec!["--search-select", "paired"]].concat());
        assert!(rival_refusal(&paired).is_some());
        assert!(!distill_probe(&paired), "labels never beside a search that plays");
        let four = parse(&[without("--search", true), vec!["--search", "4,6,96"]].concat());
        assert!(rival_refusal(&four).is_some());
        let guarded = parse(&[&probe[..], &["--choice-guard", "tactical"]].concat());
        assert!(rival_refusal(&guarded).is_some());
        let macros = parse(&[&probe[..], &["--search-macros"]].concat());
        assert!(rival_refusal(&macros).is_some());
        // Without a rival the old rules stand: labels refuse every search.
        let ranked = parse(&without("--search-rival", true));
        assert_eq!(rival_refusal(&ranked), None);
        assert!(!distill_probe(&ranked));
    }

    #[test]
    fn a_catalog_other_than_this_builds_is_accepted_only_explicitly() {
        let parse = |extra: &[&str]| {
            let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
            argv.extend_from_slice(extra);
            Args::try_parse_from(argv)
        };
        assert_eq!(catalog_policy(&parse(&[]).unwrap()), CatalogPolicy4::Exact);
        assert_eq!(
            catalog_policy(&parse(&["--catalog-superset"]).unwrap()),
            CatalogPolicy4::Superset
        );
        for drift in [
            &["--catalog-drift"][..],
            &["--catalog-superset", "--catalog-drift"],
        ] {
            assert_eq!(
                catalog_policy(&parse(drift).unwrap()),
                CatalogPolicy4::Drift
            );
        }
        assert!(parse(&["--catalog-superset", "--legacy-v3"]).is_err());
        assert!(parse(&["--catalog-drift", "--legacy-v3"]).is_err());
        let args = parse(&[]).unwrap();
        let contract = collector::Contract::new(&args, None).unwrap();
        assert_eq!(
            catalog_metadata(&args, &contract).unwrap(),
            None,
            "a run on this build's own catalog records nothing new"
        );
    }

    /// Only an evaluation binds another catalog: convert4 re-encodes with
    /// this build's, so a collection's probabilities would not match.
    #[test]
    fn a_catalog_other_than_this_builds_is_refused_outside_evaluation() {
        let parse = |extra: &[&str]| {
            let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
            argv.extend_from_slice(extra);
            Args::try_parse_from(argv).unwrap()
        };
        for catalog in ["--catalog-superset", "--catalog-drift"] {
            assert_eq!(catalog_refusal(&parse(&[catalog])), None, "{catalog}");
            assert_eq!(
                catalog_refusal(&parse(&["--collection-purpose", "evaluation", catalog])),
                None
            );
            for purpose in ["on-policy-training", "failure_terminal_research_v1"] {
                assert!(
                    catalog_refusal(&parse(&["--collection-purpose", purpose, catalog])).is_some(),
                    "{purpose} {catalog}"
                );
                assert_eq!(
                    catalog_refusal(&parse(&["--collection-purpose", purpose])),
                    None,
                    "{purpose} on this build's catalog"
                );
            }
        }
    }

    /// The identity and rule-table ablations measure the seat under test:
    /// read only by it, recorded, and refused outside an evaluation against
    /// the house.
    #[test]
    fn an_ablated_read_is_an_evaluation_of_the_seat_under_test_only() {
        let parse = |extra: &[&str]| {
            let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
            argv.extend_from_slice(extra);
            Args::try_parse_from(argv).unwrap()
        };
        assert_eq!(catalog_read(&parse(&[])), CatalogRead4::default());
        let read = catalog_read(&parse(&["--catalog-drift", "--leave-out-identity"]));
        assert_eq!(read.policy, CatalogPolicy4::Drift);
        assert!(read.leave_out_identity && !read.ablated_table);
        assert!(catalog_read(&parse(&["--ablated-rule-table"])).ablated_table);
        for ablation in ["--leave-out-identity", "--ablated-rule-table"] {
            assert_eq!(catalog_refusal(&parse(&[ablation])), None, "{ablation}");
            for purpose in ["on-policy-training", "failure_terminal_research_v1"] {
                assert!(
                    catalog_refusal(&parse(&["--collection-purpose", purpose, ablation])).is_some(),
                    "{purpose} {ablation}"
                );
            }
            assert!(
                catalog_refusal(&parse(&["--opponent-model", "x", ablation])).is_some(),
                "{ablation} with a neural opponent"
            );
            let argv = [
                "eval4",
                "--model",
                "m",
                "--out",
                "o",
                "--legacy-v3",
                ablation,
            ];
            assert!(Args::try_parse_from(argv).is_err(), "{ablation} with v3");
        }
        assert_eq!(
            catalog_refusal(&parse(&["--opponent-model", "x"])),
            None,
            "a neural opponent on this build's catalog"
        );
    }
}
