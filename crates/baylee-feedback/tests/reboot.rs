//! `scripts/server/baylee-reboot` with every command that would touch a
//! server stood in for (`date`, `stat`, `curl`, `systemctl`, `flock`,
//! `logger`): a pending reboot happens only inside the night window and
//! only while no game of this machine runs; after the drain delay the agent
//! is stopped instead, and started again when the window closes first.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// A stand-in command that appends its name and arguments to `calls` and
/// then runs `body`.
fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!("#!/usr/bin/env bash\necho \"{name} $*\" >> \"$REBOOT_CALLS\"\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

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
    /// Whether an earlier run already stopped the agent.
    drained: bool,
    /// Whether a deploy holds its lock.
    deploying: bool,
}

/// The stand-in clock's now, in Unix seconds.
const NOW: u64 = 1_800_000_000;

const QUIET_NIGHT: Server = Server {
    pending: true,
    hour: 4,
    days_pending: 0,
    health: Some((0, 0)),
    drained: false,
    deploying: false,
};

/// What one run did: the commands it ran and whether it left the drain mark.
struct Run {
    calls: String,
    drained_after: bool,
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
        "BAYLEE_UNIX_SOCKET=/run/baylee/gateway.sock\n",
    )
    .unwrap();
    if server.pending {
        std::fs::write(runtime.join("reboot-required"), "").unwrap();
        std::fs::write(runtime.join("reboot-required.pkgs"), "linux-image\n").unwrap();
    }
    if server.drained {
        std::fs::write(runtime.join("baylee-reboot.drained"), "").unwrap();
    }
    let lock = scratch.join("lock");
    std::fs::write(&lock, "").unwrap();
    let calls: PathBuf = scratch.join("calls");

    let since = NOW - server.days_pending * 86_400;
    stub(
        &stubs,
        "date",
        &format!(
            r#"[ "$1" = +%H ] && printf '%02d\n' {}; [ "$1" = +%s ] && echo {NOW}; exit 0"#,
            server.hour
        ),
    );
    stub(&stubs, "stat", &format!("echo {since}"));
    stub(
        &stubs,
        "curl",
        &match server.health {
            Some((running, waiting)) => format!(
                r#"echo '{{"games":{{"running":{},"local_running":{running},"waiting":{waiting}}}}}'"#,
                running + 5
            ),
            None => "exit 7".to_owned(),
        },
    );
    stub(&stubs, "systemctl", "exit 0");
    stub(&stubs, "logger", "exit 0");
    stub(
        &stubs,
        "flock",
        if server.deploying { "exit 1" } else { "exit 0" },
    );

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-reboot");
    let path = format!("{}:{}", stubs.display(), std::env::var("PATH").unwrap());
    let output = std::process::Command::new("bash")
        .arg(&script)
        .env("PATH", path)
        .env("REBOOT_CALLS", &calls)
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
fn after_the_drain_delay_the_agent_stops_and_no_game_ends() {
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
    assert!(due.ran("systemctl stop baylee-agent"));
    assert!(due.drained_after);
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
fn a_window_that_closes_while_drained_starts_the_agent_again() {
    let day = run(Server {
        drained: true,
        hour: 8,
        health: Some((1, 0)),
        ..QUIET_NIGHT
    });
    assert!(day.ran("systemctl start baylee-agent"));
    assert!(!day.drained_after);
    assert!(!day.rebooted());

    // Also when the reboot stopped being pending (someone rebooted by hand).
    let gone = run(Server {
        drained: true,
        pending: false,
        ..QUIET_NIGHT
    });
    assert!(gone.ran("systemctl start baylee-agent"));
    assert!(!gone.drained_after);
}
