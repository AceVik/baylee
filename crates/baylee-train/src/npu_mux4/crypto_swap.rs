//! Authenticated, anti-rollback encrypted SSD swap store (`XChaCha20-Poly1305` + `HKDF-SHA256`).
//!
//! Uses `baylee_seal` (`at_rest_seal`, `at_rest_open`, `history_name`) with an ephemeral
//! process-local CSPRNG key (`Zeroizing<[u8; 32]>`, rotatable at runtime via
//! [`EncryptedSwapStore4::rotate_master_key`]), a 128-bit boot-instance nonce, and a strictly
//! monotonic per-session `swap_generation: u64` bound into the AEAD associated data
//! (`{instance_nonce}:{session_id}:{generation}`) so neither cross-session file swapping,
//! cross-instance replay, nor rollback to an earlier swap file can pass.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail, ensure};
use baylee_seal::{Entropy, RestKind, at_rest_open, at_rest_seal, history_name};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use zeroize::Zeroizing;

use crate::netplay4::PlayerSnapshot4;

#[cfg(target_os = "linux")]
const O_NOFOLLOW_FLAG: i32 = 0o400_000;

struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&mut self, out: &mut [u8]) {
        getrandom::fill(out).expect("OS CSPRNG available");
    }
}

/// Metadata retained in RAM for a session currently paged out to SSD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapHandle4 {
    /// Key rotation epoch under which this swap file was sealed.
    pub key_epoch: u64,
    /// Monotonic swap generation bound into AEAD AAD to defeat rollback attacks.
    pub generation: u64,
    /// Opaque HMAC-SHA256 filename inside `swap_dir`.
    pub file_name: String,
    /// Size of the encrypted `.swap` file on disk (bytes).
    pub disk_bytes: u64,
}

/// Encrypted cold-tier SSD storage for evicted [`PlayerSnapshot4`] instances.
pub struct EncryptedSwapStore4 {
    swap_dir: PathBuf,
    instance_nonce_hex: String,
    current_epoch: u64,
    master_key: Zeroizing<[u8; 32]>,
    retired_keys: BTreeMap<u64, Zeroizing<[u8; 32]>>,
    d_model: usize,
    max_swap_file_bytes: u64,
    max_uncompressed_bytes: usize,
    durable_swap_fsync: bool,
}

impl EncryptedSwapStore4 {
    /// Initializes an encrypted SSD swap directory (`0700` on Unix) with a fresh CSPRNG key
    /// and a 128-bit process-instance nonce.
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
        let mut nonce_bytes = [0_u8; 16];
        getrandom::fill(&mut nonce_bytes)
            .map_err(|e| anyhow::anyhow!("CSPRNG nonce generation failed: {e}"))?;
        let mut instance_nonce_hex = String::with_capacity(32);
        for b in nonce_bytes {
            let _ = write!(instance_nonce_hex, "{b:02x}");
        }
        ensure_secure_dir(&swap_dir)?;
        Ok(Self {
            swap_dir,
            instance_nonce_hex,
            current_epoch: 0,
            master_key,
            retired_keys: BTreeMap::new(),
            d_model,
            max_swap_file_bytes,
            max_uncompressed_bytes,
            durable_swap_fsync,
        })
    }

    /// Rotates the active 256-bit master encryption key to a new CSPRNG key without
    /// interrupting existing cold sessions (which remain readable via their epoch key
    /// until paged in and re-encrypted under the new epoch).
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

    /// Compresses, encrypts (`XChaCha20-Poly1305`), and atomically writes `snapshot` to SSD.
    ///
    /// # Errors
    /// If snapshot validation, compression, encryption, or atomic file write fails.
    pub fn page_out(
        &self,
        session_id: u64,
        generation: u64,
        snapshot: &PlayerSnapshot4,
    ) -> anyhow::Result<SwapHandle4> {
        let raw = Zeroizing::new(snapshot.to_validated_wire(self.d_model)?);
        ensure!(
            raw.len() <= self.max_uncompressed_bytes,
            "uncompressed snapshot ({} B) exceeds limit ({} B)",
            raw.len(),
            self.max_uncompressed_bytes
        );
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder
            .write_all(&raw)
            .context("compressing snapshot payload")?;
        let compressed = Zeroizing::new(encoder.finish().context("finishing zlib stream")?);

        let aad_name = format!(
            "{}:{session_id}:{generation}:{}",
            self.instance_nonce_hex, self.current_epoch
        );
        let sealed = at_rest_seal(
            &self.master_key,
            RestKind::State,
            &aad_name,
            &compressed,
            &mut OsEntropy,
        )
        .context("sealing snapshot payload with XChaCha20-Poly1305")?;
        let disk_bytes = sealed.len() as u64;
        ensure!(
            disk_bytes <= self.max_swap_file_bytes,
            "sealed swap payload ({disk_bytes} B) exceeds max_swap_file_bytes ({})",
            self.max_swap_file_bytes
        );

        let base_hex = history_name(
            &self.master_key,
            &format!("{session_id}:{}", self.current_epoch),
        );
        let file_name = format!("{base_hex}.swap");
        let tmp_name = format!("{base_hex}.swap.tmp");
        let final_path = self.swap_dir.join(&file_name);
        let tmp_path = self.swap_dir.join(&tmp_name);

        write_atomic_0600(&tmp_path, &final_path, &sealed, self.durable_swap_fsync)?;
        Ok(SwapHandle4 {
            key_epoch: self.current_epoch,
            generation,
            file_name,
            disk_bytes,
        })
    }

    /// Reads, authenticates, decompresses, and validates a cold session from SSD,
    /// then removes the swap file so stale ciphertext never lingers on disk.
    ///
    /// # Errors
    /// If the swap file was tampered with, rolled back, symlinked, or fails validation.
    pub fn page_in(
        &self,
        session_id: u64,
        handle: &SwapHandle4,
    ) -> anyhow::Result<PlayerSnapshot4> {
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
        let sealed = read_nofollow_bounded(&path, self.max_swap_file_bytes)?;
        let aad_name = format!(
            "{}:{session_id}:{}:{}",
            self.instance_nonce_hex, handle.generation, handle.key_epoch
        );
        let compressed = at_rest_open(key, RestKind::State, &aad_name, &sealed).map_err(|_| {
            anyhow::anyhow!(
                "security rejection: AEAD authentication or anti-rollback check failed for session {session_id} (gen {}, epoch {})",
                handle.generation,
                handle.key_epoch
            )
        })?;

        let mut decoder =
            ZlibDecoder::new(compressed.as_slice()).take(self.max_uncompressed_bytes as u64 + 1);
        let mut raw = Zeroizing::new(Vec::new());
        decoder
            .read_to_end(&mut raw)
            .context("decompressing authenticated swap payload")?;
        if raw.len() > self.max_uncompressed_bytes {
            bail!(
                "security rejection: decompressed snapshot exceeds {} bytes",
                self.max_uncompressed_bytes
            );
        }

        let snapshot = PlayerSnapshot4::from_validated_wire(&raw, self.d_model)?;
        let _ = fs::remove_file(&path);
        Ok(snapshot)
    }

    /// Removes a cold session's swap file if present on disk.
    pub fn remove_swap_file(&self, handle: &SwapHandle4) {
        let path = self.swap_dir.join(&handle.file_name);
        if let Ok(meta) = fs::symlink_metadata(&path)
            && meta.is_file()
        {
            let _ = fs::remove_file(&path);
        }
    }

    /// Returns the filesystem path of a swap handle (used by security tests to simulate bit-flips).
    #[must_use]
    pub fn swap_path(&self, handle: &SwapHandle4) -> PathBuf {
        self.swap_dir.join(&handle.file_name)
    }
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

fn write_atomic_0600(
    tmp_path: &Path,
    final_path: &Path,
    data: &[u8],
    durable_fsync: bool,
) -> anyhow::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(tmp_path)
        && meta.file_type().is_symlink()
    {
        bail!(
            "security rejection: tmp swap path {} is a symlink",
            tmp_path.display()
        );
    }
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    #[cfg(target_os = "linux")]
    opts.custom_flags(O_NOFOLLOW_FLAG);
    let mut file = opts
        .open(tmp_path)
        .with_context(|| format!("opening tmp swap {}", tmp_path.display()))?;
    file.write_all(data)
        .with_context(|| format!("writing tmp swap {}", tmp_path.display()))?;
    if durable_fsync {
        file.sync_all()
            .with_context(|| format!("syncing tmp swap {}", tmp_path.display()))?;
    }
    drop(file);
    fs::rename(tmp_path, final_path).with_context(|| {
        format!(
            "renaming {} -> {}",
            tmp_path.display(),
            final_path.display()
        )
    })
}
