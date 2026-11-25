//! What `injective_categorical_menu_v1` asks of a research conversion and of
//! its frames, checked on every system: the fixtures of [`super::tests`] need
//! Linux process identities.
use super::{rows, source};
use crate::admission::ACTION_PROBABILITY_CONTRACT;
use crate::research_contract::{FRAME_CONTRACT, contract};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

fn history() -> source::History {
    source::History {
        receipt: json!({"episode_id":1,"frames":4,"returns":returns()}),
        path: PathBuf::new(),
        learner: 0,
    }
}

fn returns() -> Value {
    json!({"termination_kind":"learner_announcement_loop","training_utility":0.0,
        "mtg_outcome":null,"bootstrap":false})
}

/// A decision the net answered at `head`, with `menu` items and pairs and a
/// probability claimed where `logprob` is one.
fn frame(head: u8, menu: usize, logprob: Option<f64>) -> Vec<u8> {
    let masks = if matches!(head, 1 | 2) {
        vec![1; menu]
    } else {
        Vec::new()
    };
    let row = json!({"schema":FRAME_CONTRACT,"returns":returns(),"training_terminal":false,
        "actor_eligible":logprob.is_some(),
        "features":{"game_id":1,"seat":0,"step_cursor":1,"turn":1,"is_decision":true,
        "loss_weight":1.0,"head":head,"pending_kind":0,
        "state":{"entity_feat":[],"entity_object_ids":[],"entity_group_members":[],
            "entity_card_ids":[],"entity_face_ids":[],"entity_token_ids":[],
            "entity_rule_refs":[],"entity_zone_ids":[],"entity_rel_controller":[],
            "seat_feat":vec![vec![0.0;64];8],"global_feat":vec![0.0;80],"edges":[]},
        "cand_feat":vec![vec![0.0;48];menu],"cand_entity_idx":vec![-1;menu],
        "cand_rel_seat_idx":vec![-1;menu],"cand_card_id":vec![1;menu],
        "cand_face_id":vec![0;menu],"cand_ability_idx":vec![-1;menu],
        "cand_rule_refs":vec![Vec::<Value>::new();menu],
        "pairs":if head == 2 {vec![[0, 0];menu]} else {vec![]},
        "target":{"single_idx":0,"multi_mask":masks,"free_mask":masks,"pick_sequence":[],
            "scalar_norm":0.5},
        "provenance":{"source":"Net","has_logprob":logprob.is_some(),
            "behavior_logprob":logprob.unwrap_or(0.0),"thinking_steps":2,"temperature":0.7}}});
    serde_json::to_vec(&row).unwrap()
}

fn flags(raw: &[u8]) -> anyhow::Result<u32> {
    rows::validate(raw, &history(), 7, 1, &BTreeMap::new()).map(|row| row.flags)
}

/// A single choice of two or more may claim a probability and is then an
/// actor choice; one whose menu is not certified, a subset, a pair and a
/// number claim none and are none.
#[test]
fn a_probability_is_claimed_for_a_single_choice_alone() {
    assert_eq!(flags(&frame(0, 2, Some(-0.7))).unwrap() & rows::ACTOR, 4);
    for head in [0, 1, 2, 3] {
        assert_eq!(flags(&frame(head, 2, None)).unwrap() & rows::ACTOR, 0);
    }
    for (head, menu) in [(0, 1), (1, 2), (2, 2), (3, 2)] {
        assert!(
            flags(&frame(head, menu, Some(-0.7))).is_err(),
            "head {head} of {menu}"
        );
    }
    let mut unclaimed: Value = serde_json::from_slice(&frame(1, 2, None)).unwrap();
    unclaimed["features"]["provenance"]["behavior_logprob"] = json!(-0.7);
    assert!(flags(&serde_json::to_vec(&unclaimed).unwrap()).is_err());
}

/// A conversion that does not name the contract is refused by name, with what
/// to do about it.
#[test]
fn a_conversion_without_the_contract_is_refused_and_says_to_reconvert() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let manifest = json!({"schema":"failure_terminal_research_conversion_v2",
        "status":"conversion_only_unadmitted",
        "attempt_denominator_contract":"failure_terminal_attempt_ledger_v1",
        "operational_abort_ledger_supported":true,"ppo_eligible":false,
        "ordinary_target_cache_compatible":false,"target_contract":contract(),
        "frame_contract":FRAME_CONTRACT,"preprocessing_version":1,"sampling_version":2,
        "policy_provenance_version":2,"encoder_version":7});
    let load = |manifest: &Value| {
        let path = root.join("research.manifest.json");
        crate::files::atomic_json(&path, manifest).unwrap();
        let expected = crate::files::sha256(&path).unwrap();
        source::Source::load(&root, &expected)
            .err()
            .expect("the manifest names no files")
            .to_string()
    };
    let error = load(&manifest);
    assert!(
        error.contains("reconvert") && error.contains(ACTION_PROBABILITY_CONTRACT),
        "{error}"
    );
    let mut named = manifest;
    named["action_probability_contract"] = json!(ACTION_PROBABILITY_CONTRACT);
    let error = load(&named);
    assert!(!error.contains("reconvert"), "{error}");
}
