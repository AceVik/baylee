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

#[cfg(test)]
mod memory_reset_tests {
    use super::*;
    use baylee_train::netplay4::MemoryReset4;

    fn parse(extra: &[&str]) -> Result<Args, clap::Error> {
        let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv)
    }

    fn refusal(args: &Args) -> Option<&'static str> {
        memory_reset_refusal(args, &collector::Contract::new(args, None).unwrap())
    }

    #[test]
    fn without_the_flag_the_net_keeps_its_memory() {
        let args = parse(&[]).unwrap();
        assert_eq!(memory_reset(&args), MemoryReset4::Off);
        assert_eq!(refusal(&args), None);
        assert!(!args.decision_trace);
    }

    #[test]
    fn a_reset_names_its_point_and_measures_an_evaluation_only() {
        let args = parse(&["--memory-reset", "decision"]).unwrap();
        assert_eq!(memory_reset(&args), MemoryReset4::Decision);
        assert_eq!(refusal(&args), None);
        let args = parse(&["--memory-reset", "turn", "--raw-policy"]).unwrap();
        assert_eq!(memory_reset(&args), MemoryReset4::Turn);
        assert!(parse(&["--memory-reset", "game"]).is_err());
        for extra in [
            &["--search", "2,2,4"][..],
            &["--random-policy"],
            &["--shadow-teacher", "sharp"],
        ] {
            let mut argv = vec!["--memory-reset", "decision"];
            argv.extend_from_slice(extra);
            assert!(refusal(&parse(&argv).unwrap()).is_some(), "{extra:?}");
        }
        let mut args = parse(&["--memory-reset", "decision"]).unwrap();
        args.collection_purpose = collector::Purpose::OnPolicyTraining;
        let contract = collector::Contract {
            purpose: collector::Purpose::OnPolicyTraining,
            ..collector::Contract::new(&parse(&[]).unwrap(), None).unwrap()
        };
        assert!(
            memory_reset_refusal(&args, &contract).is_some(),
            "no collection"
        );
    }

    #[test]
    fn a_trace_needs_records_and_a_net_of_its_own() {
        assert!(parse(&["--decision-trace"]).is_err(), "needs --record-out");
        let args = parse(&["--decision-trace", "--record-out", "r"]).unwrap();
        assert!(args.decision_trace);
        assert_eq!(refusal(&args), None);
        let args = parse(&["--decision-trace", "--record-out", "r", "--search", "2,2,4"]).unwrap();
        assert!(refusal(&args).is_some());
    }
}
