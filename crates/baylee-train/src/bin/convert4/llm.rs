//! Explicit external LLM BC producer; original records and evidence stay immutable.
use anyhow::{Context, ensure};
use baylee_train::convert4::{
    Step4,
    llm_bc::{LlmBc4, schema::Manifest, sha},
};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) struct Source {
    run_path: PathBuf,
    manifest_path: PathBuf,
    run_sha256: String,
    manifest_sha256: String,
    manifest: Manifest,
    record_path: PathBuf,
}
impl Source {
    pub(super) fn load(
        dir: &Path,
        game: &str,
        record: &Path,
        offset: u64,
        len: u64,
    ) -> anyhow::Result<Self> {
        ensure!(
            !game.is_empty() && game.bytes().all(|v| v.is_ascii_alphanumeric() || v == b'-'),
            "Invalid LLM source game"
        );
        let run_path = dir.join("run.json").canonicalize()?;
        let manifest_path = dir
            .join("llm")
            .join(format!("{game}.json"))
            .canonicalize()?;
        ensure!(
            manifest_path.starts_with(dir.canonicalize()?),
            "LLM manifest escapes frozen run"
        );
        let run = bounded(&run_path)?;
        let bytes = bounded(&manifest_path)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        ensure!(
            manifest
                .frozen_game
                .as_deref()
                .unwrap_or(&manifest.source_game)
                == game,
            "LLM frozen game mismatch"
        );
        let record_path = record.canonicalize()?;
        ensure!(
            offset == 0 && len == fs::metadata(&record_path)?.len(),
            "LLM records require one complete gzip artifact"
        );
        ensure!(
            sha(&bounded(&record_path)?) == manifest.record_artifact_sha256,
            "LLM compressed record artifact mismatch"
        );
        Ok(Self {
            run_path,
            manifest_path,
            run_sha256: sha(&run),
            manifest_sha256: sha(&bytes),
            manifest,
            record_path,
        })
    }
    pub(super) fn record_sha(&self) -> &str {
        &self.manifest.record_sha256
    }
    pub(super) fn verify(&self, record: &[u8]) -> Result<LlmBc4, String> {
        let check = || -> anyhow::Result<LlmBc4> {
            let run = bounded(&self.run_path)?;
            let manifest = bounded(&self.manifest_path)?;
            ensure!(
                sha(&run) == self.run_sha256 && sha(&manifest) == self.manifest_sha256,
                "LLM source changed"
            );
            ensure!(
                sha(&bounded(&self.record_path)?) == self.manifest.record_artifact_sha256,
                "LLM record artifact changed"
            );
            LlmBc4::load(&run, &manifest, record, &self.manifest.source_game)
                .map_err(anyhow::Error::msg)
        };
        check().map_err(|e| e.to_string())
    }
    pub(super) fn metadata(&self, id: u32) -> Value {
        json!({"converted_game_id":id,"source_namespace":self.run_sha256,"source_game":self.manifest.source_game,
          "run_sha256":self.run_sha256,"manifest_sha256":self.manifest_sha256,"record_sha256":self.manifest.record_sha256,
          "converted_labels":format!("llm-bc/game-{id:06}.jsonl"),"converted_manifest":format!("llm-bc/game-{id:06}.manifest.json")})
    }
}
fn bounded(path: &Path) -> anyhow::Result<Vec<u8>> {
    ensure!(
        fs::metadata(path)?.len() <= 64 * 1024 * 1024,
        "LLM source too large"
    );
    Ok(fs::read(path)?)
}

pub(super) struct FrameLabels {
    path: PathBuf,
    temporary: PathBuf,
    writer: BufWriter<File>,
    labels: u64,
}
impl FrameLabels {
    pub(super) fn new_with_encoder(
        root: &Path,
        id: u32,
        source: &Source,
        encoder: u32,
    ) -> anyhow::Result<Self> {
        let dir = root.join("llm-bc");
        fs::create_dir_all(&dir)?;
        let manifest = bounded(&source.manifest_path)?;
        ensure!(
            sha(&manifest) == source.manifest_sha256,
            "LLM manifest changed before copy"
        );
        let manifest_path = dir.join(format!("game-{id:06}.manifest.json"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(manifest_path)?;
        file.write_all(&manifest)?;
        file.sync_all()?;
        let path = dir.join(format!("game-{id:06}.jsonl"));
        let temporary = path.with_extension("partial");
        ensure!(!path.exists(), "Preserving LLM labels");
        let mut writer = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?,
        );
        let mut header = json!({"kind":"header","schema":1,"converted_game_id":id,
          "source_namespace":source.run_sha256,"source_game":source.manifest.source_game,"run_sha256":source.run_sha256,
          "manifest_sha256":source.manifest_sha256,"record_sha256":source.manifest.record_sha256,"label_use":"bc_only","ppo_eligible":false});
        if encoder == 8 {
            header["encoder_version"] = json!(8);
        }
        serde_json::to_writer(&mut writer, &header)?;
        writeln!(writer)?;
        Ok(Self {
            path,
            temporary,
            writer,
            labels: 0,
        })
    }
    pub(super) fn push(&mut self, step: &Step4) -> Result<(), String> {
        if let Some(label) = &step.llm_label {
            let value = json!({"kind":"label","seat":step.seat,"step_cursor":step.step_cursor,
              "record_input_n":label.record_input_n,"executed_source":"llm","ambiguous":false,
              "model":label.model,"executed_action":label.executed_action,"target":step.target});
            serde_json::to_writer(&mut self.writer, &value).map_err(|e| e.to_string())?;
            writeln!(self.writer).map_err(|e| e.to_string())?;
            self.labels += 1;
        }
        Ok(())
    }
    pub(super) fn finish(mut self) -> anyhow::Result<()> {
        serde_json::to_writer(
            &mut self.writer,
            &json!({"kind":"completed","labels":self.labels,"ppo_eligible":false}),
        )?;
        writeln!(self.writer)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        fs::hard_link(&self.temporary, &self.path)?;
        fs::remove_file(&self.temporary)?;
        Ok(())
    }
}

pub(super) fn finish_metadata(root: &Path, meta: &mut Value) -> anyhow::Result<()> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    use std::io::Read;
    let mut labels = serde_json::Map::new();
    let mut manifests = serde_json::Map::new();
    let mut frames = serde_json::Map::new();
    for source in meta["llm_bc_sources"]
        .as_array()
        .context("Missing LLM sources")?
    {
        for (key, map) in [
            ("converted_labels", &mut labels),
            ("converted_manifest", &mut manifests),
        ] {
            let name = source[key].as_str().context("Missing LLM companion path")?;
            map.insert(name.into(), json!(sha(&bounded(&root.join(name))?)));
        }
    }
    for split in ["train", "val"] {
        for name in meta[format!("{split}_shards")]
            .as_array()
            .context("Missing frame shards")?
        {
            let name = format!("{split}/{}", name.as_str().context("Frame shard name")?);
            // Frame shards can exceed metadata bounds: hash incrementally.
            let mut file = File::open(root.join(&name))?;
            let mut hash = Sha256::new();
            let mut buffer = [0_u8; 8192];
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            let mut digest = String::new();
            for byte in hash.finalize() {
                write!(digest, "{byte:02x}").expect("String");
            }
            frames.insert(name, json!(digest));
        }
    }
    meta["llm_bc_label_sha256"] = Value::Object(labels);
    meta["llm_bc_manifest_sha256"] = Value::Object(manifests);
    meta["llm_bc_frame_sha256"] = Value::Object(frames);
    meta["llm_bc_status"] = json!("complete");
    Ok(())
}
