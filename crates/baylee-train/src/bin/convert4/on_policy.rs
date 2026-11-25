//! Role and source seals for deliberately collected on-policy games only.
//! Ordinary evaluations and archived roleless runs cannot be certified by a flag.
use anyhow::{Context as _, ensure};
use baylee_train::convert4::ProducerEncoder;
use baylee_train_ops::files;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) struct RunBinding {
    source_run_sha256: String,
    source_checkpoint_sha256: String,
    model: Value,
    opponent: Value,
    opponent_role: Value,
    source_files: Vec<(PathBuf, String)>,
}

#[derive(Clone, Debug)]
pub(super) struct GameBinding {
    run: RunBinding,
    source_game: String,
    record_sha256: String,
    seats: Vec<Value>,
    /// ε of the learner's rule-proposal exploration, when it explored.
    exploration: Option<f64>,
}

fn ordinary_contract(contract: &Value) -> anyhow::Result<()> {
    ensure!(
        contract["purpose"].is_null()
            || (matches!(
                contract["purpose"].as_str(),
                Some("evaluation" | "on_policy_training")
            ) && contract["version"] == 1),
        "unsupported collection purpose/version for ordinary conversion"
    );
    ensure!(
        contract["purpose"] != "failure_terminal_research_v1"
            && contract["research_target_contract"].is_null(),
        "failure-terminal research requires its own converter; ordinary targets cannot consume these records"
    );
    Ok(())
}

/// The first of the evaluator's modes a run was played with that changes a
/// categorical answer or takes its probability (`netplay4::decision`), with
/// what the run says of it: the plain sampler's draws are the only ones a
/// policy loss may read. `None` for a run that names no mode, or says of every
/// mode it names that it was off.
///
/// The names are `eval4`'s in `run.json`, and for the modes it refuses with
/// every collection and therefore never writes there (grouped decoding,
/// adaptive thinking, the search student) the ones its summary has, so that a
/// run that came to name one is refused. A run that does not name one of them
/// is still refused, game by game, at the first certified menu whose draw has
/// no probability ([`baylee_train::convert4::Asks4::certified`]).
pub(super) fn answer_mode(run: &Value) -> Option<String> {
    let dev = &run["dev_arms"];
    [
        ("choice_guard", &run["choice_guard"], json!("off")),
        ("tactics_rules", &run["tactics_rules"], json!("")),
        (
            "dev_arms.free_temperature",
            &dev["free_temperature"],
            json!(false),
        ),
        ("dev_arms.menu_payable", &dev["menu_payable"], json!(false)),
        ("random_policy", &run["random_policy"], json!(false)),
        (
            "opponent_legacy_v3",
            &run["opponent_legacy_v3"],
            json!(false),
        ),
        ("opponent_policy", &run["opponent_policy"], Value::Null),
        ("tactician", &run["tactician"], Value::Null),
        ("search", &run["search"], Value::Null),
        ("memory_reset", &run["memory_reset"], Value::Null),
        ("grouped_decode", &run["grouped_decode"], json!(false)),
        ("think_adaptive", &run["think_adaptive"], Value::Null),
        ("search_student", &run["search_student"], Value::Null),
    ]
    .into_iter()
    .find(|(_, said, off)| !said.is_null() && *said != off)
    .map(|(name, said, _)| format!("{name} {said}"))
}

/// The sampler, table and data contract an on-policy source must declare.
fn on_policy_contract(run: &Value) -> anyhow::Result<()> {
    let contract = &run["collection_contract"];
    ensure!(
        run["action_probability_contract"]
            == baylee_train_ops::admission::ACTION_PROBABILITY_CONTRACT,
        "on-policy source predates categorical action certification; recollect with the matching evaluator"
    );
    if let Some(mode) = answer_mode(run) {
        anyhow::bail!(
            "an on-policy source was played with {mode}, which is no draw of the plain sampler"
        );
    }
    ensure!(
        contract["version"] == 1
            && contract["precision"] == "float32"
            && contract["seat_rng_rule"] == "independent_policy_seed_xor_physical_seat_v1"
            && run["policy_mode"] == "raw"
            && run["policy_contract"] == "raw_neural_sampler2_v1"
            && run["policy_provenance_version"] == 2
            && run["sampling_version"] == 2
            && run["thinking_steps"] == 2
            && run["temperature"]
                .as_f64()
                .is_some_and(|t| (t - 0.7).abs() < 1e-6)
            && run["training_data"] == true
            && run["numbers_only"] == false
            && run["seats"] == 2
            && run["held_out_cards"] == false
            && run["holdout_tier"] == ""
            // What the evaluator always says of its modes, said.
            && [
                &run["choice_guard"],
                &run["tactics_rules"],
                &run["dev_arms"]["free_temperature"],
                &run["dev_arms"]["menu_payable"],
                &run["random_policy"],
                &run["opponent_legacy_v3"],
            ]
            .into_iter()
            .all(|said| !said.is_null())
            && (run["decks_manifest"].is_null() || run["deck_split"] == "train")
            && run["shadow_teacher"].is_null(),
        "invalid on-policy source contract"
    );
    // This converter re-encodes with its build's whole catalog: a run that
    // bound another observed with it, and its probabilities would not match.
    ensure!(
        run["catalog"].is_null(),
        "an on-policy source on another card catalog cannot be converted"
    );
    Ok(())
}

impl RunBinding {
    pub fn load(directory: &Path, run: &Value) -> anyhow::Result<Option<Self>> {
        let contract = &run["collection_contract"];
        ordinary_contract(contract)?;
        if contract["purpose"] != "on_policy_training" {
            return Ok(None);
        }
        on_policy_contract(run)?;
        ProducerEncoder::from_run(run).map_err(anyhow::Error::msg)?;
        let model = &contract["learner_model_identity"];
        ensure!(
            model == &run["model_identity"],
            "learner identity differs from runtime identity"
        );
        let source_checkpoint_sha256 = check_model(model)?;
        let opponent = &contract["opponent_model_identity"];
        if !opponent.is_null() {
            check_model(opponent)?;
            ensure!(
                opponent["encoder_version"] == model["encoder_version"],
                "opponent encoder mismatch"
            );
            ensure!(
                matches!(
                    contract["opponent_role"].as_str(),
                    Some("historical" | "self_play")
                ),
                "unknown neural opponent role"
            );
            if contract["opponent_role"] == "self_play" {
                ensure!(opponent == model, "self-play source differs");
            }
        }
        let source_run_sha256 = files::sha256(&directory.join("run.json"))?;
        let completion = files::read_json(&directory.join("run.complete.json"))?;
        let index_sha = files::sha256(&directory.join("games.jsonl"))?;
        ensure!(
            completion["version"] == 1
                && completion["source_run_sha256"] == source_run_sha256
                && completion["games_index_sha256"] == index_sha
                && completion["planned_deals"] == run["planned_deals"]
                && completion["recorded_deals"] == run["planned_deals"],
            "incomplete or changed on-policy source"
        );
        let count = run["planned_deals"]
            .as_u64()
            .context("missing planned deal count")?;
        let first = run["deal_offset"]
            .as_u64()
            .context("missing first deal index")?;
        let mut seen = BTreeSet::new();
        for line in fs::read_to_string(directory.join("games.jsonl"))?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let row: Value = serde_json::from_str(line)?;
            let index = row["i"].as_u64().context("missing source game index")?;
            ensure!(
                index >= first && index - first < count && seen.insert(index),
                "missing or duplicate source deal"
            );
        }
        ensure!(
            count > 0 && u64::try_from(seen.len())? == count,
            "missing source cases"
        );
        let source_files = ["run.json", "run.complete.json", "games.jsonl"]
            .into_iter()
            .map(|name| {
                let path = directory.join(name);
                Ok((path.clone(), files::sha256(&path)?))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Some(Self {
            source_run_sha256,
            source_checkpoint_sha256,
            model: model.clone(),
            opponent: opponent.clone(),
            opponent_role: contract["opponent_role"].clone(),
            source_files,
        }))
    }

    pub fn game(&self, row: &Value) -> anyhow::Result<GameBinding> {
        ensure!(
            row["complete"] == true
                && row["capped"] == false
                && row["fallback_answers"].as_u64() == Some(0)
                && row["refused_answers"].as_u64() == Some(0)
                && matches!(
                    row["outcome"]["kind"].as_str(),
                    Some("won" | "team_won" | "draw")
                ),
            "on-policy source contains capped, refused, fallback or incomplete game; no silent exclusions"
        );
        ensure!(
            row["source_run_sha256"] == self.source_run_sha256,
            "game bound to a different run"
        );
        let source_game = row["game"]
            .as_str()
            .context("missing source game key")?
            .to_owned();
        ensure!(!source_game.is_empty(), "empty source game key");
        let record_sha256 = digest(&row["record_sha256"])?;
        let focal = row["focal_seat"].as_u64().context("missing focal seat")?;
        let seats = row["actor_roles"]
            .as_array()
            .context("missing per-seat actor roles")?;
        ensure!(
            focal < 2
                && row["i"].as_u64().is_some_and(|index| index % 2 == focal)
                && seats.len() == 2,
            "only declared 1v1 role bindings are supported"
        );
        let mut seen = BTreeSet::new();
        for seat in seats {
            let index = seat["seat"].as_u64().context("missing actor seat")?;
            ensure!(
                index < 2 && seen.insert(index),
                "duplicate or invalid actor seat"
            );
            let (role, identity) = if index == focal {
                ("learner", &self.model)
            } else if self.opponent.is_null() {
                ("house", &self.opponent)
            } else {
                (
                    self.opponent_role.as_str().context("opponent role")?,
                    &self.opponent,
                )
            };
            ensure!(
                seat["role"] == role,
                "actor role differs from collector policy assignment"
            );
            if role == "house" {
                ensure!(
                    seat["checkpoint_sha256"].is_null(),
                    "house seat claims neural weights"
                );
            } else {
                ensure!(
                    seat["checkpoint_sha256"] == identity["source_checkpoint_sha256"]
                        && seat["thinking_steps"] == 2
                        && seat["temperature"]
                            .as_f64()
                            .is_some_and(|t| (t - 0.7).abs() < 1e-6),
                    "actor identity/depth/temperature mismatch"
                );
            }
        }
        Ok(GameBinding {
            run: self.clone(),
            source_game,
            record_sha256,
            seats: seats.clone(),
            exploration: None,
        })
    }
}

impl GameBinding {
    pub fn verify(&self, bytes: &[u8]) -> anyhow::Result<()> {
        ensure!(
            super::shadow::sha(bytes) == self.record_sha256,
            "executed record digest changed"
        );
        Ok(())
    }
    /// The game as converted, with ε when its learner explored.
    #[must_use]
    pub fn with_exploration(mut self, epsilon: Option<f64>) -> Self {
        self.exploration = epsilon;
        self
    }
    fn manifest(&self, game_id: u32) -> Value {
        let mut manifest = json!({"game_id":game_id,"complete":true,"capped":false,"fallback_answers":0,
            "refused_answers":0,"unencoded_decisions":0,"seats":self.seats,
            "source_run_sha256":self.run.source_run_sha256,"record_sha256":self.record_sha256,
            "source_game":self.source_game});
        if let Some(epsilon) = self.exploration {
            // Only exploring games name it, so other manifests stay as they were.
            manifest["behavior_mixture"] = json!({
                "contract": baylee_train_ops::exploration::CONTRACT, "epsilon": epsilon});
        }
        manifest
    }
}

pub(super) fn publish(
    root: &Path,
    meta: &Value,
    games: &[(u32, &GameBinding)],
) -> anyhow::Result<()> {
    ensure!(
        !games.is_empty()
            && meta["action_probability_contract"]
                == baylee_train_ops::admission::ACTION_PROBABILITY_CONTRACT,
        "empty or uncertified on-policy conversion"
    );
    let source = &games[0].1.run.source_checkpoint_sha256;
    let mut source_games = BTreeSet::new();
    let mut source_runs = BTreeSet::new();
    for (_, game) in games {
        ensure!(
            &game.run.source_checkpoint_sha256 == source,
            "mixed learner source checkpoints"
        );
        ensure!(
            source_games.insert((&game.run.source_run_sha256, &game.source_game)),
            "duplicate source game"
        );
        if source_runs.insert(&game.run.source_run_sha256) {
            for (path, sha) in &game.run.source_files {
                ensure!(
                    files::sha256(path)? == *sha,
                    "source metadata changed during conversion"
                );
            }
        }
    }
    let dataset_files = baylee_train_ops::admission::seal_dataset_files(root, meta)?;
    files::atomic_json(
        &root.join("collection.json"),
        &json!({"version":1,"purpose":"on_policy_training",
        "source_checkpoint_sha256":source,"precision":"float32","mode":"raw",
        "policy_provenance_version":2,"sampling_version":2,
        "action_probability_contract":baylee_train_ops::admission::ACTION_PROBABILITY_CONTRACT,
        "dataset_files":dataset_files,
        "games":games.iter().map(|(id,game)|game.manifest(*id)).collect::<Vec<_>>() }),
    )?;
    Ok(())
}

pub(super) fn digest(value: &Value) -> anyhow::Result<String> {
    let value = value.as_str().context("missing digest")?;
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "invalid digest"
    );
    Ok(value.to_owned())
}

pub(super) fn check_model(model: &Value) -> anyhow::Result<String> {
    for key in ["source_checkpoint_sha256", "onnx_sha256", "metadata_sha256"] {
        digest(&model[key])?;
    }
    for depth in ["0", "2"] {
        digest(&model["depth_graph_sha256"][depth])?;
    }
    ensure!(
        (model["model_family"] == "v4" && model["model_version"] == 3)
            || (model["model_family"] == "v5" && model["model_version"] == 5),
        "unknown model family"
    );
    ProducerEncoder::from_run(&json!({"model_identity":model})).map_err(anyhow::Error::msg)?;
    digest(&model["source_checkpoint_sha256"])
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        root: PathBuf,
        run: Value,
        row: Value,
    }
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("baylee-on-policy-{}", uuid::Uuid::now_v7()));
            fs::create_dir_all(&root).unwrap();
            let model = json!({"model_family":"v5","model_version":5,"encoder_version":7,
                "architecture":"baylee_ai_v5_relational_v2","source_checkpoint_sha256":"a".repeat(64),
                "onnx_sha256":"b".repeat(64),"metadata_sha256":"c".repeat(64),
                "depth_graph_sha256":{"0":"d".repeat(64),"2":"e".repeat(64)}});
            let run = json!({"policy_mode":"raw","policy_contract":"raw_neural_sampler2_v1",
                "policy_provenance_version":2,"sampling_version":2,"thinking_steps":2,"temperature":0.7,
                "action_probability_contract":baylee_train_ops::admission::ACTION_PROBABILITY_CONTRACT,
                "choice_guard":"off","tactics_rules":"","dev_arms":{"free_temperature":false,"menu_payable":false},
                "random_policy":false,"opponent_legacy_v3":false,"opponent_policy":null,"numbers_only":false,
                "training_data":true,"seats":2,"held_out_cards":false,"holdout_tier":"",
                "model_identity":model,"planned_deals":1,"deal_offset":0,
                "collection_contract":{"version":1,"purpose":"on_policy_training","precision":"float32",
                    "seat_rng_rule":"independent_policy_seed_xor_physical_seat_v1","learner_model_identity":model,
                    "opponent_model_identity":null,"opponent_role":null}});
            files::atomic_json(&root.join("run.json"), &run).unwrap();
            let source_sha = files::sha256(&root.join("run.json")).unwrap();
            let row = json!({"i":0,"game":"training-0","source_run_sha256":source_sha,
                "record_sha256":super::super::shadow::sha(b"executed"),"focal_seat":0,
                "complete":true,"capped":false,"fallback_answers":0,"refused_answers":0,
                "outcome":{"kind":"won","seat":1},
                "actor_roles":[{"seat":0,"role":"learner","checkpoint_sha256":"a".repeat(64),"thinking_steps":2,"temperature":0.7},
                    {"seat":1,"role":"house"}]});
            let mut bytes = serde_json::to_vec(&row).unwrap();
            bytes.push(b'\n');
            fs::write(root.join("games.jsonl"), bytes).unwrap();
            files::atomic_json(&root.join("run.complete.json"),&json!({"version":1,"source_run_sha256":source_sha,
                "games_index_sha256":files::sha256(&root.join("games.jsonl")).unwrap(),"planned_deals":1,"recorded_deals":1})).unwrap();
            Self { root, run, row }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn losing_games_bind_roles_records_and_source_seals_without_relabelling() {
        let fixture = Fixture::new();
        let run = RunBinding::load(&fixture.root, &fixture.run)
            .unwrap()
            .unwrap();
        let game = run.game(&fixture.row).unwrap();
        game.verify(b"executed").unwrap();
        assert!(game.verify(b"different").is_err());
        let manifest = game.manifest(17);
        assert_eq!(manifest["game_id"], 17);
        assert_eq!(manifest["seats"][0]["role"], "learner");
        assert_eq!(manifest["source_game"], "training-0");
        for field in [
            "capped",
            "fallback_answers",
            "refused_answers",
            "focal_seat",
        ] {
            let mut row = fixture.row.clone();
            row[field] = if field == "capped" {
                json!(true)
            } else {
                json!(1)
            };
            assert!(run.game(&row).is_err(), "{field}");
        }
        let mut row = fixture.row.clone();
        row["actor_roles"][0]["checkpoint_sha256"] = json!("f".repeat(64));
        assert!(run.game(&row).is_err());
        row = fixture.row.clone();
        row["actor_roles"][1]["role"] = json!("learner");
        assert!(run.game(&row).is_err());
        row = fixture.row.clone();
        row["source_run_sha256"] = json!("f".repeat(64));
        assert!(run.game(&row).is_err());
        fs::write(fixture.root.join("games.jsonl"), b"changed\n").unwrap();
        assert!(RunBinding::load(&fixture.root, &fixture.run).is_err());
    }

    #[test]
    fn evaluation_or_missing_completion_can_never_be_certified_by_conversion() {
        let fixture = Fixture::new();
        let mut evaluation = fixture.run.clone();
        evaluation["collection_contract"]["purpose"] = json!("evaluation");
        assert!(
            RunBinding::load(&fixture.root, &evaluation)
                .unwrap()
                .is_none()
        );
        fs::remove_file(fixture.root.join("run.complete.json")).unwrap();
        assert!(RunBinding::load(&fixture.root, &fixture.run).is_err());
    }

    #[test]
    fn a_precontract_rollout_or_an_action_modifier_cannot_certify() {
        let fixture = Fixture::new();
        let mut archived = fixture.run.clone();
        archived
            .as_object_mut()
            .unwrap()
            .remove("action_probability_contract");
        assert!(on_policy_contract(&archived).is_err());
        // Every mode the net's decision clears a probability on, under the
        // name the evaluator has for it.
        for (name, value) in [
            ("choice_guard", json!("react")),
            ("tactics_rules", json!("mass")),
            ("random_policy", json!(true)),
            ("opponent_legacy_v3", json!(true)),
            ("opponent_policy", json!("tactical")),
            (
                "tactician",
                json!({"record_contract": "tactician_records_v1"}),
            ),
            ("search", json!({"contract": "belief_lookahead_v5"})),
            ("memory_reset", json!("turn")),
            ("grouped_decode", json!(true)),
            ("think_adaptive", json!("2:0.9")),
            ("search_student", json!({"head": "h"})),
        ] {
            let mut modified = fixture.run.clone();
            modified[name] = value;
            let error = on_policy_contract(&modified).unwrap_err().to_string();
            assert!(error.contains(name), "{name}: {error}");
        }
        for arm in ["free_temperature", "menu_payable"] {
            let mut modified = fixture.run.clone();
            modified["dev_arms"][arm] = json!(true);
            let error = on_policy_contract(&modified).unwrap_err().to_string();
            assert!(error.contains(arm), "{error}");
        }
        // What the evaluator always says must be said, and data is no number.
        for name in [
            "choice_guard",
            "tactics_rules",
            "dev_arms",
            "random_policy",
            "opponent_legacy_v3",
            "numbers_only",
        ] {
            let mut unsaid = fixture.run.clone();
            unsaid.as_object_mut().unwrap().remove(name);
            assert!(on_policy_contract(&unsaid).is_err(), "{name}");
        }
        let mut numbers = fixture.run.clone();
        numbers["numbers_only"] = json!(true);
        assert!(on_policy_contract(&numbers).is_err());
        assert!(on_policy_contract(&fixture.run).is_ok());
    }

    #[test]
    fn an_on_policy_source_on_another_catalog_is_refused() {
        let fixture = Fixture::new();
        assert!(
            RunBinding::load(&fixture.root, &fixture.run)
                .unwrap()
                .is_some()
        );
        let mut drifted = fixture.run.clone();
        drifted["catalog"] = json!({"policy": "drift", "learner": {}, "opponent": null});
        let error = RunBinding::load(&fixture.root, &drifted).unwrap_err();
        assert!(error.to_string().contains("catalog"), "{error}");
    }

    #[test]
    fn research_target_semantics_never_fall_back_to_ordinary_conversion() {
        let fixture = Fixture::new();
        let mut research = fixture.run.clone();
        research["collection_contract"]["purpose"] = json!("failure_terminal_research_v1");
        research["collection_contract"]["version"] = json!(2);
        assert!(RunBinding::load(&fixture.root, &research).is_err());
        research["collection_contract"]["purpose"] = json!("evaluation");
        research["collection_contract"]["research_target_contract"] =
            json!({"name":"failure_terminal_research_v1"});
        assert!(RunBinding::load(&fixture.root, &research).is_err());
    }
}
