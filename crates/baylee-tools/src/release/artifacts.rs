//! Exact package inventories and provenance manifests (schema 2).
use super::policy::{Identity, TARGETS};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

pub(super) struct Names {
    pub archive: String,
    pub installers: Vec<String>,
    pub manifest: String,
}
impl Names {
    pub fn files(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.archive.as_str()).chain(self.installers.iter().map(String::as_str))
    }
}

pub(super) fn names(version: &str, target: &str) -> Result<Names> {
    ensure!(
        regex::Regex::new(r"\A[0-9]+\.[0-9]+\.[0-9]+[-+a-zA-Z0-9.]*\z")?.is_match(version),
        "invalid package version"
    );
    let extension = TARGETS
        .iter()
        .find(|(name, _)| *name == target)
        .context("unsupported package target")?
        .1;
    let installers = match target {
        "x86_64-unknown-linux-gnu" => vec![
            format!("Baylee-{version}-x86_64.AppImage"),
            format!("baylee_{version}_amd64.deb"),
        ],
        "aarch64-unknown-linux-gnu" => vec![
            format!("Baylee-{version}-aarch64.AppImage"),
            format!("baylee_{version}_arm64.deb"),
        ],
        "x86_64-pc-windows-msvc" => vec![format!("Baylee-Setup-{version}-x64.exe")],
        "aarch64-pc-windows-msvc" => vec![format!("Baylee-Setup-{version}-arm64.exe")],
        "aarch64-apple-darwin" => vec![format!("Baylee-{version}-aarch64.dmg")],
        _ => vec![format!("Baylee-{version}-x86_64.dmg")],
    };
    Ok(Names {
        archive: format!("baylee-client-{version}-{target}.{extension}"),
        installers,
        manifest: format!("{target}.manifest.json"),
    })
}

pub(super) fn digest(path: &Path) -> Result<String> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut file = fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65_536].into_boxed_slice();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    let mut hex = String::with_capacity(64);
    for byte in hash.finalize() {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(hex)
}
fn checksum(sha: &str, name: &str) -> String {
    format!("{sha}  {name}\n")
}

fn manifest(
    directory: &Path,
    version: &str,
    target: &str,
    run: &str,
    identity: &Identity,
    names: &Names,
) -> Result<Value> {
    let mut installers = serde_json::Map::new();
    for name in &names.installers {
        installers.insert(name.clone(), json!(digest(&directory.join(name))?));
    }
    Ok(
        json!({"schema": 2, "repository": identity.repository, "commit": identity.commit,
        "version": version, "target": target, "run_id": run,
        "archive": names.archive, "sha256": digest(&directory.join(&names.archive))?, "installers": installers}),
    )
}
fn hash_of<'a>(data: &'a Value, names: &Names, name: &str) -> &'a str {
    if name == names.archive {
        data["sha256"].as_str().unwrap()
    } else {
        data["installers"][name].as_str().unwrap()
    }
}

pub(super) fn write(
    directory: &Path,
    version: &str,
    target: &str,
    run: &str,
    identity: &Identity,
) -> Result<()> {
    let names = names(version, target)?;
    let data = manifest(directory, version, target, run, identity, &names)?;
    for name in names.files() {
        fs::write(
            directory.join(format!("{name}.sha256")),
            checksum(hash_of(&data, &names, name), name),
        )?;
    }
    fs::write(
        directory.join(&names.manifest),
        format!("{}\n", serde_json::to_string(&data)?),
    )?;
    Ok(())
}

pub(super) fn verify(
    directory: &Path,
    version: &str,
    target: &str,
    run: &str,
    identity: &Identity,
) -> Result<()> {
    let names = names(version, target)?;
    let expected: BTreeSet<_> = std::iter::once(names.manifest.clone())
        .chain(
            names
                .files()
                .flat_map(|name| [name.to_owned(), format!("{name}.sha256")]),
        )
        .collect();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        ensure!(
            entry.file_type()?.is_file(),
            "package must contain regular files only"
        );
        actual.insert(
            entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("non-UTF-8 package filename"))?,
        );
    }
    ensure!(actual == expected, "unexpected or missing package files");
    let data: Value = serde_json::from_slice(&fs::read(directory.join(&names.manifest))?)?;
    let expected = manifest(directory, version, target, run, identity, &names)?;
    ensure!(data == expected, "package provenance or checksum mismatch");
    for name in names.files() {
        ensure!(
            fs::read_to_string(directory.join(format!("{name}.sha256")))?
                == checksum(hash_of(&expected, &names, name), name),
            "checksum file mismatch: {name}"
        );
    }
    Ok(())
}

pub(super) fn installer_sums(directory: &Path, version: &str) -> Result<String> {
    let mut lines = vec![
        "| Installer | SHA-256 |".to_owned(),
        "| --- | --- |".to_owned(),
    ];
    for (target, _) in TARGETS {
        for name in names(version, target)?.installers {
            let payload = directory.join(&name);
            let sidecar = directory.join(format!("{name}.sha256"));
            ensure!(
                fs::symlink_metadata(&payload)?.file_type().is_file()
                    && fs::symlink_metadata(&sidecar)?.file_type().is_file(),
                "missing/nonregular installer or checksum: {name}"
            );
            let sha = digest(&payload)?;
            ensure!(
                fs::read_to_string(&sidecar)? == checksum(&sha, &name),
                "malformed or incorrect checksum file: {}",
                sidecar.display()
            );
            lines.push(format!("| `{name}` | `{sha}` |"));
        }
    }
    Ok(lines.join("\n"))
}
