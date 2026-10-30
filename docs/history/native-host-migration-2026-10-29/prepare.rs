//! Preparation only. Contains no service mutation, token lookup or process control.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
type R<T> = Result<T, Box<dyn std::error::Error>>;
const PROFILES: [(&str, &str); 3] = [
    ("preview", "baylee-v5-live.service"),
    ("arena", "baylee-arena-v5-gateway.service"),
    ("arena-lane2", "baylee-arena-v5-gateway-lane2.service"),
];
fn hash(path: &Path) -> R<String> {
    Ok(baylee_train_ops::files::sha256(path).map_err(|e| e.to_string())?)
}
fn read(path: &Path) -> R<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn write(path: &Path, value: &Value) -> R<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    use std::io::Write;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn status(unit: &str) -> R<Value> {
    let result = Command::new("systemctl")
        .args([
            "--user",
            "show",
            unit,
            "--property=Id,ActiveState,SubState,MainPID,UnitFileState,FragmentPath",
            "--no-pager",
        ])
        .output()?;
    if !result.status.success() {
        return Err(format!("unit status unavailable: {unit}").into());
    }
    let fields = String::from_utf8(result.stdout)?
        .lines()
        .filter_map(|line| {
            line.split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
        })
        .collect::<BTreeMap<_, _>>();
    Ok(json!(fields))
}
fn checked_copy(source: &Path, destination: &Path) -> R<String> {
    if destination.exists() || destination.is_symlink() {
        return Err("preserving existing staged output".into());
    }
    let seal = hash(source)?;
    fs::copy(source, destination)?;
    if hash(source)? != seal || hash(destination)? != seal {
        return Err("source changed during staging".into());
    }
    Ok(seal)
}
fn artifact_seals(root: &Path, path: &Path, seals: &mut BTreeMap<String, String>) -> R<()> {
    for item in fs::read_dir(path)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            artifact_seals(root, &item.path(), seals)?;
        } else if item.file_type()?.is_file() {
            seals.insert(
                item.path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .into_owned(),
                hash(&item.path())?,
            );
        }
    }
    Ok(())
}
fn verify_prepared(root: &Path, installed: bool) -> R<()> {
    let receipt = read(&root.join("prepared-artifacts.json"))?;
    for (path, expected) in receipt["artifacts_sha256"]
        .as_object()
        .ok_or("missing prepared artifact seals")?
    {
        if hash(&root.join(path))? != expected.as_str().ok_or("invalid prepared seal")? {
            return Err(format!("prepared artifact changed: {path}").into());
        }
    }
    if installed {
        for (path, expected) in receipt["original_units_sha256"]
            .as_object()
            .ok_or("missing original unit seals")?
        {
            if hash(Path::new(path))? != expected.as_str().ok_or("invalid unit seal")? {
                return Err(
                    "original unit changed since preparation; owner must review again".into(),
                );
            }
        }
    }
    Ok(())
}
fn inventory(units: &Path, backup: &Path) -> R<Value> {
    let mut entries = Vec::new();
    for item in fs::read_dir(units)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".service") || !item.file_type()?.is_file() {
            continue;
        }
        // Inspect only executable directives; never query service environments.
        let content = fs::read_to_string(item.path())?;
        let Some(start) = content
            .lines()
            .find_map(|line| line.strip_prefix("ExecStart="))
        else {
            continue;
        };
        if !(start.contains("python") || start.contains(".sh")) {
            continue;
        }
        let torch = start.contains("train4.py")
            || start.contains("train5.py")
            || start.contains("train_rl4.py")
            || start.contains("train_policy4.py");
        let state = status(&name)?;
        let inactive = matches!(state["ActiveState"].as_str(), Some("inactive" | "failed"));
        let seal = checked_copy(&item.path(), &backup.join(&name))?;
        fs::set_permissions(backup.join(&name), fs::Permissions::from_mode(0o400))?;
        let timer = name.replace(".service", ".timer");
        let timer_state = if units.join(&timer).exists() {
            checked_copy(&units.join(&timer), &backup.join(&timer))?;
            fs::set_permissions(backup.join(&timer), fs::Permissions::from_mode(0o400))?;
            status(&timer)?
        } else {
            Value::Null
        };
        entries.push(json!({"unit":name,"state":state,"sha256":seal,"paired_timer":timer_state,"responsibility":if torch{"pytorch_worker"}else{"python_or_shell_controller"},"recommendation":if torch{"retain tensor worker; audit pure wrapper separately"}else if inactive{"review then archive unit+dropins and disable/mask unit plus paired timer; do not leave executable historical controller"}else{"active controller requires explicit owner migration; do not interrupt"}}));
    }
    entries.sort_by(|a, b| a["unit"].as_str().cmp(&b["unit"].as_str()));
    Ok(json!(entries))
}
fn prepare(repo: &Path, inv: &Path, units: &Path) -> R<PathBuf> {
    let id = uuid::Uuid::now_v7();
    let root = inv.join(format!("native-host-deployment-20261029-{id}"));
    fs::create_dir(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    for directory in [
        "bin",
        "review",
        "templates",
        "manifests",
        "units",
        "unit-backups",
    ] {
        fs::create_dir(root.join(directory))?;
    }
    let review = repo.join("docs/history/native-host-migration-2026-10-29");
    checked_copy(&review.join("prepare.rs"), &root.join("prepare.rs"))?;
    checked_copy(&std::env::current_exe()?, &root.join("prepare-tool"))?;
    fs::set_permissions(root.join("prepare-tool"), fs::Permissions::from_mode(0o555))?;
    let mut profiles = Vec::new();
    for (name, unit) in PROFILES {
        let source = review.join(format!("{name}.json"));
        let manifest = read(&source)?;
        let source_seal = checked_copy(&source, &root.join("review").join(format!("{name}.json")))?;
        let mut staged = manifest.clone();
        let seals = staged["sha256"]
            .as_object_mut()
            .ok_or("missing source pins")?;
        let retired = seals
            .keys()
            .filter(|path| path.contains("/target/debug/"))
            .cloned()
            .collect::<Vec<_>>();
        for path in retired {
            seals.remove(&path);
        }
        for (path, expected) in seals.iter() {
            if hash(Path::new(path))? != expected.as_str().ok_or("invalid input seal")? {
                return Err(format!("reviewed host input changed: {path}").into());
            }
        }
        for stage in staged["stages"].as_array_mut().ok_or("missing stages")? {
            if stage["name"] == "collection" {
                stage["command"][0] = json!(root.join("bin/collect_local4"));
            }
        }
        write(
            &root.join("templates").join(format!("{name}.json")),
            &staged,
        )?;
        let unit_path = units.join(unit);
        let unit_bytes = fs::read_to_string(&unit_path)?;
        let unit_seal = checked_copy(&unit_path, &root.join("unit-backups").join(unit))?;
        fs::set_permissions(
            root.join("unit-backups").join(unit),
            fs::Permissions::from_mode(0o400),
        )?;
        let mut replacement = String::new();
        let mut starts = 0;
        for line in unit_bytes.lines() {
            if line.starts_with("ExecStart=") {
                starts += 1;
                replacement.push_str(&format!(
                    "ExecStart={} --manifest {}\n",
                    root.join("bin/serve_local4").display(),
                    root.join("manifests")
                        .join(format!("{name}.json"))
                        .display()
                ));
            } else if line.starts_with("Restart=") {
                replacement.push_str("Restart=no\n");
            } else {
                replacement.push_str(line);
                replacement.push('\n');
            }
        }
        if starts != 1 {
            return Err("one literal host ExecStart required".into());
        }
        fs::write(root.join("units").join(unit), replacement)?;
        profiles.push(json!({"profile":name,"unit":unit,"bind":manifest["bind"],"review_sha256":source_seal,"unit_before_sha256":unit_seal,"unit_before_state":status(unit)?,"deployment_gate":if name=="arena"{"HOLD_ACTIVE_GAME: no interruption; fresh game-drain review required"}else{"owner must freshly confirm idle before deployment"},"preserved":"gateway/agent/engine/seat/model paths and all reviewed stage arguments and environment","unit_changes":["ExecStart native immutable supervisor+manifest","Restart=no to avoid automatic controller retries"]}));
    }
    // Include other inactive Python units as review candidates, never disable them here.
    let historical_dir = root.join("unit-backups/historical");
    fs::create_dir(&historical_dir)?;
    write(
        &root.join("unit-inventory.json"),
        &inventory(units, &historical_dir)?,
    )?;
    write(
        &root.join("release-bin-review.pending.json"),
        &json!({"profile":"release","binaries":{"serve_local4":{"source":repo.join("target/release/serve_local4"),"sha256":null},"collect_local4":{"source":repo.join("target/release/collect_local4"),"sha256":null}},"status":"owner fills literal reviewed release hashes after final build; debug inputs refused"}),
    )?;
    write(
        &root.join("deployment.json"),
        &json!({"version":1,"id":id.to_string(),"status":"prepared-only-awaiting-reviewed-release-binaries","services_modified":false,"profiles":profiles,"helper_source_sha256":hash(&review.join("prepare.rs"))?,"archive_policy":"preserve historical bytes and receipts; owner-reviewed disable+mask of inactive pure controllers and paired timers before archive, never reactivate legacy Python","operator_sequence":["seal reviewed final release binaries into this UUID root","verify sealed manifests and unchanged original unit bytes","freshly verify idle preview and lane2; arena stays held for active game","owner separately installs staged unit files then daemon-reload; helper never does this","owner separately performs controlled idle-host migration; preserve original gateway/model","verify native identities/health and retain rollback backups"]}),
    )?;
    let mut artifacts = BTreeMap::new();
    artifact_seals(&root, &root, &mut artifacts)?;
    let originals = PROFILES
        .iter()
        .map(|(_, unit)| {
            Ok((
                units.join(unit).to_string_lossy().into_owned(),
                hash(&units.join(unit))?,
            ))
        })
        .collect::<R<BTreeMap<_, _>>>()?;
    write(
        &root.join("prepared-artifacts.json"),
        &json!({"artifacts_sha256":artifacts,"original_units_sha256":originals,"services_modified":false}),
    )?;
    fs::set_permissions(
        root.join("prepared-artifacts.json"),
        fs::Permissions::from_mode(0o444),
    )?;
    verify_prepared(&root, true)?;
    Ok(root)
}
fn seal(root: &Path, review: &Path) -> R<()> {
    verify_prepared(root, true)?;
    if root.join("immutable-binaries.json").exists() {
        return Err("deployment is already sealed".into());
    }
    let input = read(review)?;
    if input["profile"] != "release" {
        return Err("only explicitly reviewed release binaries permitted".into());
    }
    let mut binaries = BTreeMap::new();
    let mut manifests = BTreeMap::new();
    for name in ["serve_local4", "collect_local4"] {
        let item = &input["binaries"][name];
        let source = Path::new(
            item["source"]
                .as_str()
                .ok_or("missing reviewed binary path")?,
        );
        let expected = item["sha256"]
            .as_str()
            .filter(|s| s.len() == 64)
            .ok_or("literal reviewed release hash required")?;
        if source.to_string_lossy().contains("/debug/")
            || !source.is_absolute()
            || hash(source)? != expected
            || root.join("bin").join(name).exists()
        {
            return Err("release binary preflight mismatch; preserving staged package".into());
        }
    }
    for name in ["serve_local4", "collect_local4"] {
        let item = &input["binaries"][name];
        let source = Path::new(
            item["source"]
                .as_str()
                .ok_or("missing reviewed binary path")?,
        );
        let expected = item["sha256"]
            .as_str()
            .filter(|s| s.len() == 64)
            .ok_or("literal reviewed release hash required")?;
        if source.to_string_lossy().contains("/debug/")
            || !source.is_absolute()
            || hash(source)? != expected
        {
            return Err("release binary review mismatch".into());
        }
        let out = root.join("bin").join(name);
        let actual = checked_copy(source, &out)?;
        fs::set_permissions(&out, fs::Permissions::from_mode(0o555))?;
        binaries.insert(out.to_string_lossy().into_owned(), actual);
    }
    for (name, _) in PROFILES {
        let mut manifest = read(&root.join("templates").join(format!("{name}.json")))?;
        if name == "preview" {
            let collector = root
                .join("bin/collect_local4")
                .to_string_lossy()
                .into_owned();
            manifest["sha256"][&collector] = json!(binaries[&collector]);
        }
        let path = root.join("manifests").join(format!("{name}.json"));
        write(&path, &manifest)?;
        baylee_train_ops::supervisor::check(&path).map_err(|e| e.to_string())?;
        manifests.insert(path.to_string_lossy().into_owned(), hash(&path)?);
        fs::set_permissions(path, fs::Permissions::from_mode(0o444))?;
    }
    write(
        &root.join("immutable-binaries.json"),
        &json!({"profile":"release","review_sha256":hash(review)?,"sha256":binaries,"manifest_sha256":manifests,"services_modified":false}),
    )?;
    fs::set_permissions(
        root.join("immutable-binaries.json"),
        fs::Permissions::from_mode(0o444),
    )?;
    verify(root)
}
fn verify(root: &Path) -> R<()> {
    verify_prepared(root, false)?;
    let receipt = read(&root.join("immutable-binaries.json"))?;
    for (path, expected) in receipt["sha256"]
        .as_object()
        .ok_or("missing binary seals")?
    {
        if hash(Path::new(path))? != expected.as_str().ok_or("invalid seal")? {
            return Err("immutable binary changed".into());
        }
    }
    let manifests = receipt["manifest_sha256"]
        .as_object()
        .ok_or("missing runtime manifest seals")?;
    if manifests.len() != PROFILES.len() {
        return Err("incomplete runtime manifest seals".into());
    }
    for (path, expected) in manifests {
        if hash(Path::new(path))? != expected.as_str().ok_or("invalid runtime manifest seal")? {
            return Err("runtime manifest changed".into());
        }
    }
    for (name, _) in PROFILES {
        baylee_train_ops::supervisor::check(&root.join("manifests").join(format!("{name}.json")))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() -> R<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("prepare") if args.len() == 5 => {
            let root = prepare(
                Path::new(&args[2]),
                Path::new(&args[3]),
                Path::new(&args[4]),
            )?;
            println!(
                "{}",
                json!({"prepared":root,"services_modified":false,"release_binaries_pinned":false})
            );
        }
        Some("seal") if args.len() == 4 => {
            seal(Path::new(&args[2]), Path::new(&args[3]))?;
            println!("Release binaries sealed; no services changed");
        }
        Some("verify") if args.len() == 3 => {
            verify(Path::new(&args[2]))?;
            println!("Immutable host package verified; no services changed");
        }
        Some("verify-prepared") if args.len() == 3 => {
            verify_prepared(Path::new(&args[2]), true)?;
            println!("Prepared files and original units verified; no services changed");
        }
        _ => {
            return Err(
                "prepare REPO INV USER_UNIT_DIR | seal STAGE REVIEW_JSON | verify STAGE".into(),
            );
        }
    }
    Ok(())
}
