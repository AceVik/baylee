use super::*;
struct FakeSystem {
    root: std::path::PathBuf,
    current: Unit,
    starts: u32,
    fail: bool,
}
impl System for FakeSystem {
    fn unit(&mut self, _: &str) -> Result<Unit> {
        Ok(self.current.clone())
    }
    fn start(&mut self, _: &str) -> Result<()> {
        self.starts += 1;
        ensure!(!self.fail, "simulated start failure");
        self.current.pid = 200_000;
        self.current.active = "active".into();
        files::atomic_json(
            &self.root.join("controller.json"),
            &json!({"pid":200_000,"status":"running"}),
        )
    }
    fn identity(&self, pid: u32) -> Option<process::Identity> {
        (pid == 200_000).then(|| process::Identity {
            pid,
            start_ticks: 2,
            boot_id: "test".into(),
        })
    }
}
fn setup(ops: &Path) -> Result<FakeSystem> {
    let root = ops.join("primary");
    fs::create_dir_all(root.join("monitor"))?;
    fs::write(root.join("DRAIN"), "")?;
    files::atomic_json(&root.join("status.json"), &json!({"phase":"stopped"}))?;
    let old = json!({"pid":190_701,"start_ticks":1,"boot_id":"test"});
    files::atomic_json(
        &root.join("monitor/config.json"),
        &json!({"controller":old,"preserved":true}),
    )?;
    files::atomic_json(
        &root.join("monitor/state.json"),
        &json!({"delivered":{"historical":true}}),
    )?;
    let binary = ops.join("collector");
    let profiles = root.join("profiles-flash.json");
    let pairings = root.join("pairings-flash.json");
    for path in [&binary, &profiles, &pairings] {
        fs::write(path, "immutable input")?;
    }
    let argv = vec![
        binary.display().to_string(),
        "collect".into(),
        "--root".into(),
        root.display().to_string(),
        "--profiles-config".into(),
        profiles.display().to_string(),
        "--pairings-file".into(),
        pairings.display().to_string(),
        "--concurrency".into(),
        "34".into(),
    ];
    let mut config = json!({"unit":PRIMARY,"root":root,"old_controller":old,"not_before_unix":0,"binary":binary,"expected_argv":argv,"sha256":{binary.to_str().context("path")?:files::sha256(&binary)?,profiles.to_str().context("path")?:files::sha256(&profiles)?,pairings.to_str().context("path")?:files::sha256(&pairings)?}});
    let probe = super::super::probe::tests::fixture(&ops.join("probe"))?;
    config
        .as_object_mut()
        .context("config")?
        .extend(probe.as_object().context("probe config")?.clone());
    files::atomic_json(&ops.join("flash-transition.json"), &config)?;
    files::atomic_json(
        &ops.join("collector-activation-plan.json"),
        &json!({"schema_version":1,"unit":PRIMARY,"not_before_unix":0,"config_sha256":files::sha256(&ops.join("flash-transition.json"))?}),
    )?;
    Ok(FakeSystem {
        root,
        current: Unit {
            active: "inactive".into(),
            pid: 0,
            argv,
        },
        starts: 0,
        fail: false,
    })
}
#[test]
fn one_start_preserves_delivery_and_is_idempotent() -> Result<()> {
    let ops = tempfile::tempdir()?;
    let mut system = setup(ops.path())?;
    ensure!(handover_with(ops.path(), false, &mut system)? == "done");
    ensure!(system.starts == 1);
    ensure!(handover_with(ops.path(), false, &mut system)? == "done");
    ensure!(system.starts == 1);
    ensure!(!system.root.join("DRAIN").exists());
    ensure!(
        json(system.root.join("monitor/state.json"))? == json!({"delivered":{"historical":true}})
    );
    ensure!(json(system.root.join("monitor/config.json"))?["preserved"] == true);
    Ok(())
}
#[test]
fn crash_after_start_recovers_without_duplicate_start() -> Result<()> {
    let ops = tempfile::tempdir()?;
    let mut system = setup(ops.path())?;
    files::atomic_json(
        &ops.path().join("flash-transition-state.json"),
        &json!({"phase":"starting","native_config_sha256":files::sha256(&ops.path().join("flash-transition.json"))?}),
    )?;
    fs::rename(
        system.root.join("DRAIN"),
        ops.path().join("DRAIN.before-flash"),
    )?;
    system.start(PRIMARY)?;
    system.starts = 0;
    ensure!(handover_with(ops.path(), false, &mut system)? == "done");
    ensure!(system.starts == 0);
    Ok(())
}
#[test]
fn failed_start_restores_drain_and_never_retries() -> Result<()> {
    let ops = tempfile::tempdir()?;
    let mut system = setup(ops.path())?;
    system.fail = true;
    ensure!(handover_with(ops.path(), false, &mut system).is_err());
    ensure!(system.starts == 1 && system.root.join("DRAIN").is_file());
    ensure!(handover_with(ops.path(), false, &mut system)? == "failed");
    ensure!(system.starts == 1);
    Ok(())
}
