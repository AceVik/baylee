use super::*;
use flate2::{
    Compression,
    read::{MultiGzDecoder, ZlibDecoder},
    write::ZlibEncoder,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{BufReader, Seek, SeekFrom, Write},
};

mod validation;

#[derive(Clone)]
struct Index {
    attempt: u64,
    episode: u32,
    seat: u32,
    cursor: u64,
    offset: u64,
    compressed: u32,
    raw: u32,
    flags: u32,
}
impl Index {
    fn encode(&self) -> [u8; 48] {
        let mut bytes = [0; 48];
        bytes[..8].copy_from_slice(&self.attempt.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.episode.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.seat.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.cursor.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.offset.to_le_bytes());
        bytes[32..36].copy_from_slice(&self.compressed.to_le_bytes());
        bytes[36..40].copy_from_slice(&self.raw.to_le_bytes());
        bytes[40..44].copy_from_slice(&self.flags.to_le_bytes());
        bytes
    }
    fn decode(bytes: &[u8; 48]) -> Result<Self> {
        ensure!(
            bytes[44..] == [0; 4],
            "unsupported research index reserved bits"
        );
        Ok(Self {
            attempt: u64::from_le_bytes(bytes[..8].try_into()?),
            episode: u32::from_le_bytes(bytes[8..12].try_into()?),
            seat: u32::from_le_bytes(bytes[12..16].try_into()?),
            cursor: u64::from_le_bytes(bytes[16..24].try_into()?),
            offset: u64::from_le_bytes(bytes[24..32].try_into()?),
            compressed: u32::from_le_bytes(bytes[32..36].try_into()?),
            raw: u32::from_le_bytes(bytes[36..40].try_into()?),
            flags: u32::from_le_bytes(bytes[40..44].try_into()?),
        })
    }
}

fn scan(
    source: &source::Source,
    mut frame: impl FnMut(&[u8], &rows::Metadata, u64) -> Result<()>,
) -> Result<Vec<Value>> {
    let structure = read_json(&source.root.join("card_structure.json"))?;
    let roots: BTreeMap<String, u32> = serde_json::from_value(structure["roots"].clone())?;
    let encoder = uint(&source.manifest["encoder_version"])?;
    let mut histories = Vec::new();
    let mut first = 0_u64;
    for history in &source.histories {
        let (mut frames, mut candidates, mut genuine) = (0_u64, 0_u64, 0_u64);
        let attempt = uint(&history.receipt["attempt_id"])?;
        lines(
            BufReader::new(MultiGzDecoder::new(File::open(&history.path)?)),
            |raw| {
                let metadata = rows::validate(raw, history, encoder, frames, &roots)?;
                frame(raw, &metadata, attempt)?;
                frames += 1;
                candidates += u64::from(metadata.flags & 4 != 0);
                genuine += u64::from(metadata.genuine);
                Ok(())
            },
        )?;
        ensure!(
            history.receipt["frames"] == frames
                && history.receipt["actor_decisions"] == candidates
                && frames > 0,
            "research history frame/candidate count changed"
        );
        histories.push(json!({"attempt_id":attempt,"episode_id":history.receipt["episode_id"],"seat":history.learner,
            "first_frame":first,"frames":frames,"actor_candidates":candidates,"genuine_choices":genuine,
            "returns":history.receipt["returns"],"source_receipt":history.receipt}));
        first = first
            .checked_add(frames)
            .context("research frame count overflow")?;
    }
    Ok(histories)
}

fn write_json_lines(path: &Path, rows: &[Value]) -> Result<()> {
    let mut file = File::create(path)?;
    for row in rows {
        serde_json::to_writer(&mut file, row)?;
        file.write_all(b"\n")?;
    }
    file.sync_all()?;
    Ok(())
}
fn output_seals(root: &Path) -> Result<BTreeMap<String, String>> {
    [
        "attempts.jsonl",
        "histories.jsonl",
        "frames.brc1",
        "frames.bri1",
        "card_structure.json",
        "card_dsl.bin",
        "card_dsl_back.bin",
        "token_dsl.bin",
        "card_abilities.bin",
    ]
    .into_iter()
    .map(|name| Ok((name.to_owned(), files::sha256(&bound(root, name)?)?)))
    .collect()
}
fn receipt(root: &Path, manifest: &Value) -> Result<Value> {
    Ok(
        json!({"schema":CACHE_SCHEMA,"manifest_sha256":files::sha256(&root.join("research-cache.manifest.json"))?,
        "content_fingerprint":manifest["content_fingerprint"],"expected_conversion_manifest_sha256":manifest["expected_conversion_manifest_sha256"],"admitted":false}),
    )
}

fn format_contract() -> Value {
    json!({"version":1,"data_magic":"BRC1","index_magic":"BRI1","index_entry_bytes":48,
        "payload":"original_research_json_utf8_zlib","max_frame_bytes":MAX_FRAME,"max_episode_bytes":MAX_EPISODE})
}

/// Build a new lossless research-frame cache from externally sealed conversion.
/// # Errors
/// Refuses existing outputs, unsupported/unbound sources, changed rows/seals and
/// incomplete histories. Failed artifacts are preserved; even a published
/// receipt requires full verification, including the final post-publication check.
pub fn build(data_dir: &Path, out: &Path, expected_manifest_sha256: &str) -> Result<Value> {
    ensure!(
        !out.exists(),
        "research cache output already exists; never repair completed caches"
    );
    let source = source::Source::load(data_dir, expected_manifest_sha256)?;
    fs::create_dir(out)?;
    let out = out.canonicalize()?;
    let mut data = File::create(out.join("frames.brc1"))?;
    let mut index = File::create(out.join("frames.bri1"))?;
    data.write_all(DATA_MAGIC)?;
    index.write_all(INDEX_MAGIC)?;
    let histories = scan(&source, |raw, metadata, attempt| {
        let mut compressor = ZlibEncoder::new(Vec::new(), Compression::fast());
        compressor.write_all(raw)?;
        let compressed = compressor.finish()?;
        ensure!(
            u64::try_from(compressed.len())? <= MAX_FRAME,
            "compressed research frame exceeds supported limit"
        );
        let entry = Index {
            attempt,
            episode: metadata.episode,
            seat: metadata.seat,
            cursor: metadata.cursor,
            offset: data.stream_position()?,
            compressed: u32::try_from(compressed.len())?,
            raw: u32::try_from(raw.len())?,
            flags: metadata.flags,
        };
        index.write_all(&entry.encode())?;
        data.write_all(&compressed)?;
        Ok(())
    })?;
    data.sync_all()?;
    index.sync_all()?;
    write_json_lines(&out.join("histories.jsonl"), &histories)?;
    for name in [
        "attempts.jsonl",
        "card_structure.json",
        "card_dsl.bin",
        "card_dsl_back.bin",
        "token_dsl.bin",
        "card_abilities.bin",
    ] {
        fs::copy(source.root.join(name), out.join(name))?;
        File::open(out.join(name))?.sync_all()?;
    }
    source.recheck()?;
    let mut manifest = json!({"schema":CACHE_SCHEMA,"status":"indexed_research_frames_only_unadmitted",
        "admitted":false,"ppo_eligible":false,"tensor_cache":false,"ordinary_target_cache_compatible":false,
        "probability_replay_required":true,"producer_binary_certified":false,
        "source_root":source.root,"expected_conversion_manifest_sha256":source.manifest_sha,
        "source_conversion_manifest":source.manifest,"source_files":source.seals,
        "target_contract":crate::research_contract::contract(),"frame_contract":crate::research_contract::FRAME_CONTRACT,
        "format":format_contract(),
        "attempt_counts":source.manifest["attempt_counts"],"histories":histories,
        "output_files":output_seals(&out)?,"builder_executable_sha256":files::sha256(&std::env::current_exe()?)?});
    manifest["content_fingerprint"] = json!(json_digest(&manifest)?);
    files::atomic_json(&out.join("research-cache.manifest.json"), &manifest)?;
    source.recheck()?;
    File::open(&out)?.sync_all()?;
    files::atomic_json(
        &out.join("research-cache.complete.json"),
        &receipt(&out, &manifest)?,
    )?;
    verify(&out, expected_manifest_sha256)
}

/// An opened, fully verified immutable research cache with lossless random reads.
/// No tensor or target-loader compatibility is implied.
pub struct Reader {
    data: File,
    entries: Vec<Index>,
    hashes: Vec<String>,
    metadata: Value,
}

impl Reader {
    /// Revalidate external sources, completion, payloads and index before reads.
    /// This audits the full source once; reuse the reader for a sequence of reads.
    /// # Errors
    /// Any changed, unsupported, incomplete or unbound evidence is refused.
    pub fn open(cache_dir: &Path, expected_manifest_sha256: &str) -> Result<Self> {
        let root = cache_dir.canonicalize()?;
        let (metadata, source) = validation::source(&root, expected_manifest_sha256)?;
        let mut data = File::open(root.join("frames.brc1"))?;
        let mut index = File::open(root.join("frames.bri1"))?;
        let mut magic = [0; 8];
        data.read_exact(&mut magic)?;
        ensure!(&magic == DATA_MAGIC, "unsupported research data header");
        index.read_exact(&mut magic)?;
        ensure!(
            &magic == INDEX_MAGIC
                && index
                    .metadata()?
                    .len()
                    .checked_sub(8)
                    .is_some_and(|size| size % INDEX_SIZE == 0),
            "unsupported research index header/length"
        );
        let mut entries = Vec::new();
        let mut hashes = Vec::new();
        let mut offset = 8_u64;
        let histories = scan(&source, |original, row, attempt| {
            let mut bytes = [0; 48];
            index.read_exact(&mut bytes)?;
            let entry = Index::decode(&bytes)?;
            ensure!(
                entry.attempt == attempt
                    && entry.episode == row.episode
                    && entry.seat == row.seat
                    && entry.cursor == row.cursor
                    && entry.flags == row.flags
                    && entry.offset == offset,
                "research index identity/order changed"
            );
            let raw = read_payload(&mut data, &entry)?;
            ensure!(
                raw == original,
                "research cache payload differs from original frame bytes"
            );
            offset = offset
                .checked_add(u64::from(entry.compressed))
                .context("cache length overflow")?;
            hashes.push(digest(&raw));
            entries.push(entry);
            Ok(())
        })?;
        ensure!(
            offset == data.metadata()?.len()
                && index.stream_position()? == index.metadata()?.len()
                && metadata["histories"] == json!(histories),
            "research cache has missing/extra history bytes"
        );
        let mut saved = Vec::new();
        lines(
            BufReader::new(File::open(root.join("histories.jsonl"))?),
            |raw| {
                saved.push(serde_json::from_slice::<Value>(raw)?);
                Ok(())
            },
        )?;
        ensure!(saved == histories, "research cache history index changed");
        source.recheck()?;
        Ok(Self {
            data,
            entries,
            hashes,
            metadata,
        })
    }

    /// Number of stored frames; unknown and unstarted attempts contribute zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether no known-return frames were stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// The immutable storage-only manifest, including all original denominators.
    #[must_use]
    pub fn metadata(&self) -> &Value {
        &self.metadata
    }
    /// Read exactly the original JSON frame bytes, checking the validated digest.
    /// # Errors
    /// Refuses an out-of-range row, decompression fault or post-open data mutation.
    pub fn read_raw(&mut self, row: usize) -> Result<Vec<u8>> {
        let entry = self
            .entries
            .get(row)
            .context("research frame index outside cache")?;
        let raw = read_payload(&mut self.data, entry)?;
        ensure!(
            digest(&raw) == self.hashes[row],
            "research payload changed after cache open"
        );
        Ok(raw)
    }
}

fn read_payload(data: &mut File, entry: &Index) -> Result<Vec<u8>> {
    ensure!(
        entry.offset >= 8
            && entry.compressed > 0
            && u64::from(entry.compressed) <= MAX_FRAME
            && u64::from(entry.raw) <= MAX_FRAME
            && entry
                .offset
                .checked_add(u64::from(entry.compressed))
                .is_some_and(|end| data.metadata().is_ok_and(|m| end <= m.len())),
        "research payload bounds invalid"
    );
    data.seek(SeekFrom::Start(entry.offset))?;
    let mut compressed = vec![0; usize::try_from(entry.compressed)?];
    data.read_exact(&mut compressed)?;
    let mut decoder = ZlibDecoder::new(compressed.as_slice());
    let mut raw = Vec::new();
    (&mut decoder)
        .take(u64::from(entry.raw) + 1)
        .read_to_end(&mut raw)?;
    ensure!(
        raw.len() == usize::try_from(entry.raw)?
            && decoder.total_in() == u64::from(entry.compressed),
        "research decompressed length/trailing bytes mismatch"
    );
    Ok(raw)
}

/// Verify a cache without writes, rebuilding or issuing training admission.
/// # Errors
/// Refuses any source, manifest, frame or index mismatch.
pub fn verify(cache_dir: &Path, expected_manifest_sha256: &str) -> Result<Value> {
    let reader = Reader::open(cache_dir, expected_manifest_sha256)?;
    Ok(
        json!({"schema":CACHE_SCHEMA,"cache_dir":cache_dir.canonicalize()?,"content_fingerprint":reader.metadata()["content_fingerprint"],
        "frames":reader.len(),"attempt_counts":reader.metadata()["attempt_counts"],"admitted":false,"tensor_cache":false}),
    )
}
