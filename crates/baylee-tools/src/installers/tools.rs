//! Fetches this system's installer tools, pinned by version and SHA-256,
//! into a directory, and prints the variables `package installers` reads, one
//! `NAME=value` per line (the workflow appends them to `GITHUB_ENV`).
//!
//! | system | tools |
//! | --- | --- |
//! | macOS | the system's hdiutil, ditto, tiffutil and xattr; Rust writes the Finder layout |
//! | Windows | Inno Setup 7.1.0, installed for this user only |
//! | Linux | appimagetool 1.9.1 (extracted, so it needs no FUSE) and the type2 runtime it embeds; the system's dpkg-deb |
//!
//! A new version: change it and its checksum here together, from the
//! release's own asset digests (`gh release view <tag> -R <repo> --json assets`).
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const INNO_SETUP: &str =
    "https://github.com/jrsoftware/issrc/releases/download/is-7_1_0/innosetup-7.1.0-x64.exe";
const INNO_SETUP_SHA256: &str = "0362a383ed217d4c4239b5933866dd96d3eb2102737da92f80f6057a4b40df2f";

/// appimagetool and the type2 runtime per architecture: (tool, runtime).
fn appimage_pins(arch: &str) -> Option<(&'static str, &'static str)> {
    match arch {
        "x86_64" => Some((
            "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0",
            "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d",
        )),
        "aarch64" => Some((
            "f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158",
            "00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444",
        )),
        _ => None,
    }
}

pub fn fetch(dir: &Path, target: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    let dir = fs::canonicalize(dir)?;
    if target.ends_with("-apple-darwin") {
        // Native system tools; no download, Python runtime or Finder
        // automation is needed.
        for tool in ["hdiutil", "ditto", "tiffutil", "xattr"] {
            ensure!(
                Path::new("/usr/bin").join(tool).exists(),
                "missing /usr/bin/{tool}"
            );
        }
    } else if target.ends_with("-pc-windows-msvc") {
        // One compiler for both targets: the x64 edition runs emulated on
        // Windows on ARM, and either edition builds an installer for arm64.
        let setup = dir.join("innosetup.exe");
        download(INNO_SETUP, INNO_SETUP_SHA256, &setup)?;
        let inno = dir.join("inno");
        let status = Command::new(&setup)
            .args([
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/SP-",
                "/CURRENTUSER",
            ])
            // Not `inno.display()`: on Windows `canonicalize` gave the
            // directory a `\\?\` prefix, and Inno Setup then installs nothing.
            .arg(format!("/DIR={}", super::windows_path(&inno)))
            .status()?;
        ensure!(
            status.success() && inno.join("ISCC.exe").is_file(),
            "Inno Setup did not install ISCC.exe"
        );
        println!("ISCC={}", super::windows_path(&inno.join("ISCC.exe")));
    } else if target.ends_with("-unknown-linux-gnu") {
        let arch = target.split('-').next().unwrap_or_default();
        let (tool, runtime) =
            appimage_pins(arch).with_context(|| format!("no installer tools for {target}"))?;
        let image = dir.join("appimagetool.AppImage");
        download(
            &format!(
                "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-{arch}.AppImage"
            ),
            tool,
            &image,
        )?;
        download(
            &format!(
                "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-{arch}"
            ),
            runtime,
            &dir.join("runtime"),
        )?;
        executable(&image)?;
        let extracted = dir.join("appimagetool");
        let _ = fs::remove_dir_all(&extracted);
        let status = Command::new(&image)
            .arg("--appimage-extract")
            .current_dir(&dir)
            .stdout(std::process::Stdio::null())
            .status()?;
        ensure!(status.success(), "extracting appimagetool failed");
        fs::rename(dir.join("squashfs-root"), &extracted)?;
        println!("APPIMAGETOOL={}", extracted.join("AppRun").display());
        println!("APPIMAGE_RUNTIME={}", dir.join("runtime").display());
    } else {
        bail!("no installer tools for {target}");
    }
    Ok(())
}

/// Downloads a pinned file; a checksum that differs is fatal and the file
/// is not kept. A connection that fails, before or during the body, is tried
/// three times.
fn download(url: &str, sha256: &str, to: &PathBuf) -> Result<()> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let mut last: Option<anyhow::Error> = None;
    for _ in 0..3 {
        match agent.get(url).call() {
            Ok(mut response) => {
                let mut bytes = Vec::new();
                if let Err(error) = response.body_mut().as_reader().read_to_end(&mut bytes) {
                    last = Some(error.into());
                    continue;
                }
                let got = Sha256::digest(&bytes)
                    .iter()
                    .fold(String::new(), |mut s, b| {
                        let _ = write!(s, "{b:02x}");
                        s
                    });
                ensure!(got == sha256, "{url}: sha256 {got}, expected {sha256}");
                fs::write(to, &bytes)?;
                return Ok(());
            }
            Err(error) => last = Some(error.into()),
        }
    }
    Err(last.map_or_else(
        || anyhow::anyhow!("download {url}"),
        |e| e.context(format!("download {url}")),
    ))
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn executable(_path: &Path) -> Result<()> {
    Ok(())
}
