use super::*;
use crate as train;
use crate::failure_terminal::verify_episode;
use serde_json::json;
#[path = "../../../tests/support/research.rs"]
mod fixtures;

#[test]
fn certified_failure_streams_separate_learner_returns_and_unchanged_actor_provenance() {
    let fixture = fixtures::loop_episode(0);
    let proof = verify_episode(&fixture.record, &fixture.sidecar, &fixture.episode, 7, 0).unwrap();
    let mut rows = Vec::new();
    let result = stream_research4(&fixture.record, 1, &fixture.sidecar, &proof, |row| {
        rows.push(serde_json::to_value(row).unwrap());
        Ok(())
    })
    .unwrap();
    assert!(result.frames > 0 && result.actor_decisions > 0);
    assert_eq!(
        rows.iter()
            .filter(|row| row["training_terminal"] == true)
            .count(),
        1
    );
    assert_eq!(rows.last().unwrap()["training_terminal"], true);
    for row in &rows {
        assert_eq!(row["schema"], FRAME_CONTRACT);
        assert_eq!(row["returns"]["training_utility"], 0.0);
        assert!(row["returns"]["mtg_outcome"].is_null());
        for ordinary in ["outcome", "aux", "is_terminal"] {
            assert!(row["features"].get(ordinary).is_none());
        }
        if row["actor_eligible"] == true {
            assert_eq!(row["features"]["provenance"]["source"], "Net");
            assert_eq!(row["features"]["provenance"]["has_logprob"], true);
            assert!(
                fixture
                    .sidecar
                    .iter()
                    .any(|step| serde_json::to_value(step.provenance).unwrap()
                        == row["features"]["provenance"])
            );
        }
    }
    assert!(
        crate::convert4::stream_game4(&fixture.record, 1, true, Some(&fixture.sidecar), |_| Ok(()))
            .is_err()
    );
}

#[test]
fn unknown_opponent_failure_emits_no_targets_and_natural_results_remain_real() {
    let unknown = fixtures::loop_episode(1);
    let proof = verify_episode(&unknown.record, &unknown.sidecar, &unknown.episode, 7, 1).unwrap();
    assert!(proof.returns().training_utility.is_none());
    let result = stream_research4(&unknown.record, 2, &unknown.sidecar, &proof, |_| {
        panic!("unknown target row")
    })
    .unwrap();
    assert!(result.excluded_unknown);
    assert_eq!(result.frames, 0);
    for (learner, utility) in [(0, 1.0), (1, 0.0)] {
        let fixture = fixtures::natural_episode(learner);
        let proof = verify_episode(
            &fixture.record,
            &fixture.sidecar,
            &fixture.episode,
            7,
            learner,
        )
        .unwrap();
        assert_eq!(proof.returns().training_utility, Some(utility));
        assert_eq!(
            proof.returns().mtg_outcome,
            Some(json!({"kind":"won","seat":0}))
        );
        let mut scalars = Vec::new();
        stream_research4(&fixture.record, 3, &fixture.sidecar, &proof, |row| {
            let value = serde_json::to_value(row).unwrap();
            if value["features"]["head"] == 3 && value["features"]["is_decision"] == true {
                scalars.push(value);
            }
            Ok(())
        })
        .unwrap();
        if learner == 0 {
            assert!(!scalars.is_empty());
            assert!(scalars.iter().all(|row| row["actor_eligible"] == false
                && row["features"]["provenance"]["has_logprob"] == false));
        }
    }
}

#[test]
fn immutable_proofs_reject_target_boundary_role_record_and_sidecar_tampering() {
    let fixture = fixtures::loop_episode(0);
    for field in [
        "training_utility",
        "mtg_outcome",
        "target_contract",
        "executed_sample_bindings",
        "boundary",
    ] {
        let mut changed = fixture.episode.clone();
        changed[field] = json!("tampered");
        assert!(
            verify_episode(&fixture.record, &fixture.sidecar, &changed, 7, 0).is_err(),
            "{field}"
        );
    }
    assert!(verify_episode(&fixture.record, &fixture.sidecar, &fixture.episode, 7, 1).is_err());
    let proof = verify_episode(&fixture.record, &fixture.sidecar, &fixture.episode, 7, 0).unwrap();
    let mut changed = fixture.sidecar.clone();
    changed[0].provenance.behavior_logprob -= 1.0;
    assert!(stream_research4(&fixture.record, 1, &changed, &proof, |_| Ok(())).is_err());
    let mut record = fixture.record.clone();
    record[0] = b'!';
    assert!(stream_research4(&record, 1, &fixture.sidecar, &proof, |_| Ok(())).is_err());
    let mut false_natural = fixture.episode.clone();
    false_natural["termination_kind"] = json!("mtg_terminal");
    assert!(verify_episode(&fixture.record, &fixture.sidecar, &false_natural, 7, 0).is_err());
}

#[test]
fn reconstructed_menus_reject_forged_forced_sources_even_with_rebound_sidecars() {
    for mut fixture in [fixtures::loop_episode(0), fixtures::natural_episode(0)] {
        let index = fixture
            .sidecar
            .iter()
            .position(|step| {
                step.provenance.source == crate::features4::SourceKind4::Net
                    && step.provenance.has_logprob
            })
            .unwrap();
        let step = &mut fixture.sidecar[index];
        step.provenance.source = crate::features4::SourceKind4::Forced;
        step.provenance.has_logprob = false;
        step.provenance.behavior_logprob = 0.0;
        let binding = &mut fixture.episode["executed_sample_bindings"][index];
        binding["source"] = json!("Forced");
        binding["sidecar_sha256"] = json!(crate::failure_terminal::sha(
            &serde_json::to_vec(step).unwrap()
        ));
        let error =
            verify_episode(&fixture.record, &fixture.sidecar, &fixture.episode, 7, 0).unwrap_err();
        assert!(format!("{error:#}").contains("forcedness"), "{error:#}");
    }
}

#[test]
fn research_rejects_missing_original_multichoice_masks_instead_of_reencoding_them() {
    let mut fixture = fixtures::loop_episode(0);
    let index = fixture
        .sidecar
        .iter()
        .position(|step| !step.free_mask.is_empty())
        .unwrap();
    fixture.sidecar[index].free_mask.clear();
    fixture.episode["executed_sample_bindings"][index]["sidecar_sha256"] = json!(
        crate::failure_terminal::sha(&serde_json::to_vec(&fixture.sidecar[index]).unwrap())
    );
    let proof = verify_episode(&fixture.record, &fixture.sidecar, &fixture.episode, 7, 0).unwrap();
    let error =
        stream_research4(&fixture.record, 1, &fixture.sidecar, &proof, |_| Ok(())).unwrap_err();
    assert!(error.contains("mask/sequence"), "{error}");
}
