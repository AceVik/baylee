//! Read-only final identity/health and incomplete-record preservation receipt.
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
type R<T> = Result<T, Box<dyn std::error::Error>>;
fn command(program: &str, args: &[&str]) -> R<String> {
    let out = Command::new(program).args(args).output()?;
    if !out.status.success() {
        return Err(format!("read-only command failed: {program}").into());
    }
    Ok(String::from_utf8(out.stdout)?)
}
fn read(path: &Path) -> R<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn hash(path: &Path) -> R<String> {
    Ok(baylee_train_ops::files::sha256(path).map_err(|e| e.to_string())?)
}
fn state(name: &str) -> R<Value> {
    Ok(json!(
        command(
            "systemctl",
            &[
                "--user",
                "show",
                name,
                "--property=Id,ActiveState,SubState,MainPID,UnitFileState",
                "--no-pager"
            ]
        )?
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
        .collect::<BTreeMap<_, _>>()
    ))
}
fn identity(pid: u32) -> Value {
    baylee_train_ops::process::identity(pid).map_or(
        Value::Null,
        |v| json!({"pid":v.pid,"start_ticks":v.start_ticks,"boot_id":v.boot_id}),
    )
}
fn main() -> R<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let stage = Path::new(args.get(1).ok_or("STAGE required")?);
    let mut hosts = Vec::new();
    for (profile, name, port) in [
        ("preview", "baylee-v5-live.service", 28766),
        (
            "arena-lane2",
            "baylee-arena-v5-gateway-lane2.service",
            28777,
        ),
        ("arena", "baylee-arena-v5-gateway.service", 28776),
    ] {
        let status = state(name)?;
        let pid = status["MainPID"]
            .as_str()
            .ok_or("missing PID")?
            .parse::<u32>()?;
        let executable = fs::read_link(format!("/proc/{pid}/exe"))?;
        if status["ActiveState"] != "active" || executable != stage.join("bin/serve_local4") {
            return Err("native host identity differs".into());
        }
        let health: Value = serde_json::from_str(&command(
            "curl",
            &[
                "--silent",
                "--show-error",
                "--max-time",
                "3",
                &format!("http://127.0.0.1:{port}/health"),
            ],
        )?)?;
        if health["ok"] != true
            || health["database"] != true
            || health["games"]["running"] != 0
            || health["agents"]["connected"] != 1
        {
            return Err("native host health differs".into());
        }
        let installed = Path::new("/home/ace/.config/systemd/user").join(name);
        if hash(&installed)? != hash(&stage.join("units").join(name))? {
            return Err("installed unit differs from sealed unit".into());
        }
        let path = stage.join("manifests").join(format!("{profile}.json"));
        let check = command(
            stage.join("bin/serve_local4").to_str().unwrap(),
            &["--manifest", path.to_str().unwrap(), "--check"],
        )?;
        hosts.push(json!({"profile":profile,"unit":status,"identity":identity(pid),"executable":executable,"health":health,"profile_check":check.trim(),"installed_unit_sha256":hash(&installed)?,"manifest_sha256":hash(&path)?}));
    }
    let mut old = Vec::new();
    for pid in [9331, 95768, 95824, 95828, 95857, 278521] {
        let current = identity(pid);
        if !current.is_null() {
            return Err(format!("old known PID still alive: {pid}").into());
        }
        old.push(json!({"pid":pid,"live_identity":current}));
    }
    let query = "SELECT json_agg(row_to_json(g)) FROM (SELECT game_id,complete,ended_at,bytes,winners,end_reason FROM game_record WHERE game_id IN ('01a182df-c0e5-77cf-b429-9ffbb11c283c','01a18269-ca01-71ea-a842-5d222e0843a2') ORDER BY started_at DESC)g";
    let result = Command::new("psql")
        .env(
            "PGOPTIONS",
            "-c default_transaction_read_only=on -c statement_timeout=3000",
        )
        .args([
            "-X",
            "-q",
            "-d",
            "baylee_arena_v5_vs_argon_20261028",
            "-v",
            "ON_ERROR_STOP=1",
            "-Atc",
            query,
        ])
        .output()?;
    if !result.status.success() {
        return Err("read-only DB preservation query failed".into());
    }
    let records: Value = serde_json::from_slice(&result.stdout)?;
    if !records.as_array().is_some_and(|rows| {
        rows.len() == 2
            && rows.iter().all(|r| {
                r["complete"] == false
                    && r["ended_at"].is_null()
                    && r["winners"].is_null()
                    && r["end_reason"].is_null()
            })
    }) {
        return Err("incomplete records acquired an outcome; owner review required".into());
    }
    let mut masks = Vec::new();
    for name in [
        "baylee-arena-v5.service",
        "baylee-arena-v5-lane2.service",
        "baylee-continuous-v5-training.service",
        "baylee-v5-full-coverage-eval.service",
        "baylee-terra-flash-balanced.service",
    ] {
        let status = state(name)?;
        let link = fs::read_link(Path::new("/home/ace/.config/systemd/user").join(name))?;
        if status["MainPID"] != "0"
            || status["UnitFileState"] != "masked"
            || link != Path::new("/dev/null")
        {
            return Err("obsolete controller is not safely masked".into());
        }
        masks.push(status);
    }
    let receipt = json!({"status":"all-three-native-hosts-healthy-obsolete-controllers-masked","observed_unix":baylee_train_ops::files::unix_time().map_err(|e|e.to_string())?,"release_source":read(&stage.join("release-source-receipt.json"))?,"native_hosts":hosts,"old_known_pid_checks":old,"incomplete_records_preserved":records,"training_exclusion":"These two complete=false rows remain excluded by complete_ids exporter and Header-to-End admission; no outcome, winner or End synthesized.","obsolete_masked_units":masks,"services_modified_by_this_verifier":false,"no_new_training_or_collector_started":true,"no_token_or_process_environment_reads":true});
    let path = stage.join(format!(
        "final-host-migration-verification-{}.json",
        uuid::Uuid::now_v7()
    ));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    use std::io::Write;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!(
        "{}",
        json!({"receipt":path,"native_hosts":3,"all_healthy":true,"old_wrappers_masked":5,"incomplete_records_without_outcomes":2})
    );
    Ok(())
}
