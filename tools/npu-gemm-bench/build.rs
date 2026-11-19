//! Compiles the XRT shim and links `libxrt_coreutil`.
//!
//! `XRT_ROOT` names the prefix that holds `include/xrt/` and
//! `lib/x86_64-linux-gnu/libxrt_coreutil.so` (default `/usr`, where Ubuntu's
//! `libxrt-dev` puts them). A build host without XRT installed can point it
//! at the unpacked `libxrt-dev` and `libxrt2` packages instead.

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=XRT_ROOT");
    println!("cargo:rerun-if-changed=shim/xrt_shim.cpp");
    let root = env::var("XRT_ROOT").unwrap_or_else(|_| "/usr".to_owned());
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("shim/xrt_shim.cpp")
        // XRT's own headers trip -Wextra; keep the warnings for the shim.
        .flag(format!("-isystem{root}/include"))
        .warnings(true)
        .compile("xrt_shim");
    println!("cargo:rustc-link-search=native={root}/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-lib=dylib=xrt_coreutil");
}
