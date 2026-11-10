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
mod exploration_tests {
    use super::*;

    fn on_policy(extra: &[&str]) -> Result<Args, clap::Error> {
        let mut argv = vec![
            "eval4",
            "--model",
            "m",
            "--out",
            "o",
            "--record-out",
            "r",
            "--collection-purpose",
            "on-policy-training",
            "--raw-policy",
            "--temperature",
            "0.7",
        ];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv)
    }

    fn identity() -> ModelIdentity {
        ModelIdentity {
            model_family: "v5",
            model_version: 5,
            encoder_version: 10,
            global_feat_width: None,
            critic_semantics: None,
            architecture: Some("baylee_ai_v5_relational_v2".into()),
            onnx_sha256: "a".repeat(64),
            depth_graph_sha256: std::collections::BTreeMap::from([
                (0, "b".repeat(64)),
                (2, "c".repeat(64)),
            ]),
            metadata_sha256: "d".repeat(64),
            source_checkpoint_sha256: Some("e".repeat(64)),
            precision: None,
        }
    }

    fn setup(
        args: &Args,
    ) -> anyhow::Result<
        Option<(
            baylee_train::netplay4::TacticRules4,
            baylee_train_ops::exploration::RunExploration,
        )>,
    > {
        let contract = collector::Contract::new(args, Some(&identity()))?;
        exploration_setup(args, &contract)
    }

    #[test]
    fn without_the_flags_a_collection_explores_nothing() {
        assert!(setup(&on_policy(&[]).unwrap()).unwrap().is_none());
    }

    #[test]
    fn measured_explores_with_the_bundle_and_says_so() {
        let args = on_policy(&["--explore-rules", "measured", "--explore-epsilon", "0.2"]).unwrap();
        let (rules, contract) = setup(&args).unwrap().unwrap();
        let mut names = rules.names();
        names.sort_unstable();
        let mut bundle: Vec<&str> = baylee_train_ops::measured::RULE_GROUPS.split(',').collect();
        bundle.sort_unstable();
        assert_eq!(names, bundle);
        assert_eq!(contract.rules, baylee_train_ops::measured::RULE_GROUPS);
        assert_eq!(contract.records, "exploration");
        assert_eq!(contract.labels, "outcomes_only");
        assert_eq!(contract.proposers.len(), 2 + rules.names().len());
        contract.validate().unwrap();
    }

    #[test]
    fn exploration_is_an_on_policy_collection_of_the_raw_learner_only() {
        for (extra, why) in [
            (&["--explore-rules", "mass"][..], "ε is required"),
            (&["--explore-epsilon", "0.2"][..], "groups are required"),
        ] {
            assert!(on_policy(extra).is_err(), "{why}");
        }
        for epsilon in ["0", "1", "1.5", "NaN"] {
            let args = on_policy(&["--explore-rules", "mass", "--explore-epsilon", epsilon]).unwrap();
            assert!(setup(&args).is_err(), "ε {epsilon}");
        }
        let args = on_policy(&["--explore-rules", "nope", "--explore-epsilon", "0.2"]).unwrap();
        assert!(setup(&args).is_err(), "an unknown group");
        let mut args = on_policy(&["--explore-rules", "mass", "--explore-epsilon", "0.2"]).unwrap();
        args.collection_purpose = collector::Purpose::Evaluation;
        assert!(setup(&args).is_err(), "an evaluation plays what it measures");
        let args = on_policy(&[
            "--explore-rules",
            "mass",
            "--explore-epsilon",
            "0.2",
            "--tactics-rules",
            "mass",
        ])
        .unwrap();
        assert!(exploration_setup(&args, &collector::Contract::new(&args, Some(&identity())).unwrap()).is_err());
    }
}
