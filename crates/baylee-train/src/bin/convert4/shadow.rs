//! Explicit separate BC output; never changes source records or model inputs.
use anyhow::{Context, ensure};
use baylee_train::convert4::{Step4, shadow_bc::ShadowBc4};
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct Source {
    run_path: PathBuf,
    labels_path: PathBuf,
    run_sha256: String,
    labels_sha256: String,
    record_sha256: String,
    game: String,
}

pub(super) fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut result = String::new();
    for byte in Sha256::digest(bytes) {
        write!(result, "{byte:02x}").expect("String");
    }
    result
}

impl Source {
    pub(super) fn namespace_key(&self) -> (String, String, String) {
        (
            self.run_sha256.clone(),
            self.game.clone(),
            self.record_sha256.clone(),
        )
    }
    pub(super) fn load(dir: &Path, game: &str) -> anyhow::Result<Self> {
        ensure!(
            !game.is_empty() && game.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "Invalid source game key"
        );
        let run_path = dir.join("run.json").canonicalize()?;
        let labels_path = dir
            .join("shadow")
            .join(format!("{game}.jsonl"))
            .canonicalize()
            .context("Completed shadow labels required for every selected game")?;
        ensure!(
            labels_path.starts_with(dir.canonicalize()?),
            "Shadow path escapes run"
        );
        ensure!(
            fs::metadata(&labels_path)?.len() <= 64 * 1024 * 1024
                && fs::metadata(&run_path)?.len() <= 16 * 1024 * 1024,
            "Shadow source exceeds bounded game metadata size"
        );
        let run = fs::read(&run_path)?;
        let labels = fs::read(&labels_path)?;
        let footer: Value = serde_json::from_slice(
            labels
                .split(|b| *b == b'\n')
                .rfind(|l| !l.is_empty())
                .context("Missing shadow footer")?,
        )?;
        ensure!(
            footer["kind"] == "completed",
            "Only completed shadow games are eligible"
        );
        let record_sha256 = footer["record_sha256"]
            .as_str()
            .context("Record SHA")?
            .to_owned();
        Ok(Self {
            run_path,
            labels_path,
            run_sha256: sha(&run),
            labels_sha256: sha(&labels),
            record_sha256,
            game: game.into(),
        })
    }
    pub(super) fn verify(&self, record: &[u8], catalog: &str) -> Result<ShadowBc4, String> {
        if fs::metadata(&self.labels_path)
            .map_err(|e| e.to_string())?
            .len()
            > 64 * 1024 * 1024
            || fs::metadata(&self.run_path)
                .map_err(|e| e.to_string())?
                .len()
                > 16 * 1024 * 1024
        {
            return Err("Shadow source exceeds bounded game metadata size".into());
        }
        let run = fs::read(&self.run_path).map_err(|e| e.to_string())?;
        let labels = fs::read(&self.labels_path).map_err(|e| e.to_string())?;
        if sha(&run) != self.run_sha256 || sha(&labels) != self.labels_sha256 {
            return Err("Shadow source changed".into());
        }
        ShadowBc4::load(&run, &labels, record, &self.game, catalog)
    }
    pub(super) fn metadata(&self, id: u32, shard: &Path, offset: u64, len: u64) -> Value {
        json!({"converted_game_id":id,"source_namespace":self.run_sha256,"source_game":self.game,
            "run_path":self.run_path,"run_sha256":self.run_sha256,"labels_path":self.labels_path,
            "labels_sha256":self.labels_sha256,"record_shard_path":shard,"record_offset":offset,"record_len":len,"record_sha256":self.record_sha256,
            "converted_labels":format!("shadow-bc/game-{id:06}.jsonl")})
    }
}

pub(super) struct FrameLabels {
    temporary: PathBuf,
    path: PathBuf,
    writer: BufWriter<File>,
    count: u64,
}
impl FrameLabels {
    #[cfg(test)]
    pub(super) fn new(root: &Path, id: u32, source: &Source) -> anyhow::Result<Self> {
        Self::new_with_encoder(root, id, source, 7)
    }
    pub(super) fn new_with_encoder(
        root: &Path,
        id: u32,
        source: &Source,
        encoder: u32,
    ) -> anyhow::Result<Self> {
        let directory = root.join("shadow-bc");
        fs::create_dir_all(&directory)?;
        let path = directory.join(format!("game-{id:06}.jsonl"));
        let temporary = path.with_extension("partial");
        ensure!(!path.exists(), "Preserving converted labels");
        let mut writer = BufWriter::new(
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?,
        );
        let mut header = json!({"kind":"header","schema":1,"converted_game_id":id,
            "source_namespace":source.run_sha256,"source_game":source.game,"run_sha256":source.run_sha256,
            "record_sha256":source.record_sha256,"original_labels_sha256":source.labels_sha256,"label_use":"bc_only","ppo_eligible":false});
        if encoder == 8 {
            header["encoder_version"] = json!(8);
        }
        serde_json::to_writer(&mut writer, &header)?;
        writeln!(writer)?;
        Ok(Self {
            temporary,
            path,
            writer,
            count: 0,
        })
    }
    pub(super) fn push(&mut self, step: &Step4) -> Result<(), String> {
        if let Some(label) = &step.shadow_label {
            let value = json!({"kind":"label","seat":step.seat,"step_cursor":step.step_cursor,
                "target":step.target,"provenance":label});
            serde_json::to_writer(&mut self.writer, &value).map_err(|e| e.to_string())?;
            writeln!(self.writer).map_err(|e| e.to_string())?;
            self.count += 1;
        }
        Ok(())
    }
    pub(super) fn finish(mut self) -> anyhow::Result<()> {
        serde_json::to_writer(
            &mut self.writer,
            &json!({"kind":"completed","labels":self.count,"ppo_eligible":false}),
        )?;
        writeln!(self.writer)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        fs::hard_link(&self.temporary, &self.path)?;
        fs::remove_file(self.temporary)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameLabels, Source, sha};
    use baylee_engine::choice::PlayerAction;
    use baylee_train::convert4::{AuxTargets4, Step4, shadow_bc::ShadowLabelProvenance4};
    use baylee_train::features4::{EncodedState4, SourceKind4, StepProvenance4, TeacherTarget4};
    use std::sync::atomic::{AtomicU64, Ordering};
    #[test]
    fn companion_retains_namespace_and_per_frame_label_provenance_without_binary_changes() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "baylee-shadow-bc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        assert!(Source::load(&directory, "missing").is_err());
        let source = Source {
            run_path: directory.join("run.json"),
            labels_path: directory.join("input.jsonl"),
            run_sha256: sha(b"run"),
            labels_sha256: sha(b"sidecar"),
            record_sha256: sha(b"record"),
            game: "source-17".into(),
        };
        let label = ShadowLabelProvenance4 {
            run_sha256: source.run_sha256.clone(),
            source_game: source.game.clone(),
            record_sha256: source.record_sha256.clone(),
            record_input_n: 42,
            teacher: "sharp".into(),
            teacher_action: PlayerAction::PassPriority,
            executed_action: PlayerAction::PassPriority,
            executed_provenance: StepProvenance4 {
                source: SourceKind4::Net,
                thinking_steps: 2,
                ..StepProvenance4::default()
            },
        };
        let step = Step4 {
            game_id: 1,
            seat: 0,
            step_cursor: 91,
            turn: 4,
            is_decision: true,
            loss_weight: 1.,
            is_terminal: false,
            state: EncodedState4::default(),
            head: 0,
            pending_kind: 0,
            cand_feat: vec![],
            cand_entity_idx: vec![],
            cand_rel_seat_idx: vec![],
            cand_card_id: vec![],
            cand_face_id: vec![],
            cand_ability_idx: vec![],
            cand_rule_refs: vec![],
            pairs: vec![],
            target: TeacherTarget4::default(),
            outcome: 1.,
            aux: AuxTargets4::default(),
            provenance: StepProvenance4 {
                thinking_steps: 2,
                ..StepProvenance4::default()
            },
            shadow_label: Some(label),
            llm_label: None,
        };
        let mut writer = FrameLabels::new(&directory, 1, &source).unwrap();
        writer.push(&step).unwrap();
        writer.finish().unwrap();
        let path = directory.join("shadow-bc/game-000001.jsonl");
        let rows: Vec<serde_json::Value> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows[0]["source_namespace"], source.run_sha256);
        assert_eq!(rows[1]["step_cursor"], 91);
        assert_eq!(rows[1]["provenance"]["record_input_n"], 42);
        assert_eq!(rows[1]["provenance"]["source_game"], "source-17");
        assert_eq!(rows[2]["labels"], 1);
        assert!(FrameLabels::new(&directory, 1, &source).is_err());
        let roots = std::sync::Arc::new(std::collections::BTreeMap::new());
        let mut original =
            super::super::output::GameWriter::new(&directory, false, 1, roots.clone()).unwrap();
        original.push(&step).unwrap();
        original.finish().unwrap();
        let mut without = step;
        without.shadow_label = None;
        let mut reference =
            super::super::output::GameWriter::new(&directory, true, 1, roots).unwrap();
        reference.push(&without).unwrap();
        reference.finish().unwrap();
        assert_eq!(
            std::fs::read(directory.join("train/game-000001.bst4")).unwrap(),
            std::fs::read(directory.join("val/game-000001.bst4")).unwrap(),
            "separate provenance must not change BST4 model input ABI"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
