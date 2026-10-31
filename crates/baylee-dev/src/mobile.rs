//! Phone builds and the LAN gateway they dial (`docs/mobile.md` is normative).
//!
//! Which gateway address is right depends on where the client runs, and that
//! is the thing guessed wrong:
//!
//! | client | address |
//! | --- | --- |
//! | a phone on WiFi | `http://<this machine's LAN address>:28766` |
//! | a phone over USB | `http://127.0.0.1:28766` after `adb reverse` |
//! | the Android emulator | `http://10.0.2.2:28766` (10.0.2.2 is the host) |
//! | the iOS simulator | `http://127.0.0.1:28766` (it shares the host stack) |
use crate::run::{self, cargo_with};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    net::{IpAddr, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
};

/// The API level everything agrees on: cargo-apk's default and the Play
/// Store's ask; `bevy_audio` needs 26 or newer.
const ANDROID_API: &str = "33";
/// The NDK the build is pinned to.
const NDK_VERSION: &str = "28.0.12674087";
const GATEWAY_PORT: u16 = 28766;
const BUNDLE_ID: &str = "local.baylee.client";

/// This machine's address on the network a phone would use: the source
/// address of a route outwards. A UDP `connect` sends nothing; it only asks
/// the system which interface it would use.
pub fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// The environment an Android build needs, never left in a shell profile: an
/// `ANDROID_NDK_ROOT` set for every terminal is how a desktop build picks up a
/// cross compiler by accident. Each value keeps a caller's own setting.
pub fn android_env() -> Result<Vec<(String, String)>> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("no home directory")?;
    let keep = |name: &str, default: PathBuf| {
        std::env::var(name).unwrap_or_else(|_| default.display().to_string())
    };
    let sdk = keep("ANDROID_HOME", default_sdk(&home));
    let ndk = keep(
        "ANDROID_NDK_ROOT",
        Path::new(&sdk).join("ndk").join(NDK_VERSION),
    );
    let api = std::env::var("BAYLEE_ANDROID_API").unwrap_or_else(|_| ANDROID_API.into());
    let bin = Path::new(&ndk)
        .join("toolchains/llvm/prebuilt")
        .join(host_tag())
        .join("bin");
    let tool = |name: String| bin.join(name).display().to_string();
    let ext = if cfg!(windows) { ".cmd" } else { "" };
    let cc = tool(format!("aarch64-linux-android{api}-clang{ext}"));
    let mut env = vec![
        ("ANDROID_HOME".into(), sdk.clone()),
        ("ANDROID_SDK_ROOT".into(), sdk.clone()),
        ("ANDROID_NDK_ROOT".into(), ndk.clone()),
        ("BAYLEE_ANDROID_API".into(), api.clone()),
        // `cc` for the C and C++ build scripts under this target: oboe-sys,
        // the Android audio backend behind bevy_audio, builds C++ via cmake.
        ("CC_aarch64_linux_android".into(), cc.clone()),
        (
            "CXX_aarch64_linux_android".into(),
            tool(format!("aarch64-linux-android{api}-clang++{ext}")),
        ),
        ("AR_aarch64_linux_android".into(), tool("llvm-ar".into())),
        (
            "RANLIB_aarch64_linux_android".into(),
            tool("llvm-ranlib".into()),
        ),
        ("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER".into(), cc),
    ];
    // `apksigner` is a Java program, and `/usr/libexec/java_home` does not
    // find the Homebrew JDK, so JAVA_HOME is named rather than discovered.
    let java = std::env::var("JAVA_HOME").ok().or_else(|| {
        let brew = Path::new("/opt/homebrew/opt/openjdk@17");
        brew.is_dir().then(|| brew.display().to_string())
    });
    let mut path = vec![
        Path::new(&sdk).join("platform-tools"),
        Path::new(&sdk).join("emulator"),
        Path::new(&sdk).join("cmake/3.31.1/bin"),
    ];
    if let Some(java) = java {
        path.push(Path::new(&java).join("bin"));
        env.push(("JAVA_HOME".into(), java));
    }
    path.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    env.push((
        "PATH".into(),
        std::env::join_paths(path)?.to_string_lossy().into_owned(),
    ));
    Ok(env)
}

fn default_sdk(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Android/sdk")
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map_or_else(|| home.join("AppData/Local"), PathBuf::from)
            .join("Android/Sdk")
    } else {
        home.join("Android/Sdk")
    }
}

/// The NDK's prebuilt toolchain directory for this host (x86-64 on every
/// host the NDK ships for; Apple Silicon runs it under Rosetta).
fn host_tag() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else if cfg!(windows) {
        "windows-x86_64"
    } else {
        "linux-x86_64"
    }
}

/// Prints the Android environment for a shell to take over by hand.
pub fn print_env() -> Result<i32> {
    for (name, value) in android_env()? {
        if cfg!(windows) {
            println!("$env:{name} = '{}'", value.replace('\'', "''"));
        } else {
            println!("export {name}='{}'", value.replace('\'', r"'\''"));
        }
    }
    Ok(0)
}

/// Options for an APK build.
pub struct Android {
    pub gateway: Option<String>,
    pub devctl: u16,
    pub dev_control: bool,
    pub run: bool,
    pub release: bool,
}

/// Builds (and with `run` installs and starts) the client as an APK. Two
/// values are compiled in, because an Android app inherits no environment:
/// the gateway it dials and dev-control's port. Changing the gateway means a
/// rebuild, and `crates/baylee-client/build.rs` is what makes cargo notice.
pub fn android(options: &Android) -> Result<i32> {
    let gateway = options.gateway.clone().unwrap_or_else(|| {
        let ip = lan_address().map_or_else(|| "127.0.0.1".into(), |ip| ip.to_string());
        format!("http://{ip}:{GATEWAY_PORT}")
    });
    let devctl = options.dev_control.then_some(options.devctl);
    println!("gateway      {gateway}");
    println!(
        "dev-control  {}\n",
        devctl.map_or_else(|| "off".into(), |p| p.to_string())
    );

    // `cargo apk run` also installs and starts it, and applies the
    // `reverse_port_forward` table from the android package's Cargo.toml.
    let mut build = cargo_with([
        "apk",
        if options.run { "run" } else { "build" },
        "-p",
        "baylee-client-android",
    ]);
    if options.dev_control {
        build.args(["--features", "dev-control"]);
    }
    if options.release {
        build.arg("--release");
    }
    build.envs(android_env()?);
    build.env("BAYLEE_GATEWAY", &gateway);
    build.env(
        "BAYLEE_DEV_CONTROL",
        devctl.map(|p| p.to_string()).unwrap_or_default(),
    );
    let rc = run::code(run::status(&mut build));
    if let Some(apk) = newest_apk() {
        println!("\nAPK: {}\n", apk.display());
        println!("  install by hand   adb install -r {}", apk.display());
        if let Some(port) = devctl {
            println!(
                "  reach dev-control adb forward tcp:28773 tcp:{port} && curl -s localhost:28773/health"
            );
        }
        println!("  let it dial here  adb reverse tcp:{GATEWAY_PORT} tcp:{GATEWAY_PORT}");
    }
    Ok(rc)
}

fn newest_apk() -> Option<PathBuf> {
    ["target/debug/apk", "target/release/apk"]
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "apk"))
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
}

/// Builds the client for the iOS simulator, wraps it in an `.app` and starts
/// it. No Xcode project, deliberately: a simulator app is a directory with a
/// Mach-O binary and an `Info.plist`, nothing is signed, and `simctl install`
/// takes it as it is. The simulator shares this machine's network stack, so
/// the gateway and dev-control are plain loopback with nothing forwarded.
pub fn ios_sim(device: &str, gateway: &str, devctl: u16) -> Result<i32> {
    if !cfg!(target_os = "macos") {
        bail!("the iOS simulator runs on macOS only");
    }
    let target = "aarch64-apple-ios-sim";
    println!("building for {target}");
    if !run::succeeds(&mut cargo_with([
        "build",
        "-p",
        "baylee-client",
        "--features",
        "dev-control",
        "--target",
        target,
    ])) {
        return Ok(1);
    }
    let app = Path::new("target/ios/Baylee.app");
    if app.exists() {
        fs::remove_dir_all(app)?;
    }
    fs::create_dir_all(app)?;
    fs::copy(
        format!("target/{target}/debug/baylee-client"),
        app.join("baylee-client"),
    )?;
    // Beside the executable, where bevy looks when neither BEVY_ASSET_ROOT
    // nor CARGO_MANIFEST_DIR is set (`standalone::asset_root`).
    copy_dir(
        Path::new("crates/baylee-client/assets"),
        &app.join("assets"),
    )?;
    fs::write(app.join("Info.plist"), IOS_PLIST)?;

    let udid = simulator_udid(device)?;
    println!("simulator {device} {udid}");
    let _ = Command::new("xcrun")
        .args(["simctl", "boot", &udid])
        .output();
    let _ = Command::new("open").args(["-a", "Simulator"]).status();
    if !run::succeeds(
        Command::new("xcrun")
            .args(["simctl", "install", &udid])
            .arg(app),
    ) {
        bail!("simctl install failed");
    }
    // `simctl` passes an environment through one SIMCTL_CHILD_ prefix at a
    // time, which is why iOS needs no compiled-in address as Android does.
    let mut launch = Command::new("xcrun");
    launch
        .args(["simctl", "launch", "--console-pty", &udid, BUNDLE_ID])
        .env("SIMCTL_CHILD_BAYLEE_GATEWAY", gateway)
        .env("SIMCTL_CHILD_BAYLEE_DEV_CONTROL", devctl.to_string());
    Ok(run::code(run::status(&mut launch)))
}

/// The UDID of an available simulator whose line names the device.
fn simulator_udid(device: &str) -> Result<String> {
    let out = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()?;
    let list = String::from_utf8_lossy(&out.stdout);
    let wanted = format!("{device} (");
    list.lines()
        .filter(|line| line.trim_start().starts_with(&wanted))
        .find_map(|line| {
            line.split(['(', ')'])
                .map(str::trim)
                .find(|part| {
                    part.len() == 36 && part.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
                })
                .map(str::to_owned)
        })
        .with_context(|| {
            format!("no simulator called '{device}'; see `xcrun simctl list devices available`")
        })
}

pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The gateway and one agent, reachable from a phone on the same network.
/// The gateway already binds 0.0.0.0; what this adds is the address to use
/// from each surface. Needs PostgreSQL like any gateway (`DATABASE_URL`).
pub fn serve_lan(print_only: bool) -> Result<i32> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(GATEWAY_PORT);
    let lan = lan_address().map(|ip| ip.to_string());
    let shown = lan.as_deref().unwrap_or("?");
    let addresses = || {
        println!("\n  gateway port      {port}");
        println!(
            "  this machine      {}",
            lan.as_deref().unwrap_or("<no LAN address>")
        );
        println!("\n  phone on WiFi     http://{shown}:{port}");
        println!(
            "  phone over USB    http://127.0.0.1:{port}   (adb reverse tcp:{port} tcp:{port})"
        );
        println!("  Android emulator  http://10.0.2.2:{port}");
        println!("  iOS simulator     http://127.0.0.1:{port}\n");
        println!("  Build the APK against one of those:");
        println!("    cargo dev android --gateway http://{shown}:{port}\n");
    };
    if print_only {
        addresses();
        return Ok(0);
    }
    run::require_env(
        "DATABASE_URL",
        "the gateway refuses to start without PostgreSQL",
    )?;
    if !run::succeeds(&mut cargo_with([
        "build",
        "-p",
        "baylee-gateway",
        "-p",
        "baylee-agent",
        "-p",
        "baylee-engine-server",
    ])) {
        return Ok(1);
    }
    // One secret for both halves: a gateway with no agent hosts no games
    // (`POST /lobby/games` answers 503), so the two start together.
    let token = match std::env::var("BAYLEE_AGENT_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => crate::secret::hex(32)?,
    };
    let logs = std::env::temp_dir().join("baylee-lan");
    fs::create_dir_all(&logs)?;
    let exe = |name: &str| {
        PathBuf::from("target/debug").join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
    };
    let log = |name: &str| fs::File::create(logs.join(format!("{name}.log")));
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let mut gateway = Command::new(exe("baylee-gateway"))
        .env("PORT", port.to_string())
        .env("BAYLEE_AGENT_TOKEN", &token)
        // No Central here: the phone comes in as a guest.
        .env("BAYLEE_AUTH_MODE", "guests")
        .env("RUST_LOG", &rust_log)
        .stdout(log("gateway")?)
        .stderr(log("gateway")?)
        .spawn()
        .context("starting the gateway")?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut agent = Command::new(exe("baylee-agent"))
        .env("BAYLEE_GATEWAY", format!("http://127.0.0.1:{port}"))
        .env("BAYLEE_AGENT_TOKEN", &token)
        .env(
            "BAYLEE_ENGINE_BIN",
            std::fs::canonicalize(exe("baylee-engine-server"))?,
        )
        .env("RUST_LOG", &rust_log)
        .stdout(log("agent")?)
        .stderr(log("agent")?)
        .spawn()
        .context("starting the agent")?;
    println!("gateway pid {}, agent pid {}", gateway.id(), agent.id());
    println!("logs in {}", logs.display());
    addresses();
    println!("stop both with Ctrl-C");
    let status = gateway.wait()?;
    let _ = agent.kill();
    let _ = agent.wait();
    Ok(run::code(Some(status)))
}

const IOS_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>baylee-client</string>
    <key>CFBundleIdentifier</key><string>local.baylee.client</string>
    <key>CFBundleName</key><string>Baylee</string>
    <key>CFBundleDisplayName</key><string>Baylee</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleSupportedPlatforms</key><array><string>iPhoneSimulator</string></array>
    <key>MinimumOSVersion</key><string>16.0</string>
    <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
    <!-- An empty launch-screen dict stops iOS from running the app in a
         letterboxed compatibility window at the wrong resolution. -->
    <key>UILaunchScreen</key><dict/>
    <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>
    <!-- A table is played across, not down: both landscape directions and
         neither portrait one, as the Android shim's `sensorLandscape`. -->
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UIStatusBarHidden</key><true/>
    <key>UIApplicationSupportsIndirectInputEvents</key><true/>
</dict>
</plist>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_android_environment_names_this_hosts_ndk_toolchain() {
        let env = android_env().unwrap();
        let get = |name: &str| {
            env.iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .unwrap()
        };
        assert!(get("CC_aarch64_linux_android").contains(host_tag()));
        assert!(
            get("CC_aarch64_linux_android")
                .contains(&format!("android{}-clang", get("BAYLEE_ANDROID_API")))
        );
        assert_eq!(get("ANDROID_SDK_ROOT"), get("ANDROID_HOME"));
        assert_eq!(
            get("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"),
            get("CC_aarch64_linux_android")
        );
    }

    #[test]
    fn a_lan_address_is_never_loopback() {
        assert!(lan_address().is_none_or(|ip| !ip.is_loopback()));
    }
}
