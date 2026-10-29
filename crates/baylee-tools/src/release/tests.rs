use super::{artifacts as files, policy::*};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    fs,
    path::Path,
};

const VERSION: &str = "0.1.0-beta.4";
fn identity() -> Identity {
    Identity {
        repository: "AceVik/baylee".into(),
        commit: "a".repeat(40),
    }
}
fn run() -> Value {
    json!({"id":123,"head_sha":"a".repeat(40),"head_branch":"main","event":"push","status":"completed","conclusion":"success","path":".github/workflows/ci.yml","repository":{"full_name":"AceVik/baylee"},"head_repository":{"full_name":"AceVik/baylee"}})
}
fn jobs() -> Vec<Value> {
    let mut jobs: Vec<_> = REQUIRED_JOBS
        .into_iter()
        .filter(|name| *name != "test")
        .map(|name| json!({"name":name,"conclusion":"success"}))
        .collect();
    jobs.extend([
        json!({"name":"test (1/2)","conclusion":"success"}),
        json!({"name":"test (2/2)","conclusion":"success"}),
        json!({"name":"build (macos-latest)","conclusion":"skipped"}),
    ]);
    jobs
}
fn artifacts() -> Vec<Value> {
    TARGETS
        .into_iter()
        .map(|(t, _)| json!({"name":format!("client-{t}"),"expired":false}))
        .collect()
}
fn api(responses: Vec<Value>) -> impl FnMut(&str) -> Result<Value> {
    let mut responses: VecDeque<_> = responses.into();
    move |path| {
        responses
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("unexpected API call: {path}"))
    }
}
fn verify(jobs: &[Value], artifacts: &[Value]) -> Result<()> {
    verify_run(
        "123",
        &identity(),
        &mut api(vec![
            run(),
            json!({"jobs":jobs}),
            json!({"artifacts":artifacts}),
        ]),
    )
}

#[test]
fn only_complete_successful_main_pushes_from_this_repository_and_sha_are_trusted() {
    assert!(trusted_run(&run(), &identity()));
    for (field, value) in [
        ("event", json!("pull_request")),
        ("event", json!("workflow_dispatch")),
        ("head_branch", json!("feature")),
        ("head_sha", json!("b".repeat(40))),
        ("status", json!("in_progress")),
        ("conclusion", json!("failure")),
        ("conclusion", json!("cancelled")),
        ("path", json!(".github/workflows/evil.yml")),
        ("repository", json!({"full_name":"fork/baylee"})),
        ("head_repository", json!({"full_name":"fork/baylee"})),
    ] {
        let mut changed = run();
        changed[field] = value;
        assert!(!trusted_run(&changed, &identity()), "{field}");
        assert!(verify_run("123", &identity(), &mut api(vec![changed])).is_err());
    }
    for field in run().as_object().unwrap().keys().filter(|k| *k != "id") {
        let mut changed = run();
        changed.as_object_mut().unwrap().remove(field);
        assert!(!trusted_run(&changed, &identity()), "missing {field}");
    }
    for id in ["", "../123", "12/3", "１２３"] {
        verify_run(id, &identity(), &mut |_| -> Result<Value> {
            panic!("invalid run ID reached API")
        })
        .unwrap_err();
    }
}

#[test]
fn every_required_job_and_every_matrix_row_must_run_and_succeed() {
    assert!(unproven_jobs(&jobs()).is_empty());
    verify(&jobs(), &artifacts()).unwrap();
    for name in REQUIRED_JOBS {
        let missing: Vec<_> = jobs()
            .into_iter()
            .filter(|j| !j["name"].as_str().unwrap().starts_with(name))
            .collect();
        assert!(unproven_jobs(&missing).contains(&name));
        assert!(verify(&missing, &artifacts()).is_err());
    }
    for conclusion in ["skipped", "failure", "cancelled", "neutral", "timed_out"] {
        for name in ["test-release", "test (2/2)"] {
            let mut changed = jobs();
            changed.iter_mut().find(|j| j["name"] == name).unwrap()["conclusion"] =
                json!(conclusion);
            assert!(
                verify(&changed, &artifacts()).is_err(),
                "{name}: {conclusion}"
            );
        }
    }
    assert!(
        unproven_jobs(&[json!({"name":"test-release","conclusion":"success"})]).contains(&"test")
    );
    let workflow = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/ci.yml"),
    )
    .unwrap();
    let jobs = workflow.split_once("\njobs:\n").unwrap().1;
    let ids: BTreeSet<_> = jobs
        .lines()
        .filter_map(|line| line.strip_prefix("  ").and_then(|v| v.strip_suffix(':')))
        .collect();
    for name in REQUIRED_JOBS {
        assert!(ids.contains(name), "CI renamed required job {name}");
    }
}

#[test]
fn artifact_set_is_complete_unique_and_explicitly_unexpired() {
    for index in 0..TARGETS.len() {
        let mut missing = artifacts();
        missing.remove(index);
        assert!(verify(&jobs(), &missing).is_err());
        let mut duplicate = artifacts();
        duplicate.push(duplicate[index].clone());
        assert!(verify(&jobs(), &duplicate).is_err());
        for expired in [json!(true), Value::Null, json!("false"), json!(0)] {
            let mut changed = artifacts();
            changed[index]["expired"] = expired;
            assert!(verify(&jobs(), &changed).is_err());
        }
    }
}

#[test]
fn pagination_checks_late_rows_and_resolution_does_not_fall_back() {
    let mut calls = Vec::new();
    let mut answers = VecDeque::from([
        json!({"jobs":vec![json!({"name":"other"});100]}),
        json!({"jobs":[{"name":"tail"}]}),
    ]);
    let collected = pages(
        &mut |path| {
            calls.push(path.to_owned());
            Ok(answers.pop_front().unwrap())
        },
        "jobs?filter=latest",
        "jobs",
    )
    .unwrap();
    assert_eq!(collected.len(), 101);
    assert_eq!(collected[100]["name"], "tail");
    assert_eq!(
        calls,
        [
            "jobs?filter=latest&per_page=100&page=1",
            "jobs?filter=latest&per_page=100&page=2"
        ]
    );
    assert!(pages(&mut |_| Ok(json!({})), "jobs", "jobs").is_err());
    assert!(pages(&mut |_| bail!("offline"), "jobs", "jobs").is_err());
    for field in ["head_sha", "event"] {
        let mut changed = run();
        changed[field] = json!("wrong");
        assert!(
            resolve(
                &identity(),
                &mut api(vec![json!({"workflow_runs":[changed]})])
            )
            .is_err()
        );
    }
    assert_eq!(
        resolve(
            &identity(),
            &mut api(vec![
                json!({"workflow_runs":[run()]}),
                run(),
                json!({"jobs":jobs()}),
                json!({"artifacts":artifacts()})
            ])
        )
        .unwrap(),
        "123"
    );
    assert!(
        resolve(
            &identity(),
            &mut api(vec![
                json!({"workflow_runs":[run(),run()]}),
                run(),
                json!({"jobs":[]})
            ])
        )
        .is_err()
    );
}

fn package(directory: &Path, target: &str) -> files::Names {
    let names = files::names(VERSION, target).unwrap();
    for name in names.files() {
        fs::write(directory.join(name), format!("fixture contents: {name}")).unwrap();
    }
    files::write(directory, VERSION, target, "123", &identity()).unwrap();
    names
}

#[test]
fn all_six_targets_roundtrip_and_names_match_the_installer_contract() {
    let mut installers = BTreeSet::new();
    for (target, _) in TARGETS {
        let tmp = tempfile::tempdir().unwrap();
        let names = package(tmp.path(), target);
        files::verify(tmp.path(), VERSION, target, "123", &identity()).unwrap();
        for name in names.installers {
            assert!(
                !Path::new(&name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zip")
                        || extension.eq_ignore_ascii_case("gz"))
            );
            installers.insert(name);
        }
    }
    assert_eq!(
        installers,
        [
            "Baylee-0.1.0-beta.4-aarch64.dmg",
            "Baylee-0.1.0-beta.4-x86_64.dmg",
            "Baylee-Setup-0.1.0-beta.4-x64.exe",
            "Baylee-Setup-0.1.0-beta.4-arm64.exe",
            "Baylee-0.1.0-beta.4-x86_64.AppImage",
            "Baylee-0.1.0-beta.4-aarch64.AppImage",
            "baylee_0.1.0-beta.4_amd64.deb",
            "baylee_0.1.0-beta.4_arm64.deb"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
    for version in ["../evil", "1.2.3\nextra", "1.2.3/a", "$(id)"] {
        assert!(files::names(version, TARGETS[0].0).is_err());
    }
    assert!(files::names(VERSION, "../../evil").is_err());
}

#[test]
fn every_identity_field_and_each_payload_is_verified() {
    let target = "x86_64-pc-windows-msvc";
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let names = package(dir, target);
    let path = dir.join(&names.manifest);
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (field, value) in [
        ("commit", json!("b".repeat(40))),
        ("version", json!("0.1.0-beta.3")),
        ("target", json!("aarch64-pc-windows-msvc")),
        ("run_id", json!("124")),
        ("repository", json!("fork/baylee")),
        ("schema", json!(1)),
        ("archive", json!("../payload.zip")),
        ("sha256", json!("0".repeat(64))),
        ("installers", json!({})),
        (
            "installers",
            json!({names.installers[0].clone():"0".repeat(64)}),
        ),
        ("extra", json!(true)),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(
            files::verify(dir, VERSION, target, "123", &identity()).is_err(),
            "{field}"
        );
    }
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    files::verify(dir, VERSION, target, "123", &identity()).unwrap();
    for name in names.files() {
        let path = dir.join(name);
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"tampered executable").unwrap();
        assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
        fs::write(path, original).unwrap();
    }
}

#[test]
fn missing_extra_nonregular_files_and_modified_checksums_are_rejected() {
    let target = TARGETS[0].0;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let names = package(dir, target);
    fs::write(dir.join("extra.exe"), b"extra").unwrap();
    assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
    fs::remove_file(dir.join("extra.exe")).unwrap();
    for name in names.files() {
        let path = dir.join(format!("{name}.sha256"));
        let saved = fs::read(&path).unwrap();
        fs::write(&path, b"wrong").unwrap();
        assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
        fs::remove_file(&path).unwrap();
        assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
        fs::create_dir(&path).unwrap();
        assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
        fs::remove_dir(&path).unwrap();
        fs::write(path, saved).unwrap();
    }
    #[cfg(unix)]
    {
        let name = &names.installers[0];
        let saved = fs::read(dir.join(name)).unwrap();
        fs::remove_file(dir.join(name)).unwrap();
        std::os::unix::fs::symlink(&names.archive, dir.join(name)).unwrap();
        assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
        fs::remove_file(dir.join(name)).unwrap();
        fs::write(dir.join(name), saved).unwrap();
    }
    fs::remove_file(dir.join(&names.installers[1])).unwrap();
    assert!(files::verify(dir, VERSION, target, "123", &identity()).is_err());
}

#[test]
fn installer_table_requires_all_eight_verified_checksums() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    for (target, _) in TARGETS {
        package(dir, target);
    }
    let table = files::installer_sums(dir, VERSION).unwrap();
    assert_eq!(table.lines().count(), 10);
    let name = "Baylee-0.1.0-beta.4-aarch64.dmg";
    assert!(table.contains(&format!(
        "| `{name}` | `{}` |",
        files::digest(&dir.join(name)).unwrap()
    )));
    let sidecar = dir.join(format!("{name}.sha256"));
    let saved = fs::read(&sidecar).unwrap();
    fs::write(&sidecar, format!("{}  other.dmg\n", "0".repeat(64))).unwrap();
    assert!(files::installer_sums(dir, VERSION).is_err());
    fs::write(&sidecar, saved).unwrap();
    fs::write(dir.join(name), b"tampered").unwrap();
    assert!(files::installer_sums(dir, VERSION).is_err());
    fs::remove_file(sidecar).unwrap();
    assert!(files::installer_sums(dir, VERSION).is_err());
    fs::write(dir.join("known"), b"abc").unwrap();
    assert_eq!(
        files::digest(&dir.join("known")).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn rust_sources(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
}

/// One alias's command line from `.cargo/config.toml`.
fn cargo_alias(root: &Path, name: &str) -> String {
    let config = fs::read_to_string(root.join(".cargo/config.toml")).unwrap();
    let prefix = format!("{name} = \"");
    config
        .lines()
        .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
        .unwrap_or_else(|| panic!("no cargo alias {name}"))
        .to_owned()
}

#[cfg(unix)]
/// The packaging itself is `package`'s test; here, that every release build
/// makes the seat bridge it then packs (beta.6 shipped without it).
#[test]
fn every_release_build_makes_the_seat_bridge() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workflow = fs::read_to_string(root.join(".github/workflows/client-packages.yml")).unwrap();
    assert!(
        workflow
            .lines()
            .any(|l| l.contains("cargo build-client ") && l.contains("--profile dist"))
    );
    assert!(cargo_alias(&root, "build-client").contains("--bin baylee-seat"));
}

/// `cargo check-client` checks every target of the shipped client's crates on
/// every platform, since the servers and tools are built for Linux `x86_64`
/// only. A client crate that carries a platform `cfg` and is missing from the
/// alias would have its tests meet only that one platform. Pointer width is
/// not a platform difference here: every target is 64-bit.
#[test]
fn client_check_covers_every_platform_cfg() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let check = cargo_alias(&root, "check-client");
    let words: Vec<_> = check.split_whitespace().collect();
    let checked: BTreeSet<_> = words
        .windows(2)
        .filter(|pair| pair[0] == "-p")
        .map(|pair| pair[1].to_owned())
        .collect();

    // The workspace crates the shipped binaries link, through every normal
    // dependency table, target-specific ones included.
    let dependencies = |name: &str| -> Vec<String> {
        let manifest = fs::read_to_string(root.join("crates").join(name).join("Cargo.toml"))
            .unwrap_or_default();
        let mut normal = false;
        let mut found = Vec::new();
        for line in manifest.lines() {
            if line.starts_with('[') {
                normal = line.ends_with("dependencies]")
                    && !line.contains("dev-dependencies")
                    && !line.contains("build-dependencies");
            } else if normal {
                // A renamed dependency names its crate in `package`, as
                // `keyring = { package = "baylee-keyring", ... }` does.
                let renamed = line
                    .split_once("package = \"")
                    .and_then(|(_, rest)| rest.split_once('"'))
                    .map(|(crate_name, _)| crate_name);
                let key = line.split(['.', ' ', '=']).next().unwrap_or_default();
                let name = renamed.unwrap_or(key);
                if name.starts_with("baylee-") {
                    found.push(name.to_owned());
                }
            }
        }
        found
    };
    let mut client = BTreeSet::new();
    let mut queue: VecDeque<String> = ["baylee-client", "baylee-update", "baylee-seat"]
        .map(str::to_owned)
        .into();
    while let Some(name) = queue.pop_front() {
        if client.insert(name.clone()) {
            queue.extend(dependencies(&name));
        }
    }
    assert!(
        client.contains("baylee-engine"),
        "the walk found {client:?}"
    );

    // `cfg`, `cfg!` and `cfg_attr`, over line breaks inside the brackets;
    // a manifest's `[target.'cfg(…)'.dependencies]` counts as well.
    let platform = regex::Regex::new(
        r"(?s)cfg(?:_attr)?!?\([^\]]{0,300}?\b(target_os|target_arch|target_family|target_env|target_vendor|windows|unix)\b",
    )
    .unwrap();
    let mut needed = BTreeSet::new();
    for name in &client {
        let mut files = Vec::new();
        rust_sources(&root.join("crates").join(name), &mut files);
        let manifest = fs::read_to_string(root.join("crates").join(name).join("Cargo.toml"))
            .unwrap_or_default();
        if manifest
            .lines()
            .any(|line| line.starts_with("[target.'cfg("))
            || files
                .iter()
                .any(|file| platform.is_match(&fs::read_to_string(file).unwrap_or_default()))
        {
            needed.insert(name.clone());
        }
    }
    assert!(
        needed.contains("baylee-keyring"),
        "the scan found {needed:?}"
    );
    let missing: Vec<_> = needed.difference(&checked).collect();
    assert!(missing.is_empty(), "add to check-client: {missing:?}");
    let foreign: Vec<_> = checked.difference(&client).collect();
    assert!(foreign.is_empty(), "not part of the client: {foreign:?}");
}

#[test]
fn help_aliases_succeed_without_ci_environment_or_package_files() {
    let temporary = tempfile::tempdir().unwrap();
    let missing_root = temporary.path().join("absent");
    for flag in ["--help", "-h"] {
        super::run(&missing_root, &[flag.into()]).unwrap();
    }
    assert!(!missing_root.exists());
    assert!(super::run(&missing_root, &["unknown-command".into()]).is_err());
}

/// `cargo deny` and `cargo audit` ignore the same advisories. The audit
/// action reads no `deny.toml`, so the list is written twice, and it drifted
/// once: RUSTSEC-2026-0387 reached `audit` only after it had failed a run.
#[test]
fn deny_and_audit_ignore_the_same_advisories() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ids = |text: &str| -> BTreeSet<String> {
        regex::Regex::new(r"RUSTSEC-\d{4}-\d{4}")
            .unwrap()
            .find_iter(text)
            .map(|m| m.as_str().to_owned())
            .collect()
    };
    let deny = fs::read_to_string(root.join("deny.toml")).unwrap();
    let ignored = deny
        .split_once("ignore = [")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(list, _)| list)
        .expect("deny.toml has an advisories ignore list");
    // Only the quoted entries, not the ids the comments mention.
    let deny_ids: BTreeSet<String> = ignored
        .lines()
        .filter(|line| line.trim_start().starts_with('"'))
        .flat_map(ids)
        .collect();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let audit_line = workflow
        .lines()
        .find(|line| line.trim_start().starts_with("ignore: RUSTSEC-"))
        .expect("the audit job lists what it ignores");
    assert!(!deny_ids.is_empty());
    assert_eq!(ids(audit_line), deny_ids);
}
