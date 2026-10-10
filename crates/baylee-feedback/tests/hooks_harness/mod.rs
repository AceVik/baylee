//! A server for `scripts/server/baylee-deploy` to deploy to, kept between
//! runs: every command that would change a real one is a stand-in that
//! notes its arguments, and the files under `ctl/` say what the machine
//! answers (the dispatcher's status per phase, the games `/health` counts,
//! what `/info` names, which release tags exist). The deploy hooks
//! themselves are the dispatcher's business (`crates/baylee-deploy-hooks`);
//! here a stand-in for it notes the phase, the commit, and whether the
//! admission hold was up and a stage was there when it was called.

#![allow(dead_code)] // each test binary uses its own part

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// The release a test deploys, and the one after it.
pub const A: &str = "aaaaaaaaaa0123456789aaaaaaaaaa0123456789";
pub const B: &str = "bbbbbbbbbb0123456789bbbbbbbbbb0123456789";

/// A stand-in command that appends its name and arguments to `calls` and
/// then runs `body`.
fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!("#!/usr/bin/env bash\necho \"{name} $*\" >> \"$DEPLOY_CALLS\"\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Reads the next answer from a `ctl` file: one per line, the last one
/// repeated once the others are used up.
const NEXT: &str = r#"next() {
    local file=$CTL/$1 first
    [ -f "$file" ] || { echo "$2"; return; }
    first=$(head -n1 "$file")
    if [ "$(wc -l < "$file")" -gt 1 ]; then
        tail -n +2 "$file" > "$file.rest" && mv "$file.rest" "$file"
    fi
    echo "$first"
}"#;

/// What one run of the deployer did.
pub struct Ran {
    pub ok: bool,
    pub code: Option<i32>,
    /// What it printed (its log lines).
    pub said: String,
    /// Every stand-in command it ran, one per line, in order.
    pub calls: String,
}

impl Ran {
    pub fn lines(&self) -> Vec<&str> {
        self.calls.lines().collect()
    }

    /// Where `wanted` is among the calls, or a panic that shows them.
    pub fn at(&self, wanted: &str) -> usize {
        self.calls
            .lines()
            .position(|l| l == wanted)
            .unwrap_or_else(|| panic!("no `{wanted}` in:\n{}\nsaid:\n{}", self.calls, self.said))
    }

    /// Where the first call starting with `prefix` is.
    pub fn first(&self, prefix: &str) -> Option<usize> {
        self.calls.lines().position(|l| l.starts_with(prefix))
    }

    pub fn has(&self, wanted: &str) -> bool {
        self.calls.lines().any(|l| l == wanted)
    }

    /// The hook phases the dispatcher was called for, in order.
    pub fn phases(&self) -> Vec<String> {
        self.calls
            .lines()
            .filter_map(|l| l.strip_prefix("hooks "))
            .map(|l| l.split(' ').next().unwrap_or_default().to_owned())
            .collect()
    }
}

/// The scratch server.
pub struct Server {
    pub scratch: PathBuf,
    pub root: PathBuf,
    pub etc: PathBuf,
    pub ctl: PathBuf,
    pub stubs: PathBuf,
    pub hold: PathBuf,
    calls: PathBuf,
}

impl Server {
    /// A server with deploy hooks set up (`10-example`), the admission hold
    /// configured, no game running, and `git` building [`A`].
    pub fn with_hooks() -> Self {
        let server = Self::bare();
        server.add_hook("10-example");
        server
    }

    /// A server with no hook directory at all.
    pub fn bare() -> Self {
        static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let scratch = std::env::temp_dir().join(format!(
            "baylee-deploy-hooks-test-{}-{}",
            std::process::id(),
            RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let root = scratch.join("opt");
        for dir in [
            "src/target/release",
            "src/web/feedback",
            "src/crates/baylee-client",
            "src/scripts/server",
            "bin",
            "state",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for binary in ["baylee-gateway", "baylee-catalog", "run-deploy-hooks"] {
            std::fs::write(root.join("src/target/release").join(binary), b"").unwrap();
        }
        std::fs::write(
            root.join("src/scripts/server/baylee-deploy-hooks.sudoers"),
            b"",
        )
        .unwrap();
        let etc = scratch.join("etc");
        std::fs::create_dir_all(&etc).unwrap();
        std::fs::set_permissions(&etc, std::fs::Permissions::from_mode(0o755)).unwrap();
        let hold = scratch.join("hold/admission-hold");
        std::fs::write(
            etc.join("gateway.env"),
            format!("BAYLEE_ADMISSION_HOLD={}\n", hold.display()),
        )
        .unwrap();
        let ctl = scratch.join("ctl");
        std::fs::create_dir_all(&ctl).unwrap();
        let stubs = scratch.join("stubs");
        std::fs::create_dir_all(&stubs).unwrap();
        let server = Self {
            calls: scratch.join("calls"),
            scratch,
            root,
            etc,
            ctl,
            stubs,
            hold,
        };
        server.set("rev", A);
        server.write_stubs();
        server
    }

    fn write_stubs(&self) {
        let stubs = &self.stubs;
        let real_git = which("git");
        stub(
            stubs,
            "git",
            &format!(
                r#"{NEXT}
for arg in "$@"; do [ "$arg" = verify-tag ] && exec {real_git} "$@"; done
[ "$1" = -C ] && shift 2
case "$1" in
    rev-parse) next rev "" ;;
    ls-remote) cat "$CTL/tags" 2>/dev/null ;;
    show-ref) exec {real_git} "$@" ;;
esac
exit 0"#,
                real_git = real_git.display()
            ),
        );
        stub(stubs, "cargo", "echo built");
        stub(
            stubs,
            "sudo",
            r#"[ "$1" = -n ] && { shift; exec "$@"; }
last=${!#}
case "$1" in
    test|sed|grep|tee|chmod) exec "$@" ;;
    install)
        [ "$last" = "$HOLD" ] && : > "$HOLD"
        [ "$2" = -d ] && [ "$last" = "$(dirname "$HOLD")" ] && mkdir -p "$last"
        ;;
    rm) [ "$last" = "$HOLD" ] && rm -f "$HOLD" ;;
esac
exit 0"#,
        );
        stub(stubs, "logger", "exit 0");
        stub(stubs, "flock", "exit 0");
        stub(stubs, "rustup", "exit 0");
        stub(
            stubs,
            "systemctl",
            r#"[ "$1 $2" = "cat baylee-feedback" ] && exit 1; exit 0"#,
        );
        stub(
            stubs,
            "curl",
            &format!(
                r#"{NEXT}
url=${{!#}}
case "$url" in
    */health)
        mode=$(next admission honours)
        [ "$mode" = down ] && exit 7
        running=$(next running 0)
        locally=$(next local_running "$running")
        waiting=$(next waiting 0)
        admission=''
        if [ "$mode" = honours ]; then
            if [ -e "$HOLD" ]; then admission=',"admission":"held"'; else admission=',"admission":"open"'; fi
        fi
        echo "{{\"games\":{{\"running\":$running,\"local_running\":$locally,\"waiting\":$waiting}}$admission}}"
        ;;
    */info)
        info=$(next info '')
        [ "$info" = down ] && exit 7
        [ -n "$info" ] || info="{{\"commit\":\"$(head -n1 "$CTL/rev")\",\"dirty\":false}}"
        echo "$info"
        ;;
esac
exit 0"#
            ),
        );
        // The dispatcher: notes what it was called with and in what state
        // the server was, then answers with the phase's next status.
        stub(
            stubs,
            "run-deploy-hooks",
            &format!(
                r#"{NEXT}
held=open; [ -e "$HOLD" ] && held=held
staged=unstaged; [ -d "$BAYLEE_DEPLOY_ROOT/state/staged" ] && staged=staged
echo "hooks $1 $2 ($held, $staged)" >> "$DEPLOY_CALLS"
echo "run-deploy-hooks: $1: neutral"
exit "$(next "code-$1" 0)""#
            ),
        );
    }

    /// Adds a hook file under `name` (its content is the dispatcher's
    /// business; here only its name counts).
    pub fn add_hook(&self, name: &str) {
        let dir = self.etc.join("deploy-hooks.d");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(dir.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Sets a `ctl` answer (several lines: one per question, the last kept).
    pub fn set(&self, name: &str, value: &str) {
        std::fs::write(self.ctl.join(name), format!("{value}\n")).unwrap();
    }

    /// The dispatcher's status for `phase`, one per call.
    pub fn hook_codes(&self, phase: &str, codes: &str) {
        self.set(&format!("code-{phase}"), codes);
    }

    pub fn state(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join("state").join(name))
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub fn deployed(&self) -> Option<String> {
        self.state("deployed")
    }

    pub fn staged(&self) -> bool {
        self.root.join("state/staged/rev").exists()
    }

    /// The transaction's `key`, if there is one.
    pub fn txn(&self, key: &str) -> Option<String> {
        self.state("transaction")?
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")).map(str::to_owned))
    }

    pub fn held(&self) -> bool {
        self.hold.exists()
    }

    /// Runs `baylee-deploy <args>` once.
    pub fn run(&self, args: &[&str]) -> Ran {
        let _ = std::fs::remove_file(&self.calls);
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/server/baylee-deploy");
        let out = std::process::Command::new("bash")
            .arg(&script)
            .args(args)
            .env("BAYLEE_DEPLOY_ROOT", &self.root)
            .env("BAYLEE_DEPLOY_ETC", &self.etc)
            .env(
                "BAYLEE_DEPLOY_HOOKS_BIN",
                self.stubs.join("run-deploy-hooks"),
            )
            .env("BAYLEE_DEPLOY_TRUSTED_UID", rustix_free_euid().to_string())
            .env("BAYLEE_DEPLOY_INFO_TRIES", "2")
            .env("CARGO", self.stubs.join("cargo"))
            .env("NPM", self.scratch.join("no-such-npm"))
            .env("TRUNK", self.scratch.join("no-such-trunk"))
            .env("RUSTUP", self.stubs.join("rustup"))
            .env("DEPLOY_CALLS", &self.calls)
            .env("CTL", &self.ctl)
            .env("HOLD", &self.hold)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.stubs.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .output()
            .expect("bash runs");
        Ran {
            ok: out.status.success(),
            code: out.status.code(),
            said: String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr),
            calls: std::fs::read_to_string(&self.calls).unwrap_or_default(),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// The effective uid, from `/proc/self/status` (this crate links no
/// rustix).
fn rustix_free_euid() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|l| l.strip_prefix("Uid:"))
                .and_then(|ids| ids.split_whitespace().nth(1))
                .and_then(|euid| euid.parse().ok())
        })
        .unwrap_or(0)
}

/// Where a real program is, before the stand-ins shadow it.
pub fn which(program: &str) -> PathBuf {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from(program))
}
