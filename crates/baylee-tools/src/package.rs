//! Packs a built client into the archive a GitHub Release publishes, and that
//! tree into the installers offered beside it.
//!
//! `package client BIN_DIR TARGET VERSION` writes
//! `target/package/baylee-client-<version>-<target>` as `.zip` (Windows,
//! macOS) or `.tar.gz` (Linux) and prints its path. The binary alone does not
//! run: the fonts load from `assets/` beside the executable (only the shaders
//! are embedded), the OFL wants each font's licence to travel with it, and
//! the AGPL wants LICENSE.
//!
//! `package installers TARGET VERSION` wraps that tree, one file per line on
//! stdout: a `.dmg` on macOS, an Inno Setup `.exe` on Windows, an `AppImage`
//! and a `.deb` on Linux. The installers hold exactly the archive's tree; the
//! updater never downloads them (#326, `docs/releasing.md` §"Installers").
//! Tools are found on PATH or named by `ISCC`, `APPIMAGETOOL` and
//! `APPIMAGE_RUNTIME` (`installers tools` fetches them pinned).
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const HELP: &str = "Usage: baylee-tools package <COMMAND>

Commands:
  client BIN_DIR TARGET VERSION   Pack a built client into its release archive
  installers TARGET VERSION       Wrap that archive's tree into installers

Paths are relative to the repository root; outputs go to target/package.";

/// The release folder every package is built in.
const OUT: &str = "target/package";
/// The installer artwork and templates.
const ART: &str = "scripts/installers";
/// The hicolor sizes both Linux installers carry (`installers art`'s
/// `LINUX_SIZES`).
pub const LINUX_ICON_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--help" | "-h"] | [] => println!("{HELP}"),
        ["client", bin, target, version] => println!(
            "{}",
            client(root, &root.join(bin), target, version)?.display()
        ),
        ["installers", target, version] => {
            for file in installers(root, target, version)? {
                println!("{}", file.display());
            }
        }
        _ => bail!("{HELP}"),
    }
    Ok(())
}

/// The program names a package carries, with the platform's suffix.
struct Programs {
    client: String,
    launcher: String,
    runtime: String,
    seat: String,
}

impl Programs {
    fn for_target(target: &str) -> Self {
        let exe = if target.contains("-windows-") {
            ".exe"
        } else {
            ""
        };
        Self {
            client: format!("baylee-client{exe}"),
            launcher: format!("baylee-launch{exe}"),
            runtime: format!("baylee-runtime{exe}"),
            seat: format!("baylee-seat{exe}"),
        }
    }
}

/// Packs `bin` (where cargo left the binaries) for `target` at `version`.
pub fn client(root: &Path, bin: &Path, target: &str, version: &str) -> Result<PathBuf> {
    let name = format!("baylee-client-{version}-{target}");
    let out = root.join(OUT);
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    let p = Programs::for_target(target);
    // The seat bridge travels beside the client: the client seats a language
    // model only by running it from its own folder (seatbin.rs), and without
    // it the room offers no language-model chair (beta.6 shipped without it).
    for program in [&p.client, &p.launcher, &p.seat] {
        ensure!(
            bin.join(program).is_file(),
            "no {}",
            bin.join(program).display()
        );
    }
    let assets = root.join("crates/baylee-client/assets");
    if target.contains("-apple-") {
        mac_bundle(root, bin, &stage, &p, version, &assets)?;
    } else {
        // The launcher is what a player starts, under the client's name; it
        // runs the real client (`baylee-runtime`) beside itself.
        fs::copy(bin.join(&p.client), stage.join(&p.runtime))?;
        fs::copy(bin.join(&p.launcher), stage.join(&p.client))?;
        fs::copy(bin.join(&p.seat), stage.join(&p.seat))?;
        copy_tree(&assets, &stage.join("assets"))?;
        // MSVC keeps the line tables in a .pdb beside the .exe.
        if bin.join("baylee_client.pdb").is_file() {
            fs::copy(
                bin.join("baylee_client.pdb"),
                stage.join("baylee_client.pdb"),
            )?;
        }
    }
    for file in ["LICENSE", "NOTICE"] {
        fs::copy(root.join(file), stage.join(file))?;
    }
    fs::write(stage.join("README.txt"), readme(target, version))?;

    if target.contains("-apple-") {
        let zip = out.join(format!("{name}.zip"));
        let _ = fs::remove_file(&zip);
        // Without --norsrc --noextattr, ditto stores each file's extended
        // attributes (com.apple.provenance on a Mac that ran the build) as
        // `._<name>` entries. Unpacked by anything but Finder, and by the
        // updater (#326), they land inside Baylee.app and break its seal.
        exec(
            Command::new("ditto")
                .args(["-c", "-k", "--norsrc", "--noextattr", "--keepParent"])
                .arg(&stage)
                .arg(&zip),
        )?;
        Ok(zip)
    } else if target.contains("-windows-") {
        let zip = out.join(format!("{name}.zip"));
        write_zip(&stage, &name, &zip)?;
        Ok(zip)
    } else {
        let archive = out.join(format!("{name}.tar.gz"));
        write_tar_gz(&stage, &name, &archive)?;
        Ok(archive)
    }
}

/// A bundle, so Finder starts a game rather than a Terminal. Bevy looks for
/// `assets/` next to the executable; the files live in `Resources` and `MacOS`
/// holds a link, because codesign treats everything in `MacOS` as code.
fn mac_bundle(
    root: &Path,
    bin: &Path,
    stage: &Path,
    p: &Programs,
    version: &str,
    assets: &Path,
) -> Result<()> {
    let app = stage.join("Baylee.app/Contents");
    fs::create_dir_all(app.join("MacOS"))?;
    fs::create_dir_all(app.join("Resources"))?;
    fs::copy(bin.join(&p.client), app.join("MacOS").join(&p.runtime))?;
    fs::copy(bin.join(&p.launcher), app.join("MacOS").join(&p.client))?;
    fs::copy(bin.join(&p.seat), app.join("MacOS").join(&p.seat))?;
    copy_tree(assets, &app.join("Resources/assets"))?;
    fs::copy(
        root.join("crates/baylee-client/assets/brand/baylee.icns"),
        app.join("Resources/Baylee.icns"),
    )?;
    symlink("../Resources/assets", &app.join("MacOS/assets"))?;
    fs::write(app.join("Info.plist"), info_plist(&p.client, version))?;
    // Ad hoc, which costs nothing and is not an identity: without a seal a
    // downloaded app reads as "damaged" rather than "unidentified developer",
    // and only the second has a way past it.
    for path in [
        app.join("MacOS").join(&p.runtime),
        app.join("MacOS").join(&p.seat),
        stage.join("Baylee.app"),
    ] {
        exec(
            Command::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(path),
        )?;
    }
    // Symbols for a crash report, beside the app rather than in it.
    let dsym = bin.join(format!("{}.dSYM", p.client));
    if dsym.is_dir() {
        copy_tree(
            &fs::canonicalize(&dsym)?,
            &stage.join(format!("{}.dSYM", p.client)),
        )?;
    }
    Ok(())
}

/// The bundle's `Info.plist`. `CFBundleURLTypes` claims the `baylee:`
/// scheme: macOS starts the bundle for a link when no client runs, and
/// sends it to the running client as an Apple Event either way
/// (`baylee-client/src/links/macos.rs`).
fn info_plist(executable: &str, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Baylee</string>
    <key>CFBundleDisplayName</key><string>Baylee</string>
    <key>CFBundleIconFile</key><string>Baylee.icns</string>
    <key>CFBundleIdentifier</key><string>local.baylee.client</string>
    <key>CFBundleExecutable</key><string>{executable}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>{version}</string>
    <key>CFBundleVersion</key><string>{version}</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key><string>local.baylee.client</string>
            <key>CFBundleURLSchemes</key><array><string>baylee</string></array>
        </dict>
    </array>
</dict>
</plist>
"#
    )
}

fn readme(target: &str, version: &str) -> String {
    let mut text = format!(
        "Baylee {version} ({target})\n\n\
         Unofficial fan content, not affiliated with Wizards of the Coast.\n\
         Source: https://github.com/AceVik/baylee (AGPL-3.0-only, see LICENSE).\n\n\
         Start the client and play against the house AI, or enter a gateway in the\n\
         lobby to play at a table. The assets folder must stay next to the program.\n"
    );
    if target.contains("-apple-") {
        text.push_str(
            "\nmacOS: drag Baylee.app into Applications before the first start (opened\n\
             here, macOS runs it from a hidden read-only copy; it still updates, and its\n\
             settings offer the move). The app is not notarised. On first start macOS\n\
             refuses it; open System Settings > Privacy & Security and choose \"Open\n\
             Anyway\", or run\n    xattr -dr com.apple.quarantine Baylee.app\n",
        );
    } else if target.contains("-windows-") {
        text.push_str(
            "\nWindows: the program is not signed. If SmartScreen stops it, choose\n\
             \"More info\" and then \"Run anyway\". It opens no console window; to see its\n\
             log, start it as  baylee-client.exe --console\n",
        );
    } else if target.contains("-linux-") {
        text.push_str(
            "\nLinux: needs a Vulkan driver plus the ALSA, udev, X11/Wayland, xkbcommon and libsecret\n\
             runtime libraries (on Debian/Ubuntu: libasound2 libudev1 libxkbcommon-x11-0\n\
             libwayland-client0 libsecret-1-0).\n",
        );
    }
    text
}

/// The archive's tree as a zip under its versioned top folder.
fn write_zip(stage: &Path, name: &str, file: &Path) -> Result<()> {
    let _ = fs::remove_file(file);
    let mut zip = zip::ZipWriter::new(fs::File::create(file)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (path, relative) in walk(stage)? {
        let entry = format!("{name}/{}", relative.replace('\\', "/"));
        if path.is_dir() {
            zip.add_directory(format!("{entry}/"), options)?;
        } else {
            zip.start_file(entry, options.unix_permissions(mode(&path)))?;
            zip.write_all(&fs::read(&path)?)?;
        }
    }
    zip.finish()?;
    Ok(())
}

/// The archive's tree as a gzip'd tar under its versioned top folder,
/// keeping each file's mode.
fn write_tar_gz(stage: &Path, name: &str, file: &Path) -> Result<()> {
    let gz = flate2::write::GzEncoder::new(fs::File::create(file)?, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    tar.follow_symlinks(false);
    tar.append_dir_all(name, stage)?;
    tar.into_inner()?.finish()?;
    Ok(())
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map_or(0o644, |m| m.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn mode(path: &Path) -> u32 {
    if path.extension().is_some_and(|e| e == "exe") {
        0o755
    } else {
        0o644
    }
}

/// Every entry below `root` with its relative path, parents before children.
fn walk(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root)?.to_string_lossy().into_owned();
            if entry.file_type()?.is_dir() {
                pending.push(path.clone());
            }
            found.push((path, relative));
        }
    }
    found.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(found)
}

/// Copies a directory tree, links kept as links.
pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            symlink(&fs::read_link(entry.path())?, &target)?;
        } else if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(original: impl AsRef<Path>, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(original, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink(_original: impl AsRef<Path>, link: &Path) -> Result<()> {
    bail!("{}: packages with links are built on Unix", link.display())
}

fn exec(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("running {program}"))?;
    ensure!(status.success(), "{program} failed");
    Ok(())
}

/// The target's architecture word, as the installer file names spell it.
fn arch(target: &str) -> Result<&'static str> {
    match target.split('-').next() {
        Some("x86_64") => Ok("x86_64"),
        Some("aarch64") => Ok("aarch64"),
        _ => bail!("unsupported target {target}"),
    }
}

/// Wraps the packaged tree into this target's installers.
pub fn installers(root: &Path, target: &str, version: &str) -> Result<Vec<PathBuf>> {
    let stage = root
        .join(OUT)
        .join(format!("baylee-client-{version}-{target}"));
    ensure!(
        stage.is_dir(),
        "no {}; run `baylee-tools package client` first",
        stage.display()
    );
    let arch = arch(target)?;
    if target.ends_with("-apple-darwin") {
        let file = root.join(OUT).join(format!("Baylee-{version}-{arch}.dmg"));
        crate::installers::run(
            root,
            &[
                "dmg".into(),
                stage.join("Baylee.app").display().to_string(),
                root.join(ART)
                    .join("macos/background.png")
                    .display()
                    .to_string(),
                stage
                    .join("Baylee.app/Contents/Resources/Baylee.icns")
                    .display()
                    .to_string(),
                file.display().to_string(),
            ],
        )?;
        Ok(vec![file])
    } else if target.ends_with("-pc-windows-msvc") {
        Ok(vec![setup(root, &stage, version, arch)?])
    } else if target.ends_with("-unknown-linux-gnu") {
        Ok(vec![
            appimage(root, &stage, version, arch)?,
            deb(root, &stage, version, arch)?,
        ])
    } else {
        bail!("no installer for {target}")
    }
}

/// The Inno Setup installer. The file version wants four numbers, so a
/// pre-release part has no place in it (0.1.0-beta.5 → 0.1.0.0); the product
/// version keeps it.
fn setup(root: &Path, stage: &Path, version: &str, arch: &str) -> Result<PathBuf> {
    let winarch = if arch == "x86_64" { "x64" } else { "arm64" };
    let numeric = format!("{}.0", version.split(['-', '+']).next().unwrap_or(version));
    let name = format!("Baylee-Setup-{version}-{winarch}");
    let out = root.join(OUT);
    let file = out.join(format!("{name}.exe"));
    let _ = fs::remove_file(&file);
    let iscc = std::env::var("ISCC").unwrap_or_else(|_| "ISCC.exe".into());
    let windows = |p: &Path| crate::installers::windows_path(p);
    exec(
        Command::new(iscc)
            .arg("/Q")
            .arg(format!("/DSource={}", windows(stage)))
            .arg(format!("/DVersion={version}"))
            .arg(format!("/DNumeric={numeric}"))
            .arg(format!("/DArch={winarch}"))
            .arg(format!(
                "/DIcon={}",
                windows(&root.join(ART).join("windows/baylee.ico"))
            ))
            .arg(format!("/O{}", windows(&out)))
            .arg(format!("/F{name}"))
            .arg(windows(&root.join(ART).join("windows/baylee.iss"))),
    )?;
    Ok(file)
}

/// The Linux installers keep the archive's tree in one folder, because the
/// launcher finds `baylee-runtime` and the client its `assets` beside
/// themselves: /opt/baylee in the .deb, the same path inside the `AppImage`.
fn opt_tree(stage: &Path, root: &Path) -> Result<()> {
    let opt = root.join("opt/baylee");
    copy_tree(stage, &opt)?;
    // Crash symbols stay in the archive; an installed game does not need them.
    for entry in fs::read_dir(&opt)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "pdb") {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Modes that do not depend on the builder's umask: 0755 for directories and
/// programs, 0644 for everything else (`chmod -R u=rwX,go=rX`).
#[cfg(unix)]
fn settle_modes(root: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for (path, _) in walk(root)? {
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let executable = meta.is_dir() || meta.permissions().mode() & 0o111 != 0;
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
        )?;
    }
    fs::set_permissions(root, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn settle_modes(_root: &Path) -> Result<()> {
    bail!("Linux installers are built on Linux")
}

fn appimage(root: &Path, stage: &Path, version: &str, arch: &str) -> Result<PathBuf> {
    let out = root.join(OUT);
    let file = out.join(format!("Baylee-{version}-{arch}.AppImage"));
    let appdir = out.join(format!("appimage-{arch}"));
    let _ = fs::remove_dir_all(&appdir);
    let _ = fs::remove_file(&file);
    opt_tree(stage, &appdir)?;
    // AppRun is the launcher itself: a link, so `current_exe` is the real
    // file and the launcher's `baylee-runtime` and `assets` sit beside it.
    symlink("opt/baylee/baylee-client", &appdir.join("AppRun"))?;
    let desktop = fs::read_to_string(root.join(ART).join("linux/baylee.desktop"))?;
    let desktop: Vec<String> = desktop
        .lines()
        .map(|l| match l.strip_prefix("Exec=baylee") {
            // `Exec=baylee %u`: the program's name inside the image, with
            // the link argument kept.
            Some(rest) if rest.is_empty() || rest.starts_with(' ') => {
                format!("Exec=baylee-client{rest}")
            }
            _ => l.to_owned(),
        })
        .collect();
    fs::write(appdir.join("baylee.desktop"), desktop.join("\n") + "\n")?;
    fs::copy(
        root.join(ART).join("linux/baylee.png"),
        appdir.join("baylee.png"),
    )?;
    symlink("baylee.png", &appdir.join(".DirIcon"))?;
    // Every size, for a desktop that integrates the AppImage (appimaged,
    // AppImageLauncher, Gear Lever): they copy these into the theme.
    copy_tree(
        &root.join(ART).join("linux/hicolor"),
        &appdir.join("usr/share/icons/hicolor"),
    )?;
    settle_modes(&appdir)?;
    let mut tool =
        Command::new(std::env::var("APPIMAGETOOL").unwrap_or_else(|_| "appimagetool".into()));
    tool.env("ARCH", arch).arg("--no-appstream");
    if let Ok(runtime) = std::env::var("APPIMAGE_RUNTIME")
        && !runtime.is_empty()
    {
        tool.args(["--runtime-file", &runtime]);
    }
    // No update information is embedded (no zsync): the client's own updater
    // is the update path, and it never touches this file.
    exec(
        tool.arg(&appdir)
            .arg(&file)
            .stdout(std::process::Stdio::null()),
    )?;
    fs::remove_dir_all(&appdir)?;
    Ok(file)
}

fn deb(root: &Path, stage: &Path, version: &str, arch: &str) -> Result<PathBuf> {
    let debarch = if arch == "x86_64" { "amd64" } else { "arm64" };
    let out = root.join(OUT);
    let file = out.join(format!("baylee_{version}_{debarch}.deb"));
    let tree = out.join(format!("deb-{arch}"));
    let _ = fs::remove_dir_all(&tree);
    let _ = fs::remove_file(&file);
    opt_tree(stage, &tree)?;
    for dir in [
        "usr/bin",
        "usr/share/applications",
        "usr/share/doc/baylee",
        "DEBIAN",
    ] {
        fs::create_dir_all(tree.join(dir))?;
    }
    // /proc/self/exe resolves the link, so the launcher still finds /opt/baylee.
    symlink(
        "../../opt/baylee/baylee-client",
        &tree.join("usr/bin/baylee"),
    )?;
    fs::copy(
        root.join(ART).join("linux/baylee.desktop"),
        tree.join("usr/share/applications/baylee.desktop"),
    )?;
    // 16 to 512 px: a panel, a menu and a dock each take the size they draw.
    copy_tree(
        &root.join(ART).join("linux/hicolor"),
        &tree.join("usr/share/icons/hicolor"),
    )?;
    for doc in ["LICENSE", "NOTICE"] {
        fs::copy(root.join(doc), tree.join("usr/share/doc/baylee").join(doc))?;
    }
    settle_modes(&tree)?;
    let size_kib: u64 = walk(&tree)?
        .iter()
        .filter_map(|(p, _)| fs::symlink_metadata(p).ok())
        .map(|m| m.len().div_ceil(1024).max(1))
        .sum();
    // Debian sorts `~` before anything, so 0.1.0~beta.5 comes before 0.1.0;
    // with semver's `-` it would come after.
    let debversion = version.replacen('-', "~", 1);
    fs::write(
        tree.join("DEBIAN/control"),
        control(&debversion, debarch, size_kib),
    )?;
    // xz rather than dpkg's zstd default, which Debian before 12 cannot read.
    exec(
        Command::new("dpkg-deb")
            .args(["-Zxz", "--root-owner-group", "--build"])
            .arg(&tree)
            .arg(&file)
            .stdout(std::process::Stdio::null()),
    )?;
    fs::remove_dir_all(&tree)?;
    Ok(file)
}

fn control(version: &str, arch: &str, size_kib: u64) -> String {
    format!(
        "Package: baylee\n\
         Version: {version}\n\
         Architecture: {arch}\n\
         Maintainer: Baylee (AceVik) <AceVik@users.noreply.github.com>\n\
         Installed-Size: {size_kib}\n\
         Depends: libc6 (>= 2.35), libasound2 | libasound2t64, libudev1, libxkbcommon-x11-0, libwayland-client0, libvulkan1, libsecret-1-0\n\
         Section: games\n\
         Priority: optional\n\
         Homepage: https://github.com/AceVik/baylee\n\
         Description: Baylee card game client (unofficial fan content)\n \
         Plays against the house AI, or at a table on a Baylee gateway.\n \
         Unofficial fan content, not affiliated with Wizards of the Coast.\n \
         .\n \
         Installed system-wide in /opt/baylee, which a player cannot write, so the\n \
         client does not replace itself: it says when a new release is out, and\n \
         that release's .deb is installed over this one.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_follow_the_target_and_the_debian_version_sorts_prereleases_first() {
        assert_eq!(arch("aarch64-apple-darwin").unwrap(), "aarch64");
        assert!(arch("riscv64gc-unknown-linux-gnu").is_err());
        assert_eq!("0.1.0-beta.5".replacen('-', "~", 1), "0.1.0~beta.5");
        let control = control("0.1.0~beta.5", "amd64", 42);
        assert!(
            control.contains("Version: 0.1.0~beta.5\n") && control.contains("Installed-Size: 42\n")
        );
        assert!(
            control
                .lines()
                .skip_while(|l| !l.starts_with("Description:"))
                .skip(1)
                .all(|l| l.starts_with(' '))
        );
    }

    #[test]
    fn the_readme_says_each_platforms_first_start() {
        assert!(readme("aarch64-apple-darwin", "1").contains("xattr -dr com.apple.quarantine"));
        assert!(readme("x86_64-pc-windows-msvc", "1").contains("--console"));
        assert!(readme("x86_64-unknown-linux-gnu", "1").contains("libsecret-1-0"));
    }

    #[test]
    fn packaging_carries_the_seat_bridge_and_refuses_its_absence() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("bins")).unwrap();
        fs::create_dir_all(root.join("crates/baylee-client/assets/fonts")).unwrap();
        fs::write(root.join("crates/baylee-client/assets/fonts/a.ttf"), "font").unwrap();
        for name in ["LICENSE", "NOTICE"] {
            fs::write(root.join(name), name).unwrap();
        }
        for name in ["baylee-client", "baylee-launch", "baylee-seat"] {
            fs::write(root.join("bins").join(name), name).unwrap();
        }
        let target = "x86_64-unknown-linux-gnu";
        let archive = client(root, &root.join("bins"), target, "0.0.0-test.1").unwrap();
        let mut names = Vec::new();
        let gz = flate2::read::GzDecoder::new(fs::File::open(&archive).unwrap());
        for entry in tar::Archive::new(gz).entries().unwrap() {
            names.push(entry.unwrap().path().unwrap().display().to_string());
        }
        for name in [
            "baylee-client",
            "baylee-runtime",
            "baylee-seat",
            "assets/fonts/a.ttf",
            "LICENSE",
            "README.txt",
        ] {
            assert!(
                names
                    .iter()
                    .any(|n| n == &format!("baylee-client-0.0.0-test.1-{target}/{name}")),
                "{name}"
            );
        }
        fs::remove_file(root.join("bins/baylee-seat")).unwrap();
        let refused = client(root, &root.join("bins"), target, "0.0.0-test.1").unwrap_err();
        assert!(refused.to_string().contains("baylee-seat"));
    }
}
