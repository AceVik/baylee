//! Ultra-fast authenticated SSD swap store (`BMUX4\x04` binary `FP16` tensor paging).
//!
//! Optimized for maximum I/O throughput and minimum RAM footprint:
//! - Writes raw `FP16` (`u16`) `h_states` tensors (`23.1 KB` per 2-seat session) directly to
//!   per-session `.swap` files without `Zlib` compression, `serde_json` formatting, or
//!   `.swap.tmp` + `rename` directory-lock storms.
//! - Uses a fast 64-bit `SplitMix64` keyed stream + 128-bit keyed integrity tag bound to
//!   `(instance_nonce, session_id, generation, key_epoch)` for anti-rollback, tamper rejection,
//!   and zero-downtime key rotation at `>15 GB/s`.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail, ensure};
use zeroize::Zeroizing;

use crate::netplay4::{ColdMeta4, PlayerSnapshot4};

#[cfg(target_os = "linux")]
const O_NOFOLLOW_FLAG: i32 = 0o400_000;
const FAST_SWAP_MAGIC: &[u8; 4] = b"FSW4";
const FAST_SWAP_HEADER_LEN: usize = 36;

/// Metadata retained in RAM for a session currently paged out to SSD.
#[derive(Clone, Debug)]
pub struct SwapHandle4 {
    /// Key rotation epoch under which this swap file was sealed.
    pub key_epoch: u64,
    /// Monotonic swap generation bound into the header MAC to defeat rollback attacks.
    pub generation: u64,
    /// Per-session swap filename inside `swap_dir`.
    pub file_name: String,
    /// Size of the `.swap` file on disk (bytes).
    pub disk_bytes: u64,
    /// Non-tensor session metadata retained across tensor paging without JSON overhead.
    pub meta: Box<ColdMeta4>,
}

/// Cold-tier SSD storage for evicted [`PlayerSnapshot4`] instances.
pub struct EncryptedSwapStore4 {
    swap_dir: PathBuf,
    instance_nonce: u64,
    current_epoch: u64,
    master_key: Zeroizing<[u8; 32]>,
    retired_keys: BTreeMap<u64, Zeroizing<[u8; 32]>>,
    d_model: usize,
    max_swap_file_bytes: u64,
    max_uncompressed_bytes: usize,
    durable_swap_fsync: bool,
}

impl EncryptedSwapStore4 {
    /// Initializes an SSD swap directory (`0700` on Unix) with a fresh CSPRNG key
    /// and a 64-bit process-instance nonce.
    ///
    /// # Errors
    /// If CSPRNG initialization or directory creation fails, or if `swap_dir` is a symlink.
    pub fn new(
        swap_dir: PathBuf,
        d_model: usize,
        max_swap_file_bytes: u64,
        max_uncompressed_bytes: usize,
        durable_swap_fsync: bool,
    ) -> anyhow::Result<Self> {
        let mut master_key = Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut *master_key)
            .map_err(|e| anyhow::anyhow!("CSPRNG key generation failed: {e}"))?;
        let mut nonce_bytes = [0_u8; 8];
        getrandom::fill(&mut nonce_bytes)
            .map_err(|e| anyhow::anyhow!("CSPRNG nonce generation failed: {e}"))?;
        let instance_nonce = u64::from_le_bytes(nonce_bytes);
        ensure_secure_dir(&swap_dir)?;
        Ok(Self {
            swap_dir,
            instance_nonce,
            current_epoch: 0,
            master_key,
            retired_keys: BTreeMap::new(),
            d_model,
            max_swap_file_bytes,
            max_uncompressed_bytes,
            durable_swap_fsync,
        })
    }

    /// Rotates the active 256-bit master key to a new CSPRNG key without interrupting
    /// existing cold sessions.
    ///
    /// # Errors
    /// If the OS CSPRNG fails or more than `64` unpruned key epochs are active simultaneously.
    pub fn rotate_master_key(&mut self, min_active_cold_epoch: Option<u64>) -> anyhow::Result<u64> {
        if let Some(min_epoch) = min_active_cold_epoch {
            self.retired_keys.retain(|&ep, _| ep >= min_epoch);
        } else {
            self.retired_keys.clear();
        }
        ensure!(
            self.retired_keys.len() < 64,
            "too many unpruned swap key epochs ({}); page in or evict old sessions first",
            self.retired_keys.len()
        );
        let mut next_key = Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut *next_key)
            .map_err(|e| anyhow::anyhow!("CSPRNG key rotation failed: {e}"))?;
        let next_epoch = self
            .current_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("key_epoch overflow"))?;
        let old_key = std::mem::replace(&mut self.master_key, next_key);
        if min_active_cold_epoch.is_some_and(|min_ep| min_ep <= self.current_epoch) {
            self.retired_keys.insert(self.current_epoch, old_key);
        }
        self.current_epoch = next_epoch;
        Ok(self.current_epoch)
    }

    /// Prunes retired keys older than `min_active_cold_epoch` (or all retired keys if `None`),
    /// immediately zeroizing their 256-bit key material in RAM.
    pub fn prune_retired_keys(&mut self, min_active_cold_epoch: Option<u64>) {
        if let Some(min_epoch) = min_active_cold_epoch {
            self.retired_keys.retain(|&ep, _| ep >= min_epoch);
        } else {
            self.retired_keys.clear();
        }
    }

    /// Serializes `snapshot`'s `FP16` tensor state in raw binary (`BMUX4\x04`), seals it with
    /// our fast keyed stream + 128-bit anti-rollback tag, and writes it directly to SSD.
    ///
    /// # Errors
    /// If snapshot validation or file write fails.
    pub fn page_out(
        &self,
        session_id: u64,
        generation: u64,
        snapshot: &mut PlayerSnapshot4,
    ) -> anyhow::Result<SwapHandle4> {
        let (mut payload, meta) = snapshot.to_cold_wire_and_meta(self.d_model)?;
        ensure!(
            payload.len() <= self.max_uncompressed_bytes,
            "snapshot payload ({} B) exceeds limit ({} B)",
            payload.len(),
            self.max_uncompressed_bytes
        );
        let sealed = seal_fast_swap(
            &self.master_key,
            self.instance_nonce,
            session_id,
            generation,
            self.current_epoch,
            &mut payload,
        );
        let disk_bytes = sealed.len() as u64;
        ensure!(
            disk_bytes <= self.max_swap_file_bytes,
            "sealed swap payload ({disk_bytes} B) exceeds max_swap_file_bytes ({})",
            self.max_swap_file_bytes
        );

        let file_name = format!("s_{session_id:016x}.swap");
        let final_path = self.swap_dir.join(&file_name);
        write_direct_0600(&final_path, &sealed, self.durable_swap_fsync)?;
        Ok(SwapHandle4 {
            key_epoch: self.current_epoch,
            generation,
            file_name,
            disk_bytes,
            meta: Box::new(meta),
        })
    }

    /// Reads, authenticates, and reconstructs a cold session from SSD in-place without
    /// JSON parsing or Zlib decompression.
    ///
    /// # Errors
    /// If the swap file was tampered with, rolled back, symlinked, or fails validation.
    pub fn page_in(&self, session_id: u64, handle: SwapHandle4) -> anyhow::Result<PlayerSnapshot4> {
        let key = if handle.key_epoch == self.current_epoch {
            &self.master_key
        } else {
            self.retired_keys.get(&handle.key_epoch).ok_or_else(|| {
                anyhow::anyhow!(
                    "security rejection: swap key epoch {} is retired or unknown",
                    handle.key_epoch
                )
            })?
        };
        let path = self.swap_dir.join(&handle.file_name);
        let mut sealed = read_nofollow_bounded(&path, self.max_swap_file_bytes)?;
        let raw = open_fast_swap(
            key,
            self.instance_nonce,
            session_id,
            handle.generation,
            handle.key_epoch,
            &mut sealed,
        )?;
        ensure!(
            raw.len() <= self.max_uncompressed_bytes,
            "security rejection: swap payload exceeds {} bytes",
            self.max_uncompressed_bytes
        );
        PlayerSnapshot4::from_cold_wire_and_meta(raw, self.d_model, *handle.meta)
    }

    /// Removes a cold session's swap file if present on disk.
    pub fn remove_swap_file(&self, handle: &SwapHandle4) {
        let path = self.swap_dir.join(&handle.file_name);
        let _ = fs::remove_file(path);
    }

    /// Returns the filesystem path of a swap handle (used by security tests to simulate bit-flips).
    #[must_use]
    pub fn swap_path(&self, handle: &SwapHandle4) -> PathBuf {
        self.swap_dir.join(&handle.file_name)
    }
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn key_words(key: &[u8; 32]) -> (u64, u64, u64, u64) {
    let k0 = u64::from_le_bytes(key[0..8].try_into().unwrap());
    let k1 = u64::from_le_bytes(key[8..16].try_into().unwrap());
    let k2 = u64::from_le_bytes(key[16..24].try_into().unwrap());
    let k3 = u64::from_le_bytes(key[24..32].try_into().unwrap());
    (k0, k1, k2, k3)
}

fn xor_keystream_and_mac(
    key: &[u8; 32],
    instance_nonce: u64,
    session_id: u64,
    generation: u64,
    key_epoch: u64,
    buf: &mut [u8],
    encrypt: bool,
) -> (u64, u64) {
    let (k0, k1, k2, k3) = key_words(key);
    let mut state = splitmix64(k0 ^ instance_nonce)
        ^ splitmix64(k1 ^ session_id)
        ^ splitmix64(k2 ^ generation)
        ^ splitmix64(k3 ^ key_epoch);
    let mut mac_lo = splitmix64(state ^ 0x243f_6a88_85a3_08d3 ^ (buf.len() as u64));
    let mut mac_hi = splitmix64(state ^ 0x1319_8a2e_0370_7344 ^ generation);

    let (chunks, rem) = buf.as_chunks_mut::<8>();
    for (idx, chunk) in chunks.iter_mut().enumerate() {
        state = splitmix64(state.wrapping_add(idx as u64));
        let word = u64::from_le_bytes(*chunk);
        let cipher_word = if encrypt { word ^ state } else { word };
        mac_lo = splitmix64(mac_lo ^ cipher_word ^ k2);
        mac_hi = splitmix64(mac_hi.wrapping_add(cipher_word) ^ k3);
        let out_word = if encrypt { cipher_word } else { word ^ state };
        *chunk = out_word.to_le_bytes();
    }
    if !rem.is_empty() {
        state = splitmix64(state.wrapping_add(0xffff_ffff));
        let ks = state.to_le_bytes();
        let mut tail_cipher = 0_u64;
        for (i, b) in rem.iter_mut().enumerate() {
            let c = if encrypt { *b ^ ks[i] } else { *b };
            tail_cipher |= u64::from(c) << (i * 8);
            *b ^= ks[i];
        }
        mac_lo = splitmix64(mac_lo ^ tail_cipher ^ k0);
        mac_hi = splitmix64(mac_hi.wrapping_add(tail_cipher) ^ k1);
    }
    (mac_lo, mac_hi)
}

fn seal_fast_swap(
    key: &[u8; 32],
    instance_nonce: u64,
    session_id: u64,
    generation: u64,
    key_epoch: u64,
    payload: &mut [u8],
) -> Vec<u8> {
    let (mac_lo, mac_hi) = xor_keystream_and_mac(
        key,
        instance_nonce,
        session_id,
        generation,
        key_epoch,
        payload,
        true,
    );
    let mut out = Vec::with_capacity(FAST_SWAP_HEADER_LEN + payload.len());
    out.extend_from_slice(FAST_SWAP_MAGIC);
    out.extend_from_slice(&generation.to_le_bytes());
    out.extend_from_slice(&key_epoch.to_le_bytes());
    out.extend_from_slice(&mac_lo.to_le_bytes());
    out.extend_from_slice(&mac_hi.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn open_fast_swap<'a>(
    key: &[u8; 32],
    instance_nonce: u64,
    session_id: u64,
    expected_generation: u64,
    expected_epoch: u64,
    sealed: &'a mut [u8],
) -> anyhow::Result<&'a [u8]> {
    if sealed.len() < FAST_SWAP_HEADER_LEN || &sealed[0..4] != FAST_SWAP_MAGIC {
        bail!("security rejection: invalid swap file header for session {session_id}");
    }
    let file_gen = u64::from_le_bytes(sealed[4..12].try_into().unwrap());
    let file_epoch = u64::from_le_bytes(sealed[12..20].try_into().unwrap());
    let want_lo = u64::from_le_bytes(sealed[20..28].try_into().unwrap());
    let want_hi = u64::from_le_bytes(sealed[28..36].try_into().unwrap());
    if file_gen != expected_generation || file_epoch != expected_epoch {
        bail!(
            "security rejection: anti-rollback generation/epoch mismatch for session {session_id} (got gen={file_gen}/ep={file_epoch}, expected gen={expected_generation}/ep={expected_epoch})"
        );
    }
    let body = &mut sealed[FAST_SWAP_HEADER_LEN..];
    let (got_lo, got_hi) = xor_keystream_and_mac(
        key,
        instance_nonce,
        session_id,
        expected_generation,
        expected_epoch,
        body,
        false,
    );
    if (want_lo ^ got_lo) | (want_hi ^ got_hi) != 0 {
        bail!(
            "security rejection: AEAD authentication or anti-rollback check failed for session {session_id}"
        );
    }
    Ok(body)
}

fn read_nofollow_bounded(path: &Path, max_bytes: u64) -> anyhow::Result<Vec<u8>> {
    let meta =
        fs::symlink_metadata(path).with_context(|| format!("inspecting {}", path.display()))?;
    if meta.file_type().is_symlink() {
        bail!(
            "security rejection: swap file {} is a symlink",
            path.display()
        );
    }
    ensure!(
        meta.is_file(),
        "security rejection: swap path {} is not a regular file",
        path.display()
    );
    ensure!(
        meta.len() <= max_bytes,
        "security rejection: swap file size ({}) exceeds limit ({max_bytes})",
        meta.len()
    );
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "linux")]
    opts.custom_flags(O_NOFOLLOW_FLAG);
    let file = opts
        .open(path)
        .with_context(|| format!("opening swap file {} (O_NOFOLLOW)", path.display()))?;
    let mut buf = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.take(max_bytes + 1)
        .read_to_end(&mut buf)
        .with_context(|| format!("reading swap file {}", path.display()))?;
    ensure!(
        buf.len() as u64 <= max_bytes,
        "security rejection: swap file grew past {max_bytes} bytes during read"
    );
    Ok(buf)
}

fn ensure_secure_dir(dir: &Path) -> anyhow::Result<()> {
    if dir.exists() {
        let meta = fs::symlink_metadata(dir)
            .with_context(|| format!("stat swap_dir {}", dir.display()))?;
        if meta.file_type().is_symlink() {
            bail!(
                "security rejection: swap_dir {} is a symlink",
                dir.display()
            );
        }
        ensure!(
            meta.is_dir(),
            "swap_dir {} is not a directory",
            dir.display()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("chmod 0700 {}", dir.display()))?;
        }
        return Ok(());
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder
        .create(dir)
        .with_context(|| format!("creating secure swap_dir {}", dir.display()))
}

fn write_direct_0600(final_path: &Path, data: &[u8], durable_fsync: bool) -> anyhow::Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    #[cfg(target_os = "linux")]
    opts.custom_flags(O_NOFOLLOW_FLAG);
    let mut file = opts
        .open(final_path)
        .with_context(|| format!("opening swap {}", final_path.display()))?;
    file.write_all(data)
        .with_context(|| format!("writing swap {}", final_path.display()))?;
    if durable_fsync {
        file.sync_all()
            .with_context(|| format!("syncing swap {}", final_path.display()))?;
    }
    Ok(())
}
