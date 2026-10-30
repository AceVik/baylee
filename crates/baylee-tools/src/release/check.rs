//! Opens the installers `package installers` wrote and checks what a player
//! would get from them: `release check-installers [--install] TARGET VERSION`.
//!
//! Without `--install` it only looks (mounts the dmg, extracts the `AppImage`,
//! lists the .deb), which runs on a developer's machine. `--install` is for a
//! throwaway CI runner: it installs as a player would and removes it again.
//! macOS copies Baylee.app to /Applications (what the drag does) and starts it
//! until the client logs the assets it found; Windows runs the setup silently
//! for this user, checks files, shortcut and uninstall entry, then
//! uninstalls; Linux installs the .deb with apt, which also proves its
//! dependencies resolve, then removes it.
use crate::package::LINUX_ICON_SIZES;
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const OUT: &str = "target/package";

pub fn run(root: &Path, install: bool, target: &str, version: &str) -> Result<()> {
    let Some(arch @ ("x86_64" | "aarch64")) = target.split('-').next() else {
        bail!("unsupported target {target}");
    };
    let out = root.join(OUT);
    if target.ends_with("-apple-darwin") {
        mac(root, &out, install, version, arch)
    } else if target.ends_with("-pc-windows-msvc") {
        windows(root, &out, install, target, version, arch)
    } else if target.ends_with("-unknown-linux-gnu") {
        linux(&out, install, version, arch)
    } else {
        bail!("no installer for {target}")
    }
}

fn need(path: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path).is_ok(),
        "missing: {}",
        path.display()
    );
    Ok(())
}

fn link_target(path: &Path) -> String {
    fs::read_link(path)
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

fn ok(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("running {program}"))?;
    ensure!(status.success(), "{program} failed");
    Ok(())
}

fn output(command: &mut Command) -> Result<String> {
    let out = command.output()?;
    ensure!(
        out.status.success(),
        "{} failed",
        command.get_program().to_string_lossy()
    );
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The launcher logs `assets from <dir>` once the client is up; that line is
/// how a smoke test sees which folder the started client reads
/// (`docs/client.md` §"Where a packaged desktop build finds its fonts").
fn started_from(program: &Path, expect: &Path) -> Result<()> {
    let log_path = std::env::temp_dir().join(format!("baylee-start-{}.log", std::process::id()));
    let log = fs::File::create(&log_path)?;
    let mut child = Command::new(program)
        .env("RUST_LOG", "info")
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()?;
    let wanted = format!("assets from {}", expect.display());
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut seen = false;
    while Instant::now() < deadline {
        let mut text = String::new();
        fs::File::open(&log_path)?.read_to_string(&mut text)?;
        if text.contains(&wanted) {
            seen = true;
            break;
        }
        if child.try_wait()?.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    // The launcher's child is the client: end both.
    if let Ok(children) = Command::new("pgrep")
        .args(["-P", &child.id().to_string()])
        .output()
    {
        for pid in String::from_utf8_lossy(&children.stdout).split_whitespace() {
            let _ = Command::new("kill").arg(pid).status();
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let log = fs::read_to_string(&log_path).unwrap_or_default();
    let _ = fs::remove_file(&log_path);
    if !seen {
        eprintln!("{log}");
        bail!(
            "{} did not start a client reading {}",
            program.display(),
            expect.display()
        );
    }
    println!("started from {}", expect.display());
    Ok(())
}

fn mac(root: &Path, out: &Path, install: bool, version: &str, arch: &str) -> Result<()> {
    let dmg = out.join(format!("Baylee-{version}-{arch}.dmg"));
    need(&dmg)?;
    let mount = tempfile::tempdir()?;
    let mnt = mount.path();
    ok(Command::new("hdiutil")
        .args([
            "attach",
            "-nobrowse",
            "-readonly",
            "-noautoopen",
            "-mountpoint",
        ])
        .arg(mnt)
        .arg(&dmg)
        .stdout(Stdio::null()))?;
    let result = mac_mounted(root, mnt, install, arch);
    let detached = Command::new("hdiutil")
        .arg("detach")
        .arg(mnt)
        .arg("-quiet")
        .status()
        .is_ok_and(|s| s.success());
    if !detached {
        let _ = Command::new("hdiutil")
            .arg("detach")
            .arg(mnt)
            .args(["-force", "-quiet"])
            .status();
    }
    result
}

fn mac_mounted(root: &Path, mnt: &Path, install: bool, arch: &str) -> Result<()> {
    let mut listing: Vec<String> = fs::read_dir(mnt)?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    listing.sort();
    ensure!(
        listing
            == [
                ".DS_Store",
                ".VolumeIcon.icns",
                ".background.tiff",
                "Applications",
                "Baylee.app"
            ],
        "unexpected disk image contents: {}",
        listing.join(" ")
    );
    ensure!(
        link_target(&mnt.join("Applications")) == "/Applications",
        "Applications is not a link to /Applications"
    );
    let macos = mnt.join("Baylee.app/Contents/MacOS");
    ensure!(
        link_target(&macos.join("assets")) == "../Resources/assets",
        "the bundle lost its assets link"
    );
    // Each program is the target's one slice: an Intel dmg holding an arm
    // build (or the reverse) would install and then refuse to start.
    let slice = if arch == "x86_64" { "x86_64" } else { "arm64" };
    for program in ["baylee-client", "baylee-runtime", "baylee-seat"] {
        need(&macos.join(program))?;
        let archs = output(Command::new("lipo").arg("-archs").arg(macos.join(program)))?;
        ensure!(
            archs.trim() == slice,
            "{program} is {}, not {slice}",
            archs.trim()
        );
    }
    ok(Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(mnt.join("Baylee.app")))?;
    // The bundle claims the `baylee:` link scheme (`package::info_plist`).
    let scheme = output(
        Command::new("plutil")
            .args(["-extract", "CFBundleURLTypes.0.CFBundleURLSchemes.0", "raw"])
            .arg(mnt.join("Baylee.app/Contents/Info.plist")),
    )?;
    ensure!(
        scheme.trim() == "baylee",
        "the bundle claims no baylee: scheme"
    );
    crate::installers::run(
        root,
        &["check-dmg-layout".into(), mnt.display().to_string()],
    )?;
    println!("dmg: layout and seal ok");
    if !install {
        return Ok(());
    }
    let app = Path::new("/Applications/Baylee.app");
    ensure!(
        !app.exists(),
        "{} exists already; this check runs on a clean runner",
        app.display()
    );
    ok(Command::new("ditto").arg(mnt.join("Baylee.app")).arg(app))?;
    let result = (|| {
        ok(Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(app))?;
        started_from(
            &app.join("Contents/MacOS/baylee-client"),
            &app.join("Contents/MacOS/assets"),
        )
    })();
    fs::remove_dir_all(app)?;
    result
}

fn windows(
    root: &Path,
    out: &Path,
    install: bool,
    target: &str,
    version: &str,
    arch: &str,
) -> Result<()> {
    let winarch = if arch == "x86_64" { "x64" } else { "arm64" };
    let setup = out.join(format!("Baylee-Setup-{version}-{winarch}.exe"));
    need(&setup)?;
    println!("setup: present");
    // Both programs carry the icon as a resource: Explorer, the Start menu's
    // pin, the title bar and the taskbar show it (owner, 08.10.2026). Read
    // off the archive's tree, the files the setup installs.
    let stage = out.join(format!("baylee-client-{version}-{target}"));
    super::run(
        root,
        &[
            "pe-icon".into(),
            stage.join("baylee-client.exe").display().to_string(),
            stage.join("baylee-runtime.exe").display().to_string(),
        ],
    )
    .context("an executable lacks the icon resource")?;
    if !install {
        return Ok(());
    }
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").context("no LOCALAPPDATA")?);
    let roaming = PathBuf::from(std::env::var_os("APPDATA").context("no APPDATA")?);
    let dir = local.join("Programs/Baylee");
    ensure!(
        !dir.exists(),
        "{} exists already; this check runs on a clean runner",
        dir.display()
    );
    ok(Command::new(&setup)
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
        .arg(format!("/LOG={}", out.join("setup.log").display())))?;
    for file in [
        "baylee-client.exe",
        "baylee-runtime.exe",
        "baylee-seat.exe",
        "baylee.ico",
        "LICENSE",
        "NOTICE",
        "README.txt",
        "assets/fonts/AlegreyaSans-Regular.ttf",
        "unins000.exe",
    ] {
        need(&dir.join(file))?;
    }
    need(&roaming.join("Microsoft/Windows/Start Menu/Programs/Baylee.lnk"))?;
    // The uninstall entry is per user: Settings > Apps lists it, and no
    // administrator was asked.
    ok(Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{01A111A7-93E6-7434-8D14-F893B3405EC8}_is1", "/v", "DisplayName"])
        .stdout(Stdio::null()))
    .context("no per-user uninstall entry")?;
    // The `baylee:` link scheme opens the launcher with the link.
    let command = output(Command::new("reg").args([
        "query",
        r"HKCU\Software\Classes\baylee\shell\open\command",
        "/ve",
    ]))
    .context("no baylee: link scheme")?;
    ensure!(
        command.contains(r#"Programs\Baylee\baylee-client.exe" "%1""#),
        "the baylee: scheme does not open the launcher: {command}"
    );
    ok(Command::new(dir.join("unins000.exe")).args([
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
    ]))?;
    // The uninstaller hands itself to a copy in %TEMP% and returns at once.
    let deadline = Instant::now() + Duration::from_secs(60);
    while dir.join("baylee-client.exe").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(1));
    }
    ensure!(
        !dir.join("baylee-client.exe").exists(),
        "uninstall left baylee-client.exe"
    );
    // Inno undoes its registry entries before its files: the scheme is gone.
    let scheme_left = Command::new("reg")
        .args(["query", r"HKCU\Software\Classes\baylee"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    ensure!(!scheme_left, "uninstall left the baylee: link scheme");
    println!("setup: installed per user and uninstalled");
    Ok(())
}

/// `xdg-open baylee://…` reaches /usr/bin/baylee once the MIME cache names
/// the installed entry. desktop-file-utils' dpkg trigger writes it; where the
/// trigger did not run (both arrived in one transaction), the cache is
/// rebuilt and that is said, so what is checked is still the entry.
fn deb_takes_links() -> Result<()> {
    let handles = || {
        fs::read_to_string("/usr/share/applications/mimeinfo.cache").is_ok_and(|cache| {
            cache
                .lines()
                .any(|l| l.starts_with("x-scheme-handler/baylee=") && l.contains("baylee.desktop"))
        })
    };
    if !handles() {
        println!(".deb: the MIME cache was not refreshed by the trigger; rebuilding it");
        ok(Command::new("sudo").args(["update-desktop-database", "/usr/share/applications"]))?;
    }
    ensure!(
        handles(),
        "the MIME cache names no handler for baylee: links"
    );
    Ok(())
}

fn linux(out: &Path, install: bool, version: &str, arch: &str) -> Result<()> {
    let debarch = if arch == "x86_64" { "amd64" } else { "arm64" };
    let image = fs::canonicalize(out.join(format!("Baylee-{version}-{arch}.AppImage")))
        .context("missing: the AppImage")?;
    let deb = out.join(format!("baylee_{version}_{debarch}.deb"));
    need(&deb)?;
    // Extracting needs no FUSE; the runtime is what a player's system runs.
    let tmp = tempfile::tempdir()?;
    ok(Command::new(&image)
        .arg("--appimage-extract")
        .current_dir(tmp.path())
        .stdout(Stdio::null()))?;
    let root = tmp.path().join("squashfs-root");
    ensure!(
        link_target(&root.join("AppRun")) == "opt/baylee/baylee-client",
        "AppRun is not the launcher"
    );
    for file in [
        "baylee-client",
        "baylee-runtime",
        "baylee-seat",
        "LICENSE",
        "NOTICE",
        "README.txt",
        "assets/fonts/AlegreyaSans-Regular.ttf",
    ] {
        need(&root.join("opt/baylee").join(file))?;
    }
    need(&root.join("baylee.desktop"))?;
    need(&root.join("baylee.png"))?;
    for n in LINUX_ICON_SIZES {
        need(&root.join(format!("usr/share/icons/hicolor/{n}x{n}/apps/baylee.png")))?;
    }
    // A Wayland dock pairs the window (app_id `baylee`, window_icon.rs) with
    // the entry of that name; X11 by WM_CLASS.
    let entry = fs::read_to_string(root.join("baylee.desktop"))?;
    ensure!(
        entry.lines().any(|l| l == "StartupWMClass=baylee"),
        "the AppImage's desktop entry names no WM class"
    );
    // The `baylee:` scheme, for a desktop that integrates the image: the
    // link reaches the launcher as its argument.
    ensure!(
        entry
            .lines()
            .any(|l| l == "MimeType=x-scheme-handler/baylee;")
            && entry.lines().any(|l| l == "Exec=baylee-client %u"),
        "the AppImage's desktop entry claims no baylee: links"
    );
    println!("AppImage: layout ok");
    output(Command::new("dpkg-deb").arg("--info").arg(&deb))?;
    let contents = output(Command::new("dpkg-deb").arg("--contents").arg(&deb))?;
    let has = |suffix: &str| contents.lines().any(|l| l.ends_with(suffix));
    ensure!(has("./opt/baylee/baylee-runtime"), ".deb has no runtime");
    ensure!(has("./opt/baylee/baylee-seat"), ".deb has no seat bridge");
    for n in LINUX_ICON_SIZES {
        ensure!(
            has(&format!(
                "./usr/share/icons/hicolor/{n}x{n}/apps/baylee.png"
            )),
            ".deb has no {n}px icon"
        );
    }
    println!(".deb: readable");
    if !install {
        return Ok(());
    }
    ok(Command::new("sudo")
        .args(["apt-get", "install", "-y", "--no-install-recommends"])
        .arg(fs::canonicalize(&deb)?)
        .arg("desktop-file-utils"))?;
    ensure!(
        fs::canonicalize("/usr/bin/baylee").ok().as_deref()
            == Some(Path::new("/opt/baylee/baylee-client")),
        "/usr/bin/baylee does not lead to the launcher"
    );
    for file in [
        "/opt/baylee/baylee-runtime",
        "/opt/baylee/baylee-seat",
        "/usr/share/applications/baylee.desktop",
    ] {
        need(Path::new(file))?;
    }
    for n in LINUX_ICON_SIZES {
        need(&PathBuf::from(format!(
            "/usr/share/icons/hicolor/{n}x{n}/apps/baylee.png"
        )))?;
    }
    ok(Command::new("desktop-file-validate").arg("/usr/share/applications/baylee.desktop"))?;
    deb_takes_links()?;
    // Root-owned, so the launcher will not install updates here; the client
    // only links to the release (`docs/releasing.md` §"Installers").
    let writable = Command::new("test")
        .args(["-w", "/opt/baylee"])
        .status()
        .is_ok_and(|s| s.success());
    ensure!(!writable, "/opt/baylee is writable for this user");
    ok(Command::new("sudo").args(["apt-get", "remove", "-y", "baylee"]))?;
    ensure!(
        !Path::new("/opt/baylee/baylee-client").exists(),
        "removing the package left /opt/baylee"
    );
    println!(".deb: installed and removed");
    Ok(())
}
