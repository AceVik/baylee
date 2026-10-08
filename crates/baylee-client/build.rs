//! Two compile-time fallbacks and the Windows icon resource.
//!
//! On a phone there is no environment: an app started by `am start` or by a
//! tap on an icon inherits nothing, so `BAYLEE_GATEWAY` and
//! `BAYLEE_DEV_CONTROL` cannot be read at runtime there and are baked in by
//! `option_env!` instead (`settings::gateway_url`, `devctl::baked_port`).
//!
//! `option_env!` reads the variable **when the crate is compiled**, and cargo
//! does not know that — without the lines below, changing the gateway address
//! and rebuilding would hand the phone the address from two days ago, with
//! nothing anywhere saying so. Only for the two targets that need it: a
//! desktop build that declared the same dependency would recompile the whole
//! client every time somebody exported `BAYLEE_GATEWAY` to *run* it.
//!
//! On Windows the client's executable (`baylee-runtime.exe` in a package)
//! carries the icon as resource 1: Explorer shows it, and `window_icon.rs`
//! loads it for the title bar and the taskbar button. An executable without
//! one shows Windows' generic icon, which is what 0.1.0-beta.3 did (owner,
//! 08.10.2026). `winresource` is a build-dependency only on a Windows *host*
//! (Cargo evaluates a build-dependency's `cfg` for the machine the build
//! script runs on), so it is named only there; the shipped executables are
//! built on Windows (`client-packages.yml`), and the launcher's `build.rs`
//! (`baylee-update`) does the same for `baylee-launch.exe`.

fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "android" || target == "ios" {
        println!("cargo::rerun-if-env-changed=BAYLEE_GATEWAY");
        println!("cargo::rerun-if-env-changed=BAYLEE_DEV_CONTROL");
    }
    if target == "windows" {
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
    // Ordinal 1: Explorer's choice, and `window_icon::RESOURCE`.
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
        "cargo::warning=baylee-client: a Windows build from a non-Windows host carries no icon resource"
    );
}
