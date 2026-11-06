//! What a profile-guided build needs stood in, for a test that stages.

use std::path::Path;

/// `cargo`: says it built, and a build given a target directory leaves the
/// workload and the engine server there, as the real one does.
pub const CARGO: &str = r#"echo built
if [ -n "$CARGO_TARGET_DIR" ]; then
  mkdir -p "$CARGO_TARGET_DIR/release"
  printf '#!/bin/sh\necho same games\n' > "$CARGO_TARGET_DIR/release/engine_perf"
  chmod +x "$CARGO_TARGET_DIR/release/engine_perf"
  : > "$CARGO_TARGET_DIR/release/baylee-engine-server"
fi"#;

/// Writes `rustc` and the `llvm-profdata` it finds through `stub`. The tool
/// fails while `PROFILE_FAILS` is set, which stands for any step of the build.
pub fn install(stubs: &Path, stub: impl Fn(&Path, &str, &str)) {
    let sysroot = stubs.join("sysroot");
    let tools = sysroot.join("lib/rustlib/x/bin");
    std::fs::create_dir_all(&tools).unwrap();
    stub(
        stubs,
        "rustc",
        &format!(
            r#"[ "$1" = --print ] && echo {}; [ "$1" = -vV ] && echo "host: x"; exit 0"#,
            sysroot.display()
        ),
    );
    stub(
        &tools,
        "llvm-profdata",
        r#"[ -n "$PROFILE_FAILS" ] && exit 9; [ "$1" = merge ] && : > "$3"
[ "$1" = show ] && echo "Total functions: 100"; exit 0"#,
    );
}
