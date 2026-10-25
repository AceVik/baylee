use super::{
    Arc, BufWriter, Compression, File, GameRef, GzEncoder, Path, PathBuf, Step4, UNKNOWN_CARD, fs,
    json,
};
use anyhow::Context as _;
use baylee_train::features4::SourceKind4;
use flate2::write::ZlibEncoder;
use std::io::Write as _;
use std::os::unix::fs::MetadataExt as _;

/// One private per-game stream. Its final filename is the commit marker.
pub(super) struct GameWriter {
    partial: PathBuf,
    final_path: PathBuf,
    enc: GzEncoder<BufWriter<File>>,
    sidecar_bin_partial: PathBuf,
    sidecar_bin_final: PathBuf,
    sidecar_idx_partial: PathBuf,
    sidecar_idx_final: PathBuf,
    sidecar_meta_final: PathBuf,
    bin_writer: BufWriter<File>,
    idx_writer: BufWriter<File>,
    bin_offset: u64,
    records: u64,
    frame: Vec<u8>,
    bst4_buf: Vec<u8>,
    zlib_buf: Vec<u8>,
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
        let shard_name = format!("game-{id:06}.jsonl.gz");
        let final_path = dir.join(&shard_name);
        let partial = final_path.with_extension("gz.partial");
        let file = File::create(&partial)?;

        let sidecar_dir = root.join(".shard-cache-v3");
        fs::create_dir_all(&sidecar_dir)?;
        let sidecar_bin_final = sidecar_dir.join(format!("{shard_name}.bin"));
        let sidecar_bin_partial = sidecar_dir.join(format!("{shard_name}.bin.partial"));
        let sidecar_idx_final = sidecar_dir.join(format!("{shard_name}.idx"));
        let sidecar_idx_partial = sidecar_dir.join(format!("{shard_name}.idx.partial"));
        let sidecar_meta_final = sidecar_dir.join(format!("{shard_name}.meta"));

        Ok(Self {
            partial,
            final_path,
            enc: GzEncoder::new(BufWriter::new(file), Compression::fast()),
            bin_writer: BufWriter::new(File::create(&sidecar_bin_partial)?),
            idx_writer: BufWriter::new(File::create(&sidecar_idx_partial)?),
            sidecar_bin_partial,
            sidecar_bin_final,
            sidecar_idx_partial,
            sidecar_idx_final,
            sidecar_meta_final,
            bin_offset: 0,
            records: 0,
            frame: Vec::with_capacity(64 * 1024),
            bst4_buf: Vec::with_capacity(32 * 1024),
            zlib_buf: Vec::with_capacity(16 * 1024),
            roots,
        })
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Contiguous little-endian BST4 binary frame layout"
    )]
    fn pack_bst4(&mut self, step: &Step4) -> anyhow::Result<()> {
        let state = &step.state;
        let n_ent = state.entity_feat.len();
        let ent_width = state.entity_feat.first().map_or(0, Vec::len);
        let ent_rule_cols = state
            .entity_rule_refs
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(1);
        let seat_width = state.seat_feat.first().map_or(0, Vec::len);
        let global_width = state.global_feat.len();
        let n_edg = state.edges.len();
        let n_cand = step.cand_feat.len();
        let cand_width = step.cand_feat.first().map_or(0, Vec::len);
        let cand_rule_cols = step
            .cand_rule_refs
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(1);
        let n_pair = step.pairs.len();
        let n_mm = step.target.multi_mask.len();
        let n_fm = step.target.free_mask.len();

        let flags = u32::from(step.is_decision)
            | (u32::from(step.is_terminal) << 1)
            | (u32::from(step.provenance.has_logprob) << 2);
        let src_kind: u32 = match step.provenance.source {
            SourceKind4::Net => 0,
            SourceKind4::House => 1,
            SourceKind4::Forced => 2,
            SourceKind4::Search => 3,
        };

        self.bst4_buf.clear();
        self.bst4_buf.extend_from_slice(b"BST4");
        for v in [
            1_u32,
            step.game_id,
            u32::from(step.seat),
            step.step_cursor,
            u32::from(step.turn),
            flags,
            u32::from(step.head),
            u32::from(step.pending_kind),
            src_kind,
            0_u32,
        ] {
            self.bst4_buf.extend_from_slice(&v.to_le_bytes());
        }
        self.bst4_buf
            .extend_from_slice(&i32::from(step.provenance.thinking_steps).to_le_bytes());
        self.bst4_buf
            .extend_from_slice(&i32::from(step.target.single_idx).to_le_bytes());
        for v in [
            step.loss_weight,
            step.target.scalar_norm,
            step.outcome,
            step.aux.turns_remaining_norm,
            step.aux.my_final_life_norm,
            step.aux.opp_min_final_life_norm,
            step.aux.next_life_delta_norm,
            step.provenance.behavior_logprob,
            step.provenance.temperature,
        ] {
            self.bst4_buf.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            n_ent as u32,
            ent_width as u32,
            ent_rule_cols as u32,
            seat_width as u32,
            global_width as u32,
            n_edg as u32,
            n_cand as u32,
            cand_width as u32,
            cand_rule_cols as u32,
            n_pair as u32,
            n_mm as u32,
            n_fm as u32,
        ] {
            self.bst4_buf.extend_from_slice(&v.to_le_bytes());
        }

        for row in &state.entity_feat {
            for &x in row {
                self.bst4_buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        for i in 0..n_ent {
            let cid = state
                .entity_card_ids
                .get(i)
                .copied()
                .unwrap_or(UNKNOWN_CARD);
            self.bst4_buf.extend_from_slice(&cid.to_le_bytes());
        }
        for i in 0..n_ent {
            self.bst4_buf
                .push(state.entity_face_ids.get(i).copied().unwrap_or(0));
        }
        for i in 0..n_ent {
            let tid = state.entity_token_ids.get(i).copied().unwrap_or(0);
            self.bst4_buf.extend_from_slice(&tid.to_le_bytes());
        }
        for refs in &state.entity_rule_refs[..n_ent] {
            for col in 0..ent_rule_cols {
                if let Some(reference) = refs.get(col) {
                    let Some(&root_id) = self.roots.get(&reference.key) else {
                        anyhow::bail!(
                            "unrepresented rule {} in game {} cursor {}",
                            reference.key,
                            step.game_id,
                            step.step_cursor
                        );
                    };
                    self.bst4_buf.extend_from_slice(&root_id.to_le_bytes());
                    self.bst4_buf
                        .extend_from_slice(&u32::from(reference.role).to_le_bytes());
                } else {
                    self.bst4_buf.extend_from_slice(&0_u64.to_le_bytes());
                }
            }
        }
        for row in &state.seat_feat {
            for &x in row {
                self.bst4_buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        for &x in &state.global_feat {
            self.bst4_buf.extend_from_slice(&x.to_le_bytes());
        }
        for &[src, dst, rel] in &state.edges {
            self.bst4_buf
                .extend_from_slice(&i32::from(src).to_le_bytes());
            self.bst4_buf
                .extend_from_slice(&i32::from(dst).to_le_bytes());
            self.bst4_buf
                .extend_from_slice(&i32::from(rel).to_le_bytes());
        }
        for row in &step.cand_feat {
            for &x in row {
                self.bst4_buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        for i in 0..n_cand {
            let ce = step.cand_entity_idx.get(i).copied().unwrap_or(-1);
            self.bst4_buf.extend_from_slice(&ce.to_le_bytes());
        }
        for i in 0..n_cand {
            let cs = step.cand_rel_seat_idx.get(i).copied().unwrap_or(-1);
            self.bst4_buf.extend_from_slice(&cs.to_le_bytes());
        }
        for i in 0..n_cand {
            let ccid = step.cand_card_id.get(i).copied().unwrap_or(UNKNOWN_CARD);
            self.bst4_buf.extend_from_slice(&ccid.to_le_bytes());
        }
        for i in 0..n_cand {
            self.bst4_buf
                .push(step.cand_face_id.get(i).copied().unwrap_or(255));
        }
        for refs in &step.cand_rule_refs[..n_cand] {
            for col in 0..cand_rule_cols {
                if let Some(reference) = refs.get(col) {
                    let Some(&root_id) = self.roots.get(&reference.key) else {
                        anyhow::bail!(
                            "unrepresented rule {} in game {} cursor {}",
                            reference.key,
                            step.game_id,
                            step.step_cursor
                        );
                    };
                    self.bst4_buf.extend_from_slice(&root_id.to_le_bytes());
                    self.bst4_buf
                        .extend_from_slice(&u32::from(reference.role).to_le_bytes());
                } else {
                    self.bst4_buf.extend_from_slice(&0_u64.to_le_bytes());
                }
            }
        }
        for &[b_idx, a_idx] in &step.pairs {
            self.bst4_buf.extend_from_slice(&b_idx.to_le_bytes());
            self.bst4_buf.extend_from_slice(&a_idx.to_le_bytes());
        }
        self.bst4_buf.extend_from_slice(&step.target.multi_mask);
        self.bst4_buf.extend_from_slice(&step.target.free_mask);
        Ok(())
    }

    pub fn push(&mut self, step: &Step4) -> anyhow::Result<()> {
        self.pack_bst4(step)?;
        self.zlib_buf.clear();
        {
            let mut zenc = ZlibEncoder::new(&mut self.zlib_buf, Compression::fast());
            zenc.write_all(&self.bst4_buf)?;
            zenc.finish()?;
        }
        let idx_flags = u32::from(step.is_decision) | (u32::from(step.is_terminal) << 1);
        let mut idx_row = [0_u8; 32];
        idx_row[0..4].copy_from_slice(&step.game_id.to_le_bytes());
        idx_row[4..8].copy_from_slice(&u32::from(step.seat).to_le_bytes());
        idx_row[8..16].copy_from_slice(&u64::from(step.step_cursor).to_le_bytes());
        idx_row[16..24].copy_from_slice(&self.bin_offset.to_le_bytes());
        idx_row[24..28].copy_from_slice(&(self.zlib_buf.len() as u32).to_le_bytes());
        idx_row[28..32].copy_from_slice(&idx_flags.to_le_bytes());
        self.idx_writer.write_all(&idx_row)?;
        self.bin_writer.write_all(&self.zlib_buf)?;
        self.bin_offset += self.zlib_buf.len() as u64;
        self.records += 1;

        self.frame.clear();
        serde_json::to_writer(&mut self.frame, step)?;
        self.frame.push(b'\n');
        self.enc.write_all(&self.frame)?;
        Ok(())
    }

    pub fn finish(mut self) -> anyhow::Result<String> {
        let mut file = self.enc.finish()?;
        file.flush()?;
        file.get_ref().sync_all()?;
        fs::rename(self.partial, &self.final_path)?;

        self.bin_writer.flush()?;
        self.bin_writer.get_ref().sync_all()?;
        self.idx_writer.flush()?;
        self.idx_writer.get_ref().sync_all()?;
        fs::rename(&self.sidecar_bin_partial, &self.sidecar_bin_final)?;
        fs::rename(&self.sidecar_idx_partial, &self.sidecar_idx_final)?;
        let st = fs::metadata(&self.final_path)?;
        let mtime_ns = i128::from(st.mtime()) * 1_000_000_000 + i128::from(st.mtime_nsec());
        let meta_tmp = self.sidecar_meta_final.with_extension("meta.partial");
        fs::write(
            &meta_tmp,
            serde_json::to_vec(&json!({
                "size": st.len(),
                "mtime_ns": mtime_ns,
                "records": self.records,
            }))?,
        )?;
        fs::rename(meta_tmp, &self.sidecar_meta_final)?;

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
