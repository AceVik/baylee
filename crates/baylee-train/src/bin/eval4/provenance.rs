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
                    catalog_refusal(&parse(&["--collection-purpose", purpose, catalog]))
                        .is_some(),
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
}
