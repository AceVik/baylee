use super::*;
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, Write};

/// One verified source/cache session. Reuse across complete recurrent histories.
pub struct Session {
    reader: crate::research_cache::Reader,
    catalog: catalog::Catalog,
    handshake: Value,
}
impl Session {
    /// Open externally pinned storage. This alone grants no learning admission.
    /// # Errors
    /// Rejects changed evidence, incompatible source catalogs and unsupported frames.
    pub fn open(cache: &Path, conversion_sha: &str, cache_sha: &str) -> Result<Self> {
        let cache = cache.canonicalize()?;
        let manifest = cache.join("research-cache.manifest.json");
        pinned(&manifest, cache_sha)?;
        let mut reader = crate::research_cache::Reader::open(&cache, conversion_sha)?;
        let catalog = catalog::Catalog::load(&cache, reader.metadata())?;
        // Preflight every row before returning a handshake; no late malformed tensor labels.
        for i in 0..reader.len() {
            frame::pack(&serde_json::from_slice(&reader.read_raw(i)?)?, &catalog)?;
        }
        pinned(&manifest, cache_sha)?;
        let meta = reader.metadata();
        let conversion = &meta["source_conversion_manifest"];
        let actor_candidates = meta["histories"]
            .as_array()
            .context("research histories")?
            .iter()
            .try_fold(0_usize, |n, h| {
                n.checked_add(uint(&h["actor_candidates"])?)
                    .context("actor count overflow")
            })?;
        let genuine = meta["histories"]
            .as_array()
            .context("research histories")?
            .iter()
            .try_fold(0_usize, |n, h| {
                n.checked_add(uint(&h["genuine_choices"])?)
                    .context("choice count overflow")
            })?;
        let handshake = json!({"schema":SCHEMA,"protocol":"jsonl_request_length_prefixed_le_arrays_v1","objective":OBJECTIVE,
            "cache_dir":cache,"conversion_manifest_sha256":conversion_sha,"cache_manifest_sha256":cache_sha,
            "content_fingerprint":meta["content_fingerprint"],"source_checkpoint_sha256":conversion["learner_model_identity"]["source_checkpoint_sha256"],
            "learner_model_identity":conversion["learner_model_identity"],"encoder_version":conversion["encoder_version"],
            "structure_sha256":conversion["catalog"]["structure_sha256"],"structure":catalog.structure,"catalog":conversion["catalog"],
            "attempt_counts":meta["attempt_counts"],"histories":meta["histories"],"frames":reader.len(),"actor_candidates":actor_candidates,"genuine_choices":genuine,
            "research_input_admitted":false,"ordinary_ppo_admitted":false,"numerical_replay_required":true,"admission":null});
        Ok(Self {
            reader,
            catalog,
            handshake,
        })
    }
    /// Exact provenance, histories and all attempt denominators for this session.
    #[must_use]
    pub fn metadata(&self) -> &Value {
        &self.handshake
    }
    pub(super) fn admitted(&mut self, receipt: Value) {
        self.handshake["research_input_admitted"] = json!(true);
        self.handshake["admission"] = receipt;
    }
    /// Pack a contiguous range inside one history without discarding passive frames.
    /// # Errors
    /// Refuses empty, oversized, cross-history or malformed requests.
    pub fn batch(&mut self, history: usize, start: usize, length: usize) -> Result<Batch> {
        let h = self.handshake["histories"]
            .as_array()
            .and_then(|a| a.get(history))
            .context("research history out of bounds")?
            .clone();
        let total = uint(&h["frames"])?;
        ensure!(
            (1..=256).contains(&length) && start.checked_add(length).is_some_and(|n| n <= total),
            "research range outside history or bound"
        );
        let first = uint(&h["first_frame"])?;
        let mut frames = Vec::new();
        let mut bytes = 0_usize;
        for cursor in start..start + length {
            let frame = frame::pack(
                &serde_json::from_slice(&self.reader.read_raw(first + cursor)?)?,
                &self.catalog,
            )?;
            for tensor in frame.values() {
                bytes = bytes
                    .checked_add(tensor.bytes.len())
                    .context("tensor request overflow")?;
            }
            ensure!(bytes <= MAX_BYTES, "tensor request exceeds memory bound");
            frames.push(frame);
        }
        arrays::stack(
            &frames,
            &json!({"history":history,"attempt_id":h["attempt_id"],"episode_id":h["episode_id"],"seat":h["seat"],"start":start,"length":length,"new_history":start==0,"complete_history":start==0&&length==total,"returns":h["returns"]}),
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    history: usize,
    start: usize,
    length: usize,
}

/// Serve a verified session until EOF, using bounded requests and binary arrays.
/// # Errors
/// Stops immediately on malformed input, changed row bytes or write failures.
pub fn serve(mut session: Session, mut input: impl BufRead, mut output: impl Write) -> Result<()> {
    serde_json::to_writer(&mut output, session.metadata())?;
    output.write_all(b"\n")?;
    output.flush()?;
    loop {
        let mut line = String::new();
        let n = std::io::Read::take(&mut input, 4097).read_line(&mut line)?;
        if n == 0 {
            break;
        }
        ensure!(
            n <= 4096 && line.ends_with('\n'),
            "oversized/truncated tensor request"
        );
        let request: Request = serde_json::from_str(&line)?;
        let batch = session.batch(request.history, request.start, request.length)?;
        let descriptor = serde_json::to_vec(&batch.descriptor)?;
        output.write_all(&u64::try_from(descriptor.len())?.to_le_bytes())?;
        output.write_all(&descriptor)?;
        output.write_all(&u64::try_from(batch.payload.len())?.to_le_bytes())?;
        output.write_all(&batch.payload)?;
        output.flush()?;
    }
    Ok(())
}
