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
            .any(|l| l.contains("cargo build ") && l.contains("--bin baylee-seat"))
    );
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
