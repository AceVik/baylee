//! Authenticated, anti-rollback encrypted SSD swap store (`XChaCha20-Poly1305` + `HKDF-SHA256`).
//!
//! Uses `baylee_seal` (`at_rest_seal`, `at_rest_open`, `history_name`) with an ephemeral
//! process-local CSPRNG key (`Zeroizing<[u8; 32]>`) and a strictly monotonic per-session
//! `swap_generation: u64` bound into the AEAD associated data (`{session_id}:{generation}`)
//! so neither cross-session file swapping nor rollback to an earlier swap file can pass.

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

struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&mut self, out: &mut [u8]) {
        getrandom::fill(out).expect("OS CSPRNG available");
    }
}

/// Metadata retained in RAM for a session currently paged out to SSD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapHandle4 {
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
    master_key: Zeroizing<[u8; 32]>,
    d_model: usize,
    max_swap_file_bytes: u64,
    max_uncompressed_bytes: usize,
}

impl EncryptedSwapStore4 {
    /// Initializes an encrypted SSD swap directory (`0700` on Unix) with a fresh CSPRNG key.
    ///
    /// # Errors
    /// If CSPRNG initialization or directory creation fails, or if `swap_dir` is a symlink.
    pub fn new(
        swap_dir: PathBuf,
        d_model: usize,
        max_swap_file_bytes: u64,
        max_uncompressed_bytes: usize,
    ) -> anyhow::Result<Self> {
        let mut master_key = Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut *master_key)
            .map_err(|e| anyhow::anyhow!("CSPRNG key generation failed: {e}"))?;
        ensure_secure_dir(&swap_dir)?;
        Ok(Self {
            swap_dir,
            master_key,
            d_model,
            max_swap_file_bytes,
            max_uncompressed_bytes,
        })
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

        let aad_name = format!("{session_id}:{generation}");
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

        let base_hex = history_name(&self.master_key, &session_id.to_string());
        let file_name = format!("{base_hex}.swap");
        let tmp_name = format!("{base_hex}.swap.tmp");
        let final_path = self.swap_dir.join(&file_name);
        let tmp_path = self.swap_dir.join(&tmp_name);

        write_atomic_0600(&tmp_path, &final_path, &sealed)?;
        Ok(SwapHandle4 {
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
        let path = self.swap_dir.join(&handle.file_name);
        let meta = fs::symlink_metadata(&path)
            .with_context(|| format!("inspecting swap file {}", path.display()))?;
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
            meta.len() <= self.max_swap_file_bytes,
            "security rejection: swap file size ({}) exceeds limit ({})",
            meta.len(),
            self.max_swap_file_bytes
        );

        let sealed = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let aad_name = format!("{session_id}:{}", handle.generation);
        let compressed = at_rest_open(
            &self.master_key,
            RestKind::State,
            &aad_name,
            &sealed,
        )
        .map_err(|_| {
            anyhow::anyhow!(
                "security rejection: AEAD authentication or anti-rollback check failed for session {session_id} (gen {})",
                handle.generation
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

fn write_atomic_0600(tmp_path: &Path, final_path: &Path, data: &[u8]) -> anyhow::Result<()> {
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
    let mut file = opts
        .open(tmp_path)
        .with_context(|| format!("opening tmp swap {}", tmp_path.display()))?;
    file.write_all(data)
        .with_context(|| format!("writing tmp swap {}", tmp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing tmp swap {}", tmp_path.display()))?;
    drop(file);
    fs::rename(tmp_path, final_path).with_context(|| {
        format!(
            "renaming {} -> {}",
            tmp_path.display(),
            final_path.display()
        )
    })
}
