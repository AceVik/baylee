//! Two compile-time fallbacks, and the one thing cargo has to be told about
//! them.
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

fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "android" || target == "ios" {
        println!("cargo::rerun-if-env-changed=BAYLEE_GATEWAY");
        println!("cargo::rerun-if-env-changed=BAYLEE_DEV_CONTROL");
    }
}
