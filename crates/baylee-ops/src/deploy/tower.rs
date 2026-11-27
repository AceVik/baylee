//! The Tower Echoes authority (`docs/tower-operations.md` §"Production on one
//! server") on a server whose `baylee-tower` unit is installed: built with
//! the release and switched right after the gateway it serves, inside the
//! same window, while no game starts.
//!
//! The new binaries go in beside the running ones (`.next`) and the new
//! service checks the configuration it will read. Then the service stops,
//! the schema owner's documents migrate the database and renew the runtime
//! grants, the new binaries take the names (the previous ones kept as
//! `.prev`), and the service starts and must answer `/health` with `ready`.
//! The documents are root's (`migrate.json`, `grants.json` and
//! `service.json` in `/etc/baylee-tower/`, 0600), read by `systemd-run` as
//! standard input and never by this program, as Central's hook reads its
//! own. A failure before the stop leaves the running service alone; one
//! after it starts the previous build again, which a migrated schema may
//! refuse, and then it says so. A Tower that did not switch never stops the
//! deploy: the gateway and the games go on. The Tower engine the cohort is
//! bound to is not touched here (`BAYLEE_DEPLOY_TOWER_ENGINE`, `keep.rs`).
use super::{Deploy, quiet, say, short};

/// The unit; without it this server has no Tower.
const UNIT: &str = "baylee-tower";
/// What a release builds for it.
pub(super) const BINARIES: [&str; 2] = ["baylee-tower-service", "baylee-tower-provision"];
/// The configuration the running service reads.
const SERVICE: &str = "service.json";
/// The schema owner's configuration, for `--migrate`.
const MIGRATE: &str = "migrate.json";
/// The schema owner's `--admin` input that renews the runtime grants.
const GRANTS: &str = "grants.json";

impl Deploy {
    /// Whether this server runs the Tower: its build is then part of the
    /// release's.
    pub(super) fn tower_here() -> bool {
        Self::installed(UNIT)
    }

    /// Switches the Tower to this release's build once the gateway answers as
    /// it. Never stops the deploy: a Tower that stays behind says why.
    pub(super) fn switch_tower(&self, rev: &str) {
        if !Self::tower_here() {
            return;
        }
        match self.tower_switch() {
            Ok(()) => say(&format!("Tower service {} installed", short(rev))),
            Err(why) => say(&format!(
                "the Tower service was not switched to {}: {why}",
                short(rev)
            )),
        }
    }

    fn tower_switch(&self) -> Result<(), String> {
        if BINARIES.iter().any(|name| !self.p.staged(name).is_file()) {
            return Err("this release's Tower binaries were not built".into());
        }
        for document in [SERVICE, MIGRATE, GRANTS] {
            if !quiet(&mut self.sudo(&["test", "-f", &self.tower_document(document)])) {
                return Err(format!(
                    "{} is missing (docs/tower-operations.md); the running build stays",
                    self.tower_document(document)
                ));
            }
        }
        for name in BINARIES {
            if !quiet(&mut self.sudo(&[
                "install",
                "-m755",
                &self.p.staged(name).display().to_string(),
                &self.tower_binary(name, ".next"),
            ])) {
                return Err("the new binaries could not be put beside the running ones".into());
            }
        }
        let next = self.tower_binary(BINARIES[0], ".next");
        if !self.tower_run(SERVICE, &next, "--check") {
            return Err(
                "the new build refuses the service's configuration; the running one stays".into(),
            );
        }
        if !quiet(&mut self.sudo(&["systemctl", "stop", UNIT])) {
            return Err("the running service did not stop; it stays".into());
        }
        if !self.tower_run(MIGRATE, &next, "--migrate") || !self.tower_run(GRANTS, &next, "--admin")
        {
            return Err(self.tower_back("the migration or the runtime grants failed", false));
        }
        for name in BINARIES {
            let (current, previous) = (
                self.tower_binary(name, ""),
                self.tower_binary(name, ".prev"),
            );
            if !quiet(&mut self.sudo(&["cp", "-p", &current, &previous]))
                || !quiet(&mut self.sudo(&[
                    "mv",
                    "-f",
                    &self.tower_binary(name, ".next"),
                    &current,
                ]))
            {
                return Err(self.tower_back("the new binaries could not take their names", true));
            }
        }
        if !quiet(&mut self.sudo(&["systemctl", "start", UNIT])) || !self.tower_ready() {
            return Err(self.tower_back("the new build did not answer ready", true));
        }
        Ok(())
    }

    /// The previous build again, after `why`: from the kept `.prev` files
    /// once the new ones took the names.
    fn tower_back(&self, why: &str, swapped: bool) -> String {
        if swapped {
            for name in BINARIES {
                let _ = quiet(&mut self.sudo(&[
                    "mv",
                    "-f",
                    &self.tower_binary(name, ".prev"),
                    &self.tower_binary(name, ""),
                ]));
            }
        }
        let _ = quiet(&mut self.sudo(&["systemctl", "restart", UNIT]));
        if self.tower_ready() {
            format!("{why}; the previous build runs again")
        } else {
            format!(
                "{why}; the previous build does not answer either (a migrated schema refuses it): the Tower is down until it is put right by hand (docs/tower-operations.md)"
            )
        }
    }

    /// `binary` in `mode` with one of the root-only documents as its
    /// standard input, as the service's own user.
    fn tower_run(&self, document: &str, binary: &str, mode: &str) -> bool {
        quiet(&mut self.sudo(&[
            "systemd-run",
            "--wait",
            "--collect",
            "--quiet",
            "-p",
            "User=baylee-tower",
            "-p",
            "Group=baylee-tower",
            "-p",
            &format!("StandardInput=file:{}", self.tower_document(document)),
            binary,
            mode,
        ]))
    }

    /// Whether the service answers `/health` with `ready`, asked about once
    /// a second for as long as a new gateway's `/info` is.
    fn tower_ready(&self) -> bool {
        (0..self.p.info_tries).any(|_| {
            let ready = crate::sh::out_quiet(std::process::Command::new("curl").args([
                "-fsS",
                "-m",
                "2",
                &self.p.tower_health,
            ]))
            .is_some_and(|body| body.trim() == "ready");
            if !ready {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            ready
        })
    }

    fn tower_binary(&self, name: &str, suffix: &str) -> String {
        self.p
            .tower_bin
            .join(format!("{name}{suffix}"))
            .display()
            .to_string()
    }

    fn tower_document(&self, name: &str) -> String {
        self.p.tower_etc.join(name).display().to_string()
    }
}
