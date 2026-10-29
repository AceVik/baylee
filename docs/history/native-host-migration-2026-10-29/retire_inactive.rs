//! Explicit owner-authorized retirement of exactly five inactive historical units.
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
type R<T> = Result<T, Box<dyn std::error::Error>>;
const UNITS: [&str; 5] = [
    "baylee-arena-v5.service",
    "baylee-arena-v5-lane2.service",
    "baylee-continuous-v5-training.service",
    "baylee-v5-full-coverage-eval.service",
    "baylee-terra-flash-balanced.service",
];
fn hash(path: &Path) -> R<String> {
    Ok(baylee_train_ops::files::sha256(path).map_err(|e| e.to_string())?)
}
fn write(path: &Path, value: &Value) -> R<()> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    use std::io::Write;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn state(unit: &str) -> R<Value> {
    let output = Command::new("systemctl")
        .args([
            "--user",
            "show",
            unit,
            "--property=Id,LoadState,ActiveState,SubState,MainPID,UnitFileState,Triggers",
            "--no-pager",
        ])
        .output()?;
    if !output.status.success() {
        return Err("unit preflight unavailable".into());
    }
    Ok(json!(
        String::from_utf8(output.stdout)?
            .lines()
            .filter_map(|line| line
                .split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned())))
            .collect::<BTreeMap<_, _>>()
    ))
}
fn main() -> R<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let stage = Path::new(args.get(1).ok_or("STAGE required")?);
    let units = Path::new("/home/ace/.config/systemd/user");
    let id = uuid::Uuid::now_v7();
    let history = stage
        .join("unit-backups/historical")
        .join(format!("retired-{id}"));
    fs::create_dir(&history)?;
    fs::set_permissions(&history, fs::Permissions::from_mode(0o700))?;
    let mut records = Vec::new();
    // Complete every readiness check and durable backup before changing any unit.
    for name in UNITS {
        let before = state(name)?;
        if before["MainPID"] != "0"
            || !matches!(before["ActiveState"].as_str(), Some("inactive" | "failed"))
            || before["Triggers"] != ""
        {
            return Err(format!("refusing non-idle or triggered unit: {name}").into());
        }
        let source = units.join(name);
        if !source.symlink_metadata()?.file_type().is_file() {
            return Err("expected original regular unit file".into());
        }
        let seal = hash(&source)?;
        let backup = history.join(name);
        fs::copy(&source, &backup)?;
        if hash(&backup)? != seal || hash(&source)? != seal {
            return Err("unit changed during archive".into());
        }
        fs::File::open(&backup)?.sync_all()?;
        fs::set_permissions(&backup, fs::Permissions::from_mode(0o400))?;
        records.push(
            json!({"unit":name,"source":source,"backup":backup,"sha256":seal,"before":before}),
        );
    }
    fs::File::open(&history)?.sync_all()?;
    write(
        &history.join("retirement-intent.json"),
        &json!({"authorization":"root explicitly authorized archive/remove/mask these exact five obsolete inactive controllers","units":records,"active_native_hosts_excluded":true,"no_stop_or_start":true}),
    )?;
    for record in &records {
        let name = record["unit"].as_str().unwrap();
        let current = state(name)?;
        if current["MainPID"] != "0"
            || !matches!(current["ActiveState"].as_str(), Some("inactive" | "failed"))
            || current["Triggers"] != ""
            || hash(&units.join(name))? != record["sha256"].as_str().unwrap()
        {
            return Err("unit readiness changed; retirement refused".into());
        }
    }
    for name in UNITS {
        fs::remove_file(units.join(name))?;
    }
    fs::File::open(units)?.sync_all()?;
    let output = Command::new("systemctl")
        .args(["--user", "mask"])
        .args(UNITS)
        .output()?;
    let mut after = Vec::new();
    for name in UNITS {
        after.push(json!({"unit":name,"state":state(name)?,"mask_target":fs::read_link(units.join(name)).ok()}));
    }
    let complete = output.status.success()
        && after.iter().all(|entry| {
            entry["state"]["UnitFileState"] == "masked"
                && entry["state"]["MainPID"] == "0"
                && entry["mask_target"] == "/dev/null"
        });
    let receipt = stage.join(format!("obsolete-controller-retirement-{id}.json"));
    write(
        &receipt,
        &json!({"status":if complete{"archived-and-masked"}else{"retirement-incomplete-owner-review-required"},"units":records,"after":after,"systemctl_exit":output.status.code(),"systemctl_stderr":String::from_utf8(output.stderr)?,"no_stop_or_start":true,"active_native_hosts_modified":false,"source_history_unchanged":true}),
    )?;
    println!(
        "{}",
        json!({"receipt":receipt,"history":history,"complete":complete})
    );
    if !complete {
        return Err("retirement needs owner review".into());
    }
    Ok(())
}
