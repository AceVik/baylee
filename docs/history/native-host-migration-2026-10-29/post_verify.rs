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
fn unit(name: &str) -> R<Value> {
    let text = command(
        "systemctl",
        &[
            "--user",
            "show",
            name,
            "--property=Id,ActiveState,SubState,MainPID,UnitFileState,Triggers",
            "--no-pager",
        ],
    )?;
    Ok(json!(
        text.lines()
            .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
            .collect::<BTreeMap<_, _>>()
    ))
}
fn identity(pid: u32) -> Value {
    baylee_train_ops::process::identity(pid).map_or(
        Value::Null,
        |value| json!({"pid":value.pid,"start_ticks":value.start_ticks,"boot_id":value.boot_id}),
    )
}
fn main() -> R<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let stage = Path::new(args.get(1).ok_or("STAGE required")?);
    let mut hosts = Vec::new();
    for (name, service, port) in [
        ("preview", "baylee-v5-live.service", "28766"),
        (
            "arena-lane2",
            "baylee-arena-v5-gateway-lane2.service",
            "28777",
        ),
    ] {
        let status = unit(service)?;
        let pid = status["MainPID"]
            .as_str()
            .ok_or("missing main pid")?
            .parse::<u32>()?;
        let executable = fs::read_link(format!("/proc/{pid}/exe"))?;
        if executable != stage.join("bin/serve_local4") {
            return Err("migrated unit executable mismatch".into());
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
        if health["ok"] != true || health["database"] != true || health["games"]["running"] != 0 {
            return Err("migrated idle host health changed; review required".into());
        }
        let installed = Path::new("/home/ace/.config/systemd/user").join(service);
        let staged = stage.join("units").join(service);
        if hash(&installed)? != hash(&staged)? {
            return Err("installed native unit differs from sealed plan".into());
        }
        let check = command(
            stage.join("bin/serve_local4").to_str().unwrap(),
            &[
                "--manifest",
                stage
                    .join("manifests")
                    .join(format!("{name}.json"))
                    .to_str()
                    .unwrap(),
                "--check",
            ],
        )?;
        let manifest = read(&stage.join("manifests").join(format!("{name}.json")))?;
        let model_pins = manifest["sha256"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.contains("/model-control-final1500/"))
            .map(|(path, expected)| {
                Ok((
                    path.clone(),
                    json!({"expected":expected,"actual":hash(Path::new(path))?}),
                ))
            })
            .collect::<R<BTreeMap<_, _>>>()?;
        hosts.push(json!({"profile":name,"unit":status,"controller_identity":identity(pid),"executable":executable,"health":health,"installed_unit_sha256":hash(&installed)?,"profile_check":check.trim(),"model_pins":model_pins}));
    }
    let sql = "SELECT json_build_object('game',(SELECT row_to_json(g) FROM (SELECT game_id,started_at,ended_at,complete,bytes FROM game_record WHERE game_id='01a182df-c0e5-77cf-b429-9ffbb11c283c')g),'seats',(SELECT json_agg(row_to_json(s)) FROM (SELECT seat,account_id IS NOT NULL AS has_account,delegated_by IS NOT NULL AS delegated FROM game_record_seat WHERE game_id='01a182df-c0e5-77cf-b429-9ffbb11c283c' ORDER BY seat)s),'chunks',(SELECT json_build_object('count',count(*),'max_seq',max(seq),'bytes',sum(octet_length(data))) FROM game_record_chunk WHERE game_id='01a182df-c0e5-77cf-b429-9ffbb11c283c'),'excluded_incomplete_game_ids',(SELECT json_agg(game_id) FROM (SELECT game_id FROM game_record WHERE complete=false ORDER BY started_at DESC LIMIT 5)x))";
    let out = Command::new("psql")
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
            sql,
        ])
        .output()?;
    if !out.status.success() {
        return Err("read-only arena DB metadata query failed".into());
    }
    let database: Value = serde_json::from_slice(&out.stdout)?;
    let arena_root = Path::new("/home/ace/baylee-data/v5/arena-v5-vs-argon-20261028");
    let log = arena_root.join("gateway/baylee-agent.log");
    let owner_confirmed = fs::read_to_string(&log)?.lines().any(|line| {
        line.contains("engine started")
            && line.contains("01a182df-c0e5-77cf-b429-9ffbb11c283c")
            && line.contains("278521")
    });
    if !owner_confirmed {
        return Err("arena engine ownership log no longer matches".into());
    }
    let processes = command("ps", &["-eo", "pid,ppid,comm"])?;
    let seats = processes
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .last()
                .is_some_and(|name| matches!(name, "seat4" | "llm_collector4"))
        })
        .collect::<Vec<_>>();
    let identities=[95824,95828,95857,278521].into_iter().map(|pid|json!({"identity":identity(pid),"executable":fs::read_link(format!("/proc/{pid}/exe")).ok()})).collect::<Vec<_>>();
    let mut obsolete = Vec::new();
    for name in [
        "baylee-arena-v5.service",
        "baylee-arena-v5-lane2.service",
        "baylee-continuous-v5-training.service",
        "baylee-v5-full-coverage-eval.service",
        "baylee-terra-flash-balanced.service",
    ] {
        obsolete.push(unit(name)?);
    }
    let arena = json!({"unit":unit("baylee-arena-v5-gateway.service")?,"owned_identities":identities,"game_id":"01a182df-c0e5-77cf-b429-9ffbb11c283c","engine_log_owner_confirmed":owner_confirmed,"engine_log_evidence":"2026-10-28T14:17:49.430447Z engine started game_id=01a182df-c0e5-77cf-b429-9ffbb11c283c pid=278521","database_metadata":database,"health":serde_json::from_str::<Value>(&command("curl",&["--silent","--show-error","--max-time","3","http://127.0.0.1:28776/health"])?)?,"tcp_peers":command("ss",&["-tnp","( sport = :28776 or dport = :28776 )"])? ,"global_seat_or_collector_processes":seats,"collector_status":read(&arena_root.join("status.json"))?,"collector_stop_exists":arena_root.join("STOP").exists(),"previous_attempt_receipt":read(&arena_root.join("attempts/attempt-000900031/receipt.json"))?["game_id"],"unfinished_attempt_directory_exists":arena_root.join("attempts/attempt-000900032").exists(),"classification":"abandoned owned collector session: STOP+idle error controller, no collector/arena-seat clients; only owned agent/engine TCP peers","cleanup_scope":"dedicated arena28776 unit only; root authorized cleanup; this verifier did not stop/abort anything","training_eligibility":"EXCLUDED_INCOMPLETE: all complete=false IDs listed above; exporter selects complete_ids and importer requires Header-to-End. Do not fabricate outcome or export/admit these incomplete records."});
    let receipt = json!({"operation":"post-native-preview-lane2-verification-and-orphan-arena-ownership","observed_unix":baylee_train_ops::files::unix_time().map_err(|e|e.to_string())?,"native_hosts":hosts,"arena":arena,"obsolete_unit_readiness":obsolete,"services_modified_by_this_verifier":false,"no_token_or_process_environment_reads":true});
    let path = stage.join(format!(
        "post-migration-observation-{}.json",
        uuid::Uuid::now_v7()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    use std::io::Write;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!(
        "{}",
        json!({"receipt":path,"arena_game":"01a182df-c0e5-77cf-b429-9ffbb11c283c","services_modified":false})
    );
    Ok(())
}
