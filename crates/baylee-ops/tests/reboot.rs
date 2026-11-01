//! `baylee-reboot` with every command that would touch a server stood in for
//! (`date` for the hour, `curl`, `systemctl`, `logger`), the pending file's
//! age set on the file itself and a running deploy holding the real lock: a
//! pending reboot happens only inside the night window and
//! only while admission is held and no game of this machine runs. The owner
//! stays alive during draining; closing the window releases only our hold.

#![cfg(unix)]

use std::path::PathBuf;

#[path = "support/native_command.rs"]
mod native_command;

/// One server state the script is run against.
#[derive(Clone, Copy, Debug)]
struct Server {
    /// Whether an update asked for a reboot.
    pending: bool,
    /// The local hour now.
    hour: u32,
    /// Days since the reboot was first asked for.
    days_pending: u64,
    /// `/health`'s `(local_running, waiting)`, or `None` when it does not answer.
    health: Option<(u32, u32)>,
    /// Whether an earlier run already held admission.
    drained: bool,
    /// Whether a deploy holds its lock.
    deploying: bool,
    /// What the gateway reports after the hold is created.
    admission: &'static str,
    health_after_hold: Option<(u32, u32)>,
    foreign_hold: Option<&'static str>,
}

const QUIET_NIGHT: Server = Server {
    pending: true,
    hour: 4,
    days_pending: 0,
    health: Some((0, 0)),
    drained: false,
    deploying: false,
    admission: "held",
    health_after_hold: None,
    foreign_hold: None,
};

/// What one run did: the commands it ran and whether it left the drain mark.
struct Run {
    calls: String,
    drained_after: bool,
    hold_after: Option<String>,
}

impl Run {
    fn rebooted(&self) -> bool {
        self.calls.lines().any(|l| l == "systemctl reboot")
    }
    fn ran(&self, line: &str) -> bool {
        self.calls.lines().any(|l| l == line)
    }
}

fn run(server: Server) -> Run {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "baylee-reboot-test-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (runtime, etc, stubs) = (
        scratch.join("run"),
        scratch.join("etc"),
        scratch.join("stubs"),
    );
    for dir in [&runtime, &etc, &stubs] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(
        etc.join("gateway.env"),
        format!(
            "BAYLEE_UNIX_SOCKET=/run/baylee/gateway.sock\nBAYLEE_ADMISSION_HOLD={}\n",
            runtime.join("admission-hold").display()
        ),
    )
    .unwrap();
    if server.pending {
        let required = runtime.join("reboot-required");
        std::fs::write(&required, "").unwrap();
        // Asked for that many days ago, a few minutes more so a slow run
        // cannot land on the boundary.
        let since = std::time::SystemTime::now()
            - std::time::Duration::from_secs(server.days_pending * 86_400 + 300);
        std::fs::File::options()
            .write(true)
            .open(&required)
            .unwrap()
            .set_modified(since)
            .unwrap();
        std::fs::write(runtime.join("reboot-required.pkgs"), "linux-image\n").unwrap();
    }
    if server.drained {
        std::fs::write(runtime.join("baylee-reboot.drained"), "").unwrap();
        std::fs::write(
            runtime.join("admission-hold"),
            "baylee-reboot admission hold\n",
        )
        .unwrap();
    }
    if let Some(contents) = server.foreign_hold {
        std::fs::write(runtime.join("admission-hold"), contents).unwrap();
    }
    let lock = scratch.join("lock");
    std::fs::write(&lock, "").unwrap();
    let calls: PathBuf = scratch.join("calls");

    native_command::install(&stubs, &["date", "curl", "systemctl", "logger"]);
    let state = scratch.join("native-command-state.json");
    std::fs::write(
        &state,
        serde_json::to_vec(&serde_json::json!({
            "hour": server.hour,
            "health": server.health,
            "admission": server.admission,
            "health_after_hold": server.health_after_hold,
        }))
        .unwrap(),
    )
    .unwrap();
    // A deploy holds its lock for its whole run: this test process holds it
    // while the reboot command looks.
    let held = std::fs::File::open(&lock).unwrap();
    if server.deploying {
        held.try_lock().unwrap();
    }

    let path = format!("{}:{}", stubs.display(), std::env::var("PATH").unwrap());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-reboot"))
        .env("PATH", path)
        .env("OPS_FAKE_CALLS", &calls)
        .env("OPS_FAKE_STATE", &state)
        .env_remove("OPS_FAKE_HEALTH")
        .env("BAYLEE_REBOOT_RUN", &runtime)
        .env("BAYLEE_REBOOT_ETC", &etc)
        .env("BAYLEE_REBOOT_DEPLOY_LOCK", &lock)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Run {
        calls: std::fs::read_to_string(&calls).unwrap_or_default(),
        drained_after: runtime.join("baylee-reboot.drained").exists(),
        hold_after: std::fs::read_to_string(runtime.join("admission-hold")).ok(),
    };
    let _ = std::fs::remove_dir_all(&scratch);
    run
}

#[test]
fn a_quiet_night_with_a_pending_reboot_reboots() {
    assert!(run(QUIET_NIGHT).rebooted());
}

#[test]
fn nothing_pending_nothing_happens() {
    let run = run(Server {
        pending: false,
        ..QUIET_NIGHT
    });
    assert!(!run.rebooted());
    assert!(
        !run.calls.lines().any(|l| l.starts_with("systemctl")),
        "{}",
        run.calls
    );
}

#[test]
fn a_running_game_here_holds_the_reboot() {
    let run = run(Server {
        health: Some((1, 0)),
        ..QUIET_NIGHT
    });
    assert!(!run.rebooted());
    assert!(
        !run.ran("systemctl stop baylee-agent"),
        "no drain before the delay"
    );
}

#[test]
fn only_this_machines_games_count() {
    // `running` counts 5 games of agents elsewhere; `local_running` is 0.
    assert!(run(QUIET_NIGHT).rebooted());
}

#[test]
fn a_waiting_room_holds_the_reboot_until_drained() {
    assert!(
        !run(Server {
            health: Some((0, 1)),
            ..QUIET_NIGHT
        })
        .rebooted()
    );
    assert!(
        run(Server {
            health: Some((0, 1)),
            drained: true,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
}

#[test]
fn a_gateway_that_does_not_answer_is_not_taken_for_an_empty_one() {
    let run = run(Server {
        health: None,
        days_pending: 10,
        ..QUIET_NIGHT
    });
    assert!(!run.rebooted());
    assert!(!run.ran("systemctl stop baylee-agent"));
}

#[test]
fn the_window_bounds_are_inclusive_and_the_day_is_outside() {
    assert!(
        !run(Server {
            hour: 2,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
    assert!(
        run(Server {
            hour: 3,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
    assert!(
        run(Server {
            hour: 7,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
    assert!(
        !run(Server {
            hour: 8,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
    assert!(
        !run(Server {
            hour: 15,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
}

#[test]
fn a_running_deploy_is_waited_for() {
    assert!(
        !run(Server {
            deploying: true,
            ..QUIET_NIGHT
        })
        .rebooted()
    );
}

#[test]
fn after_the_drain_delay_admission_closes_and_owner_stays_alive() {
    let early = run(Server {
        health: Some((2, 0)),
        days_pending: 2,
        ..QUIET_NIGHT
    });
    assert!(!early.ran("systemctl stop baylee-agent"));
    assert!(!early.drained_after);

    let due = run(Server {
        health: Some((2, 0)),
        days_pending: 3,
        ..QUIET_NIGHT
    });
    assert!(!due.ran("systemctl stop baylee-agent"));
    assert!(due.drained_after);
    assert!(due.hold_after.is_some());
    assert!(!due.rebooted());
}

#[test]
fn a_drained_night_reboots_once_the_last_game_ends() {
    let run = run(Server {
        drained: true,
        days_pending: 4,
        ..QUIET_NIGHT
    });
    assert!(run.rebooted());
    assert!(!run.drained_after);
}

#[test]
fn a_window_that_closes_releases_admission_without_restarting_owner() {
    let day = run(Server {
        drained: true,
        hour: 8,
        health: Some((1, 0)),
        ..QUIET_NIGHT
    });
    assert!(!day.ran("systemctl start baylee-agent"));
    assert!(!day.drained_after);
    assert!(day.hold_after.is_none());
    assert!(!day.rebooted());

    // Also when the reboot stopped being pending (someone rebooted by hand).
    let gone = run(Server {
        drained: true,
        pending: false,
        ..QUIET_NIGHT
    });
    assert!(!gone.ran("systemctl start baylee-agent"));
    assert!(!gone.drained_after);
}

#[test]
fn unverified_admission_never_reboots_or_stops_owner_even_at_zero_games() {
    for admission in ["open", "missing", "down"] {
        let result = run(Server {
            admission,
            ..QUIET_NIGHT
        });
        assert!(!result.rebooted(), "{admission}");
        assert!(!result.ran("systemctl stop baylee-agent"));
    }
}

#[test]
fn a_game_started_before_hold_is_seen_in_the_fresh_count() {
    let result = run(Server {
        health_after_hold: Some((1, 0)),
        ..QUIET_NIGHT
    });
    assert!(!result.rebooted());
    assert!(!result.ran("systemctl stop baylee-agent"));
    assert!(
        !result.drained_after,
        "before the drain delay, keep allowing games"
    );
    assert!(result.hold_after.is_none());
}

#[test]
fn foreign_holds_block_reboot_and_survive_window_cleanup() {
    for hour in [4, 8] {
        let result = run(Server {
            hour,
            foreign_hold: Some("deploy owns this\n"),
            drained: true,
            ..QUIET_NIGHT
        });
        assert!(!result.rebooted());
        assert_eq!(result.hold_after.as_deref(), Some("deploy owns this\n"));
        assert!(!result.ran("systemctl start baylee-agent"));
    }
}
