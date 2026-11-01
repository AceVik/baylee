//! A real Cargo resolution proves the guard includes optional consumers.
use super::*;
use std::{fs, path::PathBuf, process::Output};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "baylee-bigint-scope.{}",
            crate::secret::hex(8).unwrap()
        ));
        fs::create_dir(&path).unwrap();
        let fixture = Self(path);
        fs::create_dir_all(fixture.0.join("app/src")).unwrap();
        fs::create_dir_all(fixture.0.join("integer/src")).unwrap();
        for (file, text) in [
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"app\", \"integer\"]\nresolver = \"3\"\n",
            ),
            (
                "app/Cargo.toml",
                "[package]\nname = \"optional-consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
                 [dependencies]\ncrypto-bigint = { path = \"../integer\", optional = true }\n",
            ),
            (
                "integer/Cargo.toml",
                "[package]\nname = \"crypto-bigint\"\nversion = \"0.5.5\"\nedition = \"2024\"\n",
            ),
            ("app/src/lib.rs", "// Metadata fixture; never compiled.\n"),
            (
                "integer/src/lib.rs",
                "// Metadata fixture; never compiled.\n",
            ),
        ] {
            fs::write(fixture.0.join(file), text).unwrap();
        }
        fixture
    }

    fn run(&self, command: &mut Command) -> Output {
        let output = command.current_dir(&self.0).output().unwrap();
        assert!(output.status.success(), "{}", run::text(&output));
        output
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_disabled_optional_consumer_is_still_rejected() {
    let fixture = Fixture::new();
    fixture.run(&mut cargo_with(["generate-lockfile", "--offline"]));
    let default = fixture.run(&mut cargo_with([
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
    ]));
    let default: Value = serde_json::from_slice(&default.stdout).unwrap();
    let target = default["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "crypto-bigint")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    // The control: a default-only guard would miss the optional edge.
    expect_callers(default["resolve"]["nodes"].as_array().unwrap(), target, &[]).unwrap();
    let all = fixture.run(metadata_command().arg("--offline"));
    let all: Value = serde_json::from_slice(&all.stdout).unwrap();
    assert!(expect_callers(all["resolve"]["nodes"].as_array().unwrap(), target, &[]).is_err());
}
