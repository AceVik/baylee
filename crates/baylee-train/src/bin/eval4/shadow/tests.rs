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
            "model_identity":{"encoder_version":7,"model_version":5,"source_checkpoint_sha256":"a".repeat(64),
                "model_family":"v5","architecture":"baylee_ai_v5_relational_v2",
                "onnx_sha256":"c".repeat(64),"metadata_sha256":"d".repeat(64)},
            "shadow_teacher": super::metadata("sharp"),
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
        let (_, menu) = self.policy_encoding(view, pending, teams).unwrap();
        let forced = match menu.head {
            baylee_train::features4::HeadKind4::SingleChoice => menu.candidates.len() <= 1,
            baylee_train::features4::HeadKind4::SubsetPick => menu.candidates.is_empty(),
            baylee_train::features4::HeadKind4::PairPick => menu.pairs.is_empty(),
            baylee_train::features4::HeadKind4::ScalarValue => menu.min_choices >= menu.max_choices,
        };
        Ok(Some(Answer4 {
            action,
            value: 0.5,
            target: TeacherTarget4::default(),
            provenance: StepProvenance4 {
                source: if forced {
                    SourceKind4::Forced
                } else {
                    SourceKind4::Net
                },
                temperature: 1.,
                has_logprob: !forced,
                behavior_logprob: if forced { 0.0 } else { -0.125 },
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
    configure_export_catalog(&dir);
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
            None,
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
        serde_json::to_value(&a.9).unwrap(),
        serde_json::to_value(&b.9).unwrap(),
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
    verify_bc_conversion(&dir, &b.7, &b.9);
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

fn verify_bc_conversion(
    dir: &Directory,
    record: &[u8],
    behavior: &[baylee_train::convert4::RolloutStepSidecar4],
) {
    use baylee_train::convert4::{shadow_bc::ShadowBc4, stream_game4, stream_game4_shadow_bc};
    let run = std::fs::read(dir.0.join("run.json")).unwrap();
    let run_value: Value = serde_json::from_slice(&run).unwrap();
    let catalog = run_value["shadow_structure_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let sidecar = std::fs::read(dir.0.join("shadow/eval4-00000.jsonl")).unwrap();
    let mut labels = ShadowBc4::load(&run, &sidecar, record, "eval4-00000", &catalog).unwrap();
    let mut original = Vec::new();
    let mut bc = Vec::new();
    stream_game4(record, 1, true, Some(behavior), |step| {
        original.push(step);
        Ok(())
    })
    .unwrap();
    stream_game4_shadow_bc(record, 1, behavior, &mut labels, |step| {
        bc.push(step);
        Ok(())
    })
    .unwrap();
    assert_eq!(original.len(), bc.len(), "every K0/decision frame retained");
    let mut advice = 0;
    for (old, new) in original.iter().zip(&bc) {
        assert_eq!(
            (old.seat, old.step_cursor, old.turn, old.is_decision),
            (new.seat, new.step_cursor, new.turn, new.is_decision)
        );
        assert_eq!(
            serde_json::to_value(&old.state).unwrap(),
            serde_json::to_value(&new.state).unwrap()
        );
        assert_eq!(old.outcome, new.outcome);
        assert_eq!(old.aux, new.aux);
        assert_eq!(old.provenance.thinking_steps, new.provenance.thinking_steps);
        assert!(!new.provenance.has_logprob);
        if new.shadow_label.is_some() {
            advice += 1;
            assert_eq!(new.provenance.source, SourceKind4::House);
            assert_eq!(new.loss_weight, 1.0);
        } else {
            assert_eq!(new.loss_weight, 0.0);
        }
    }
    assert!(advice > 0);
    export_fixture(dir, record, behavior, &original, &bc);

    let mut rows: Vec<Value> = sidecar
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).unwrap())
        .collect();
    let index = rows.iter().position(|v| v["kind"] == "label").unwrap();
    let encode = |rows: &Vec<Value>| {
        rows.iter()
            .map(|v| serde_json::to_string(v).unwrap() + "\n")
            .collect::<String>()
            .into_bytes()
    };
    let original_rows = rows.clone();
    rows[index]["candidates_sha256"] = json!("f".repeat(64));
    let mut bad = ShadowBc4::load(&run, &encode(&rows), record, "eval4-00000", &catalog).unwrap();
    assert!(stream_game4_shadow_bc(record, 1, behavior, &mut bad, |_| Ok(())).is_err());
    rows = original_rows.clone();
    rows[index]["teacher_target"]["scalar_norm"] = json!(1.0);
    let mut bad = ShadowBc4::load(&run, &encode(&rows), record, "eval4-00000", &catalog).unwrap();
    assert!(
        stream_game4_shadow_bc(record, 1, behavior, &mut bad, |_| Ok(())).is_err(),
        "stored raw targets must never override reconstructed teacher action"
    );
    let mut changed_record = record.to_vec();
    changed_record.push(b'\n');
    assert!(ShadowBc4::load(&run, &sidecar, &changed_record, "eval4-00000", &catalog).is_err());
    rows = original_rows.clone();
    rows.insert(index, rows[index].clone());
    assert!(ShadowBc4::load(&run, &encode(&rows), record, "eval4-00000", &catalog).is_err());
    rows = original_rows.clone();
    rows.remove(index);
    assert!(ShadowBc4::load(&run, &encode(&rows), record, "eval4-00000", &catalog).is_err());
    rows = original_rows;
    rows.pop();
    assert!(ShadowBc4::load(&run, &encode(&rows), record, "eval4-00000", &catalog).is_err());
    assert!(ShadowBc4::load(&run, &sidecar, record, "wrong-source-game", &catalog).is_err());
}

// Explicit test-only export. Refuses overwrite and does not load ONNX or run
// production games. The actor remains the documented deterministic mock.
fn configure_export_catalog(dir: &Directory) {
    if std::env::var_os("BAYLEE_SHADOW_TEST_EXPORT").is_none() {
        return;
    }
    let catalog = baylee_train::cardstructure::catalog(
        &baylee_cards::all()
            .map(|c| c.index.get())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut run: Value =
        serde_json::from_slice(&std::fs::read(dir.0.join("run.json")).unwrap()).unwrap();
    run["shadow_structure_sha256"] = json!(baylee_train::cardstructure::digest(&catalog).unwrap());
    run["fixture_scope"] =
        json!("CPU engine plumbing with mock actor and stub behavior LP, not real ONNX sampling");
    std::fs::write(dir.0.join("run.json"), serde_json::to_vec(&run).unwrap()).unwrap();
}

fn export_fixture(
    dir: &Directory,
    record: &[u8],
    behavior: &[baylee_train::convert4::RolloutStepSidecar4],
    original: &[baylee_train::convert4::Step4],
    bc: &[baylee_train::convert4::Step4],
) {
    use std::io::Write as _;
    let Some(output) = std::env::var_os("BAYLEE_SHADOW_TEST_EXPORT") else {
        return;
    };
    let root = std::path::PathBuf::from(output);
    assert!(root.is_absolute(), "fixture export must use absolute path");
    std::fs::create_dir(&root).expect("preserve existing fixture; choose a new export path");
    let raw = root.join("raw-run");
    std::fs::create_dir_all(raw.join("records")).unwrap();
    std::fs::create_dir(raw.join("shadow")).unwrap();
    std::fs::copy(dir.0.join("run.json"), raw.join("run.json")).unwrap();
    std::fs::copy(
        dir.0.join("shadow/eval4-00000.jsonl"),
        raw.join("shadow/eval4-00000.jsonl"),
    )
    .unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gzip.write_all(record).unwrap();
    let compressed = gzip.finish().unwrap();
    std::fs::write(raw.join("records/fixture.jsonl.gz"), &compressed).unwrap();
    let end: Value = serde_json::from_slice(
        record
            .split(|b| *b == b'\n')
            .rfind(|l| !l.is_empty())
            .unwrap(),
    )
    .unwrap();
    let winners = end["winners"].as_array().unwrap();
    let outcome = match winners.as_slice() {
        [] => json!({"kind":"draw"}),
        [winner] => json!({"kind":"won","seat":winner}),
        _ => json!({"kind":"team_won","seats":winners}),
    };
    let index = json!({"game":"eval4-00000","i":0,"seed":12,"outcome":outcome,
        "shard":"fixture.jsonl.gz","offset":0,"len":compressed.len(),"rollout_sidecar":behavior});
    std::fs::write(
        raw.join("games.jsonl"),
        serde_json::to_string(&index).unwrap() + "\n",
    )
    .unwrap();
    for (name, steps) in [
        ("original-steps.jsonl.gz", original),
        ("bc-steps.jsonl.gz", bc),
    ] {
        let gzip = flate2::write::GzEncoder::new(
            std::fs::File::create(root.join(name)).unwrap(),
            flate2::Compression::fast(),
        );
        let mut output = std::io::BufWriter::new(gzip);
        for step in steps {
            serde_json::to_writer(&mut output, step).unwrap();
            writeln!(output).unwrap();
        }
        output.into_inner().unwrap().finish().unwrap();
    }
    std::fs::write(root.join("fixture.json"),serde_json::to_vec_pretty(&json!({"schema":1,
        "generator":"eval4 shadow::tests::shadow_preserves_executed_record_and_binds_only_bc_labels",
        "record_sha256":sha(record),"original_frames":original.len(),"bc_frames":bc.len(),
        "scope":"actual Rust writer/replay, mock actor, no ONNX/GPU"})).unwrap()).unwrap();
}
