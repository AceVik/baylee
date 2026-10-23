use super::{
    Arc, BufWriter, Compression, File, GameRef, GzEncoder, Path, PathBuf, Step4, fs, json,
};
use anyhow::Context as _;
use std::io::Write as _;

/// One private per-game stream. Its final filename is the commit marker.
pub(super) struct GameWriter {
    partial: PathBuf,
    final_path: PathBuf,
    enc: GzEncoder<BufWriter<File>>,
    frame: Vec<u8>,
    roots: Arc<std::collections::BTreeMap<String, u32>>,
}

impl GameWriter {
    pub fn new(
        root: &Path,
        validation: bool,
        id: u32,
        roots: Arc<std::collections::BTreeMap<String, u32>>,
    ) -> anyhow::Result<Self> {
        let dir = root.join(if validation { "val" } else { "train" });
        fs::create_dir_all(&dir)?;
        let final_path = dir.join(format!("game-{id:06}.jsonl.gz"));
        let partial = final_path.with_extension("gz.partial");
        let file = File::create(&partial)?;
        Ok(Self {
            partial,
            final_path,
            enc: GzEncoder::new(BufWriter::new(file), Compression::fast()),
            frame: Vec::with_capacity(64 * 1024),
            roots,
        })
    }

    pub fn push(&mut self, step: &Step4) -> anyhow::Result<()> {
        for references in step
            .state
            .entity_rule_refs
            .iter()
            .chain(&step.cand_rule_refs)
        {
            for reference in references {
                anyhow::ensure!(
                    self.roots.contains_key(&reference.key),
                    "unrepresented rule {} in game {} cursor {}",
                    reference.key,
                    step.game_id,
                    step.step_cursor
                );
            }
        }
        self.frame.clear();
        serde_json::to_writer(&mut self.frame, step)?;
        self.frame.push(b'\n');
        self.enc.write_all(&self.frame)?;
        Ok(())
    }

    pub fn finish(self) -> anyhow::Result<String> {
        let mut file = self.enc.finish()?;
        file.flush()?;
        file.get_ref().sync_all()?;
        fs::rename(self.partial, &self.final_path)?;
        Ok(self
            .final_path
            .file_name()
            .context("game filename")?
            .to_string_lossy()
            .into_owned())
    }
}

pub(super) fn publish_ready(
    root: &Path,
    game: &GameRef,
    shard: &str,
    steps: u64,
    structure_sha256: &str,
) -> anyhow::Result<()> {
    let path = root
        .join("ready")
        .join(format!("game-{:06}.json", game.game_id));
    let temporary = path.with_extension("partial");
    fs::write(
        &temporary,
        serde_json::to_vec(&json!({
            "game_id": game.game_id, "split": if game.validation { "val" } else { "train" },
            "shard": shard, "steps": steps, "structure_sha256": structure_sha256,
            "source_shard": game.shard_path, "source_offset": game.offset, "source_len": game.len,
            "replay_hashes_checked": true, "unencoded_decisions": 0,
        }))?,
    )?;
    fs::rename(temporary, path)?;
    Ok(())
}
