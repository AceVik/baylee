use super::{
    Answer4, EncodedCandidates4, EncodedState4, HeuristicAgent, Pending, PlayerView, ShadowGame,
    SourceKind4, sha,
};
use crate::{CoverageMatrix4, EvaluationPlayer, play_one_table};
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_train::features4::{SeatKnowledge4, StepProvenance4, TeacherTarget4};
use baylee_train::features7::{encode_candidates7_grouped, encode_state7};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "baylee-shadow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        let run = json!({"policy_mode":"raw", "policy_provenance_version":2,
            "sampling_version":2, "policy_contract":"raw_neural_sampler2_v1",
            "model_identity":{"encoder_version":7,"source_checkpoint_sha256":"a".repeat(64)},
            "shadow_structure_sha256":"b".repeat(64)});
        std::fs::write(path.join("run.json"), serde_json::to_vec(&run).unwrap()).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

// Engine-backed collector plumbing without an ONNX session. This fixture is
// explicitly NOT evidence about learned policy skill or real stochastic LP.
#[derive(Default)]
struct FixturePolicy {
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    coverage: CoverageMatrix4,
    observations: u64,
}
impl EvaluationPlayer for FixturePolicy {
    fn reseed(&mut self, _: u64) {}
    fn init_game(&mut self, preset: &baylee_core::preset::GamePreset) {
        for s in 0..preset.seats.len() {
            let p = PlayerId::new(u8::try_from(s).unwrap());
            self.knowledge
                .insert(p.get(), SeatKnowledge4::from_preset(preset, p));
        }
    }
    fn uses_context(&self) -> bool {
        true
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.observations += 1;
        self.knowledge
            .get_mut(&view.seat.get())
            .unwrap()
            .observe(view);
        Ok(())
    }
    fn policy_encoding(
        &self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Option<(EncodedState4, EncodedCandidates4)> {
        let state = encode_state7(
            view,
            Some(pending),
            self.knowledge.get(&view.seat.get()),
            teams,
        );
        let menu = encode_candidates7_grouped(&state, view, pending);
        Some((state, menu))
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<Answer4>> {
        let action = HeuristicAgent::new(AIProfile::STEADY)
            .with_teams(teams.to_vec())
            .act(view, pending);
        Ok(Some(Answer4 {
            action,
            value: 0.5,
            target: TeacherTarget4::default(),
            provenance: StepProvenance4 {
                source: SourceKind4::Net,
                temperature: 1.,
                has_logprob: true,
                behavior_logprob: -0.125,
                thinking_steps: 2,
            },
        }))
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
}

#[test]
fn shadow_preserves_executed_record_and_binds_only_bc_labels() {
    let dir = Directory::new();
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let mut off = FixturePolicy::default();
    let mut on = FixturePolicy::default();
    let mut shadow = ShadowGame::new(&dir.0, "eval4-00000", 0, "sharp", 999).unwrap();
    let play = |policy: &mut FixturePolicy, labels| {
        play_one_table(
            "shadow-test",
            0,
            0,
            12,
            34,
            &[&deck, &deck],
            &[AIProfile::STEADY; 2],
            &[],
            &[None; 2],
            Some(policy),
            true,
            5000,
            true,
            labels,
        )
        .unwrap()
    };
    let a = play(&mut off, None);
    let b = play(&mut on, Some(&mut shadow));
    assert!(!b.1, "fixture must finish, not cap");
    assert_eq!(a.0, b.0);
    assert_eq!(
        a.7, b.7,
        "teacher query must not affect engine inputs/hashes"
    );
    assert_eq!(
        serde_json::to_value(a.9).unwrap(),
        serde_json::to_value(b.9).unwrap(),
        "behavior LP must stay unchanged"
    );
    assert_eq!(
        off.observations, on.observations,
        "no extra observe/H updates"
    );
    shadow.finish(&b.7, false).unwrap();
    let rows: Vec<Value> = std::fs::read_to_string(dir.0.join("shadow/eval4-00000.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows[0]["contract"]["history_mode"], "stateless_view_only");
    let labels: Vec<_> = rows.iter().filter(|v| v["kind"] == "label").collect();
    assert!(!labels.is_empty());
    assert!(
        labels
            .iter()
            .any(|v| v["teacher_action"] != v["executed_action"]),
        "teacher labels must remain separate even when advice differs"
    );
    assert!(labels.iter().all(|v| v["record_input_n"].is_u64()
        && v["record_input_hash"].is_string()
        && v["candidates_sha256"].as_str().unwrap().len() == 64
        && v["teacher_has_logprob"] == false
        && v["executed_provenance"]["behavior_logprob"] == -0.125));
    assert_eq!(rows.last().unwrap()["record_sha256"], sha(&b.7));
    assert_eq!(rows.last().unwrap()["ppo_certification"], false);
}

#[test]
fn incomplete_and_mismatched_records_never_publish_labels() {
    let dir = Directory::new();
    let shadow = ShadowGame::new(&dir.0, "capped", 0, "expert", 99).unwrap();
    shadow.finish(b"not a complete record", true).unwrap();
    assert!(!dir.0.join("shadow/capped.jsonl").exists());
    let mut shadow = ShadowGame::new(&dir.0, "mismatch", 0, "sharp", 99).unwrap();
    shadow
        .executed(
            Some(json!({"kind":"label", "record_byte_offset":3, "seat":0,
        "executed_action":{"kind":"pass_priority"}, "label_use":"bc_only",
        "teacher_source":"house", "teacher_has_logprob":false})),
            true,
        )
        .unwrap();
    assert!(
        shadow
            .finish(
                b"{}\n{\"kind\":\"input\",\"seat\":1,\"action\":null}\n{\"kind\":\"end\"}\n",
                false
            )
            .is_err()
    );
    assert!(!dir.0.join("shadow/mismatch.jsonl").exists());
    assert!(ShadowGame::new(&dir.0, "../escape", 0, "sharp", 99).is_err());
    assert!(ShadowGame::new(&dir.0, "unknown", 0, "future_teacher", 99).is_err());
}

#[test]
fn non_neural_and_unmappable_labels_are_excluded_and_old_runs_refused() {
    let dir = Directory::new();
    let mut shadow = ShadowGame::new(&dir.0, "exclusions", 0, "sharp", 99).unwrap();
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let preset =
        baylee_train::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "test").unwrap();
    let pid = PlayerId::new(0);
    session.take_over(pid);
    let (pending, view) = session.view_for_with_context(pid).unwrap();
    let state = encode_state7(&view, Some(&pending), None, &[None; 2]);
    let mut menu = encode_candidates7_grouped(&state, &view, &pending);
    let mut answer = Answer4 {
        action: HeuristicAgent::new(AIProfile::STEADY).act(&view, &pending),
        target: TeacherTarget4::default(),
        value: 0.5,
        provenance: StepProvenance4::default(),
    };
    assert!(
        shadow
            .propose(
                &view,
                &pending,
                &state,
                &menu,
                &answer,
                &[None; 2],
                0,
                session.record_pending()
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(shadow.counts.non_neural_exclusions, 1);
    answer.provenance.source = SourceKind4::Net;
    menu.candidates.clear();
    assert!(
        shadow
            .propose(
                &view,
                &pending,
                &state,
                &menu,
                &answer,
                &[None; 2],
                0,
                session.record_pending()
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(shadow.counts.mapping_failures, 1);
    assert_eq!(shadow.counts.labels, 0);
    let mut run: Value =
        serde_json::from_slice(&std::fs::read(dir.0.join("run.json")).unwrap()).unwrap();
    run["policy_provenance_version"] = json!(1);
    std::fs::write(dir.0.join("run.json"), serde_json::to_vec(&run).unwrap()).unwrap();
    assert!(ShadowGame::new(&dir.0, "historic", 0, "sharp", 99).is_err());
}
