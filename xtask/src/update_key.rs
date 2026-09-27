//! `update-key`: the Ed25519 key release archives are signed with (#326).
//!
//! Writes a fresh 32-byte seed, base64, to
//! `~/.config/baylee-release/update-signing.key` (the directory `0700`, the
//! file `0600`, created only if it does not exist) and prints the public key
//! for `baylee_update::sign::TRUSTED_KEYS`. The seed is never printed and
//! never written anywhere else; the repository gets only the public half.
//! An existing key is never overwritten: losing the seed means no client
//! can verify another update signed by it, so a second run only prints the
//! public key the file already holds. `docs/releasing.md` §"Signing".

use std::path::{Path, PathBuf};

/// Where the seed lives unless `--out` says otherwise.
pub fn default_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("baylee-release")
        .join("update-signing.key"))
}

/// Creates the key at `path`, or reads the one already there, and prints
/// its public half.
pub fn run(path: &Path) -> anyhow::Result<()> {
    use baylee_update::sign;
    if path.exists() {
        let seed = std::fs::read_to_string(path)?;
        let key =
            sign::signing_key(&seed).map_err(|err| anyhow::anyhow!("{}: {err}", path.display()))?;
        println!(
            "{} already holds a key; it is left as it is.",
            path.display()
        );
        println!("public key: {}", sign::public_text(&key.verifying_key()));
        return Ok(());
    }
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("{} has no directory", path.display()))?;
    create_private_dir(dir)?;
    let seed = sign::new_seed_text().map_err(|err| anyhow::anyhow!("no entropy: {err}"))?;
    let key = sign::signing_key(&seed).map_err(|err| anyhow::anyhow!("{err}"))?;
    write_private(path, &format!("{seed}\n"))?;
    println!("wrote a new signing key to {} (mode 600)", path.display());
    println!("public key: {}", sign::public_text(&key.verifying_key()));
    println!();
    println!("Next:");
    println!("  1. put the public key in front of baylee_update::sign::TRUSTED_KEYS");
    println!(
        "  2. gh secret set BAYLEE_UPDATE_SIGNING_KEY < {}",
        path.display()
    );
    println!("  3. back the file up somewhere safe: without it no client can verify");
    println!("     an update signed by this key (docs/releasing.md §\"Signing\")");
    Ok(())
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// `create_new`, so a key that appeared between the check and the write is
/// not overwritten either, and the mode is set as the file is made rather
/// than after, so the seed is never readable by anybody else for an instant.
fn write_private(path: &Path, text: &str) -> anyhow::Result<()> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("baylee-update-key-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_new_key_is_its_owners_alone_and_never_overwritten() {
        let dir = scratch("new");
        let path = dir.join("release").join("update-signing.key");
        run(&path).unwrap();
        let seed = std::fs::read_to_string(&path).unwrap();
        let key = baylee_update::sign::signing_key(&seed).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&path), 0o600);
            assert_eq!(mode(path.parent().unwrap()), 0o700);
        }
        // A second run keeps the key it found.
        run(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), seed);
        assert_eq!(
            baylee_update::sign::signing_key(&std::fs::read_to_string(&path).unwrap())
                .unwrap()
                .to_bytes(),
            key.to_bytes()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
