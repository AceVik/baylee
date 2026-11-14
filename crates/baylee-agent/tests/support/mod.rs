//! A harmless Rust executable for process-boundary tests; no shell interpreter.
use std::path::{Path, PathBuf};

pub fn identity() -> baylee_protocol::execution::Identity {
    baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::StandardLoops,
        "test-build",
        "test-revision",
        19,
    )
}

pub fn executable(dir: &Path) -> PathBuf {
    let source = dir.join("fixture.rs");
    let binary = dir.join(format!("fake-engine{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(dir.join("metadata"), identity().to_metadata()).unwrap();
    std::fs::write(&source, r#"
#![forbid(unsafe_code)]
fn main() {
    use std::io::Write as _;
    let executable = std::env::current_exe().unwrap();
    let dir = executable.parent().unwrap();
    if std::env::args().nth(1).as_deref() == Some("--metadata") {
        assert!(std::env::var_os("BAYLEE_AGENT_TOKEN").is_none());
        assert!(std::env::var_os("BAYLEE_ENGINE_TOKEN").is_none());
        std::fs::write(dir.join("probe.pid"), std::process::id().to_string()).unwrap();
        let mode = std::fs::read_to_string(dir.join("mode")).unwrap_or_default();
        match mode.as_str() {
            "huge" => println!("{}", "x".repeat(8192)),
            "timeout" => std::thread::sleep(std::time::Duration::from_secs(60)),
            "failure" => std::process::exit(1),
            _ => println!("{}", std::fs::read_to_string(dir.join("metadata")).unwrap()),
        }
        return;
    }
    assert!(std::env::var_os("BAYLEE_AGENT_TOKEN").is_none());
    std::fs::write(std::env::var_os("BAYLEE_TEST_CHILD_PID").unwrap(), std::process::id().to_string()).unwrap();
    let mode = std::fs::read_to_string(dir.join("mode")).unwrap_or_default();
    match mode.as_str() {
        "start-stall" => {},
        "busy" => { std::io::stdout().write_all(&[2, 1]).unwrap(); },
        "invalid-progress" => { std::io::stdout().write_all(&[255]).unwrap(); },
        _ => { std::io::stdout().write_all(&[2]).unwrap(); },
    }
    std::io::stdout().flush().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(60));
}
"#).unwrap();
    let output = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture compiler: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    binary
}
