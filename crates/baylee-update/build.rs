//! The launcher's Windows icon resource.
//!
//! `baylee-launch.exe` is what a player starts: the package names it
//! `baylee-client.exe`, and the Start-menu entry, Explorer and a pinned
//! taskbar button all show *its* icon. An executable without an icon
//! resource shows Windows' generic one, which is what 0.1.0-beta.3 did
//! (owner, 08.10.2026). The client's own `build.rs` does the same for the
//! runtime it starts; the icon is `scripts/installers/make-art.py`'s.
//!
//! `winresource` is a build-dependency only on a Windows *host* (Cargo
//! evaluates a build-dependency's `cfg` for the machine the build script runs
//! on), so this script names it only there. A cross-build for Windows from
//! another host links no icon, and says so; the shipped executables are
//! built on Windows (`client-packages.yml`).

fn main() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if windows {
        icon();
    }
}

#[cfg(windows)]
fn icon() {
    let ico = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/installers/windows/baylee.ico"
    );
    println!("cargo::rerun-if-changed={ico}");
    println!("cargo::rerun-if-changed=build.rs");
    let mut resource = winresource::WindowsResource::new();
    // Ordinal 1, the one Explorer takes as the program's icon.
    resource.set_icon(ico);
    // Task Manager and the taskbar's menu name a program by these.
    resource.set("FileDescription", "Baylee");
    resource.set("ProductName", "Baylee");
    if let Err(err) = resource.compile() {
        panic!("could not embed the Windows icon ({ico}): {err}");
    }
}

#[cfg(not(windows))]
fn icon() {
    println!(
        "cargo::warning=baylee-launch: a Windows build from a non-Windows host carries no icon resource"
    );
}
