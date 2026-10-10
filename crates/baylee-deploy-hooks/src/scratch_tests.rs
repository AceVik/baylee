//! The dispatcher in a scratch tree: a [`Layout`] rooted in a temporary
//! directory, owned by root or by whoever runs the test. The installed
//! binary has no such seam; it always uses [`Layout::production`].

use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Layout;
use crate::args::Phase;
use crate::check::Refusal;
use crate::run::{Ended, Outcome, run};

/// One way to break a tree, named.
type Breaking = (&'static str, fn(&Tree, &Path));
/// One way to break a directory of a tree, named.
type BreakingDir = (&'static str, fn(&Tree));

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A tree with `etc/baylee/deploy-hooks.d` and `var/log`, all `0755`.
struct Tree {
    base: PathBuf,
}

impl Tree {
    fn new() -> Self {
        static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let scratch = std::env::temp_dir().join(format!(
            "baylee-deploy-hooks-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let base = scratch.join("root");
        for dir in ["etc/baylee/deploy-hooks.d", "var/log"] {
            std::fs::create_dir_all(base.join(dir)).unwrap();
        }
        for dir in [
            "",
            "etc",
            "etc/baylee",
            "etc/baylee/deploy-hooks.d",
            "var",
            "var/log",
        ] {
            chmod(&base.join(dir), 0o755);
        }
        Self { base }
    }

    fn hooks(&self) -> PathBuf {
        self.base.join("etc/baylee/deploy-hooks.d")
    }

    /// Where the hooks of these tests write what they did.
    fn out(&self) -> PathBuf {
        self.base.join("out")
    }

    fn said(&self) -> String {
        std::fs::read_to_string(self.out()).unwrap_or_default()
    }

    fn log(&self) -> PathBuf {
        self.base.join("var/log/baylee/deploy-hooks.log")
    }

    /// A `#!/bin/sh` hook, `0755` unless `mode` says otherwise.
    fn hook(&self, name: &str, body: &str) -> PathBuf {
        let path = self.hooks().join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        chmod(&path, 0o755);
        path
    }

    /// A hook that notes its name and arguments, then exits `code`.
    fn noting(&self, name: &str, code: i32) -> PathBuf {
        let out = self.out();
        self.hook(
            name,
            &format!("echo \"$0 $*\" >> '{}'\nexit {code}", out.display()),
        )
    }

    fn layout(&self) -> Layout {
        Layout {
            base: self.base.clone(),
            owners: vec![0, rustix::process::geteuid().as_raw()],
            require_root: false,
            budget: |_| Duration::from_secs(2),
            grace: Duration::from_millis(300),
            ..Layout::production()
        }
    }

    fn run(&self, phase: Phase) -> Outcome {
        run(&self.layout(), phase, SHA)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        if let Some(scratch) = self.base.parent() {
            let _ = std::fs::remove_dir_all(scratch);
        }
    }
}

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn is_root() -> bool {
    rustix::process::geteuid().is_root()
}

/// Whether a process is gone (or a zombie nobody has reaped yet), given
/// five seconds for a signal sent to it to land.
fn gone(pid: &str) -> bool {
    let dead = || {
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid.trim()));
        stat.map_or(true, |s| {
            s.rsplit(')')
                .next()
                .is_some_and(|rest| rest.trim_start().starts_with('Z'))
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !dead() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    dead()
}

#[test]
fn no_directory_an_empty_one_or_only_ignored_names_run_nothing() {
    let tree = Tree::new();
    assert!(matches!(tree.run(Phase::Prepare), Outcome::NoHooks));
    for name in [
        ".hidden",
        "10-example~",
        "10-example.bak",
        "10-example.dpkg-old",
        "-x",
    ] {
        tree.noting(name, 0);
    }
    assert!(matches!(tree.run(Phase::Prepare), Outcome::NoHooks));
    assert_eq!(tree.said(), "");
    std::fs::remove_dir_all(tree.hooks()).unwrap();
    let outcome = tree.run(Phase::Prepare);
    assert!(matches!(outcome, Outcome::NoHooks), "{outcome}");
    assert_eq!(outcome.code(), 0);
}

#[test]
fn hooks_run_in_byte_order_with_the_phase_and_the_commit() {
    let tree = Tree::new();
    for name in ["20-b", "10-a", "9-c", "Z-upper"] {
        tree.noting(name, 0);
    }
    for phase in Phase::ALL {
        let outcome = tree.run(phase);
        assert!(matches!(outcome, Outcome::Ran(4)), "{outcome}");
        assert_eq!(outcome.code(), 0);
    }
    let lines: Vec<String> = tree.said().lines().map(str::to_owned).collect();
    let mut wanted = Vec::new();
    for phase in Phase::ALL {
        // `$0` of a script is the descriptor path its interpreter opened.
        for _ in ["10-a", "20-b", "9-c", "Z-upper"] {
            wanted.push(format!("{phase} {SHA}"));
        }
    }
    let args: Vec<String> = lines
        .iter()
        .map(|l| l.split_once(' ').unwrap().1.to_owned())
        .collect();
    assert_eq!(args, wanted);
    // Which file ran is in the log, in order: 10-a, 20-b, 9-c, Z-upper.
    let log = std::fs::read_to_string(tree.log()).unwrap();
    let order: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("prepare") && l.ends_with("starts"))
        .map(|l| l.rsplit('(').next().unwrap().split(')').next().unwrap())
        .collect();
    assert_eq!(order, ["10-a", "20-b", "9-c", "Z-upper"]);
}

#[test]
fn a_hook_gets_a_fixed_environment_root_as_its_directory_and_no_input() {
    let tree = Tree::new();
    let out = tree.out();
    tree.hook(
        "10-example",
        &format!(
            "tr '\\0' '\\n' < /proc/$$/environ | grep -v '^$' | sort > '{0}.env'\npwd -P > '{0}.cwd'\ncat > '{0}.in'",
            out.display()
        ),
    );
    let outcome = tree.run(Phase::BeforeSwitch);
    assert!(matches!(outcome, Outcome::Ran(1)), "{outcome}");
    let env = std::fs::read_to_string(format!("{}.env", out.display())).unwrap();
    assert_eq!(env, "LANG=C.UTF-8\nPATH=/usr/sbin:/usr/bin:/sbin:/bin\n");
    let cwd = std::fs::read_to_string(format!("{}.cwd", out.display())).unwrap();
    assert_eq!(cwd, "/\n");
    let input = std::fs::read_to_string(format!("{}.in", out.display())).unwrap();
    assert_eq!(input, "");
}

#[test]
fn exit_75_is_not_ready_and_stops_the_phase() {
    let tree = Tree::new();
    tree.noting("10-a", 0);
    tree.noting("20-b", 75);
    tree.noting("30-c", 0);
    let outcome = tree.run(Phase::Prepare);
    assert!(
        matches!(outcome, Outcome::NotReady { index: 2, count: 3 }),
        "{outcome}"
    );
    assert_eq!(outcome.code(), 75);
    assert_eq!(tree.said().lines().count(), 2, "the third did not run");
}

#[test]
fn any_other_status_or_a_signal_fails_the_phase() {
    let tree = Tree::new();
    tree.noting("10-a", 3);
    tree.noting("20-b", 0);
    let outcome = tree.run(Phase::AfterSwitch);
    assert!(
        matches!(
            outcome,
            Outcome::Failed {
                index: 1,
                count: 2,
                how: Ended::Exit(3)
            }
        ),
        "{outcome}"
    );
    assert_eq!(outcome.code(), 1);
    assert_eq!(tree.said().lines().count(), 1);

    let tree = Tree::new();
    tree.hook("10-a", "kill -KILL $$");
    let outcome = tree.run(Phase::AfterSwitch);
    assert!(
        matches!(
            outcome,
            Outcome::Failed {
                how: Ended::Signal(9),
                ..
            }
        ),
        "{outcome}"
    );
    assert_eq!(outcome.code(), 1);
}

#[test]
fn a_hook_past_its_budget_is_stopped_with_its_whole_group() {
    let tree = Tree::new();
    let out = tree.out();
    // A child in the background that ignores SIGTERM, and the hook itself
    // waiting forever.
    tree.hook(
        "10-slow",
        &format!(
            "sh -c 'trap \"\" TERM; sleep 60' &\necho $! > '{}'\nsleep 60",
            out.display()
        ),
    );
    tree.noting("20-never", 0);
    let started = std::time::Instant::now();
    let outcome = tree.run(Phase::BeforeSwitch);
    assert!(
        matches!(
            outcome,
            Outcome::Failed {
                index: 1,
                how: Ended::TimedOut(_),
                ..
            }
        ),
        "{outcome}"
    );
    assert_eq!(outcome.code(), 1);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    let child = std::fs::read_to_string(&out).unwrap();
    assert!(gone(&child), "the background child {child} survived");
    assert!(!tree.said().contains("20-never"));
}

#[test]
fn what_a_hook_leaves_running_in_its_group_is_killed_when_it_exits() {
    let tree = Tree::new();
    let out = tree.out();
    tree.hook(
        "10-leaves",
        &format!("sleep 60 &\necho $! > '{}'\nexit 0", out.display()),
    );
    let outcome = tree.run(Phase::AfterSwitch);
    assert!(matches!(outcome, Outcome::Ran(1)), "{outcome}");
    let child = std::fs::read_to_string(&out).unwrap();
    assert!(gone(&child), "{child} survived its hook");
}

#[test]
fn output_goes_to_the_root_only_log_and_never_to_the_outcome() {
    let tree = Tree::new();
    tree.hook(
        "10-private-name",
        "echo private-stdout-line\necho private-stderr-line >&2\nexit 4",
    );
    let outcome = tree.run(Phase::Prepare);
    let said = outcome.to_string();
    assert_eq!(said, "hook 1 of 1 exited with status 4");
    for secret in ["private", "10-"] {
        assert!(!said.contains(secret), "{said}");
    }
    let log = std::fs::read_to_string(tree.log()).unwrap();
    assert!(log.contains("private-stdout-line\n"), "{log}");
    assert!(log.contains("private-stderr-line\n"), "{log}");
    let mode = std::fs::metadata(tree.log()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let dir = std::fs::metadata(tree.base.join("var/log/baylee")).unwrap();
    assert_eq!(dir.permissions().mode() & 0o777, 0o750);
}

/// Each case breaks one hook; a harmless one before it in byte order must
/// not have run, and the refusal names no hook.
#[test]
fn a_hook_that_fails_a_check_refuses_the_whole_phase() {
    let cases: [Breaking; 6] = [
        ("group-writable", |_, p| chmod(p, 0o775)),
        ("other-writable", |_, p| chmod(p, 0o757)),
        ("not executable", |_, p| chmod(p, 0o644)),
        ("a symbolic link", |t, p| {
            std::fs::remove_file(p).unwrap();
            let target = t.base.join("elsewhere");
            std::fs::write(&target, "#!/bin/sh\nexit 0\n").unwrap();
            chmod(&target, 0o755);
            symlink(&target, p).unwrap();
        }),
        ("a directory", |_, p| {
            std::fs::remove_file(p).unwrap();
            std::fs::create_dir(p).unwrap();
            chmod(p, 0o755);
        }),
        ("a relative interpreter", |_, p| {
            std::fs::write(p, "#!sh\nexit 0\n").unwrap();
        }),
    ];
    for (what, break_it) in cases {
        let tree = Tree::new();
        tree.noting("10-fine", 0);
        let bad = tree.noting("20-broken", 0);
        break_it(&tree, &bad);
        let outcome = tree.run(Phase::Prepare);
        assert!(
            matches!(
                outcome,
                Outcome::Refused(Refusal::Hook {
                    index: 2,
                    count: 2,
                    ..
                })
            ),
            "{what}: {outcome}"
        );
        assert_eq!(outcome.code(), 77, "{what}");
        assert_eq!(tree.said(), "", "{what}: something ran");
        assert!(!outcome.to_string().contains("broken"), "{what}: {outcome}");
        let log = std::fs::read_to_string(tree.log()).unwrap();
        assert!(
            log.contains("(20-broken)"),
            "{what}: the log names it: {log}"
        );
    }
}

#[test]
fn a_fifo_under_a_hook_name_is_refused_without_hanging() {
    let tree = Tree::new();
    let fifo = tree.hooks().join("10-fifo");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::from_raw_mode(0o755),
        0,
    )
    .unwrap();
    let outcome = tree.run(Phase::Prepare);
    assert!(
        matches!(outcome, Outcome::Refused(Refusal::Hook { .. })),
        "{outcome}"
    );
}

#[test]
fn an_interpreter_anyone_could_replace_is_refused() {
    let tree = Tree::new();
    // The scratch tree lives under the system's temporary directory, which
    // everybody may write to.
    let interpreter = tree.base.join("interpreter");
    std::fs::copy("/bin/sh", &interpreter).unwrap();
    chmod(&interpreter, 0o755);
    let hook = tree.hooks().join("10-example");
    std::fs::write(&hook, format!("#!{}\nexit 0\n", interpreter.display())).unwrap();
    chmod(&hook, 0o755);
    let outcome = tree.run(Phase::Prepare);
    assert!(
        matches!(outcome, Outcome::Refused(Refusal::Path { .. })),
        "{outcome}"
    );
    assert_eq!(outcome.code(), 77);
}

#[test]
fn a_bad_directory_on_the_way_refuses_even_with_good_hooks() {
    let cases: [BreakingDir; 6] = [
        ("the hook directory group-writable", |t| {
            chmod(&t.hooks(), 0o775);
        }),
        ("etc/baylee other-writable", |t| {
            chmod(&t.base.join("etc/baylee"), 0o757);
        }),
        ("etc group-writable", |t| chmod(&t.base.join("etc"), 0o775)),
        ("the base other-writable", |t| chmod(&t.base, 0o757)),
        ("the hook directory a link", |t| {
            let real = t.base.join("real-hooks");
            std::fs::rename(t.hooks(), &real).unwrap();
            symlink(&real, t.hooks()).unwrap();
        }),
        ("etc/baylee a link", |t| {
            let real = t.base.join("real-baylee");
            std::fs::rename(t.base.join("etc/baylee"), &real).unwrap();
            symlink(&real, t.base.join("etc/baylee")).unwrap();
        }),
    ];
    for (what, break_it) in cases {
        let tree = Tree::new();
        tree.noting("10-fine", 0);
        break_it(&tree);
        let outcome = tree.run(Phase::BeforeSwitch);
        assert!(
            matches!(outcome, Outcome::Refused(Refusal::Path { .. })),
            "{what}: {outcome}"
        );
        assert_eq!(outcome.code(), 77, "{what}");
        assert_eq!(tree.said(), "", "{what}");
    }
}

/// Ownership by somebody else can only be set up by root; elsewhere the
/// test says so and passes.
#[test]
fn a_hook_or_directory_owned_by_somebody_else_is_refused() {
    if !is_root() {
        eprintln!("not root: ownership cases skipped");
        return;
    }
    let nobody = 65534;
    let tree = Tree::new();
    tree.noting("10-fine", 0);
    let bad = tree.noting("20-foreign", 0);
    std::os::unix::fs::chown(&bad, Some(nobody), None).unwrap();
    let mut layout = tree.layout();
    layout.owners = vec![0];
    let outcome = run(&layout, Phase::Prepare, SHA);
    assert!(
        matches!(outcome, Outcome::Refused(Refusal::Hook { index: 2, .. })),
        "{outcome}"
    );
    assert_eq!(tree.said(), "");

    let tree = Tree::new();
    tree.noting("10-fine", 0);
    std::os::unix::fs::chown(tree.base.join("etc"), Some(nobody), None).unwrap();
    let mut layout = tree.layout();
    layout.owners = vec![0];
    let outcome = run(&layout, Phase::Prepare, SHA);
    assert!(
        matches!(outcome, Outcome::Refused(Refusal::Path { .. })),
        "{outcome}"
    );
    assert_eq!(tree.said(), "");
}

#[test]
fn a_dispatcher_that_must_be_root_and_is_not_refuses() {
    if is_root() {
        eprintln!("root: the not-root case cannot be shown");
        return;
    }
    let tree = Tree::new();
    tree.noting("10-fine", 0);
    let mut layout = tree.layout();
    layout.require_root = true;
    let outcome = run(&layout, Phase::Prepare, SHA);
    assert!(
        matches!(outcome, Outcome::Refused(Refusal::NotRoot)),
        "{outcome}"
    );
    assert_eq!(tree.said(), "");
}

#[test]
fn a_hook_replaced_after_the_check_is_not_what_runs() {
    // The descriptor is what runs: renaming another file over the name
    // between the check and the run cannot change the program. Shown by
    // running a hook that replaces a later hook's name before that one runs.
    let tree = Tree::new();
    let out = tree.out();
    let swap = tree.base.join("swap");
    std::fs::write(
        &swap,
        format!("#!/bin/sh\necho swapped >> '{}'\n", out.display()),
    )
    .unwrap();
    chmod(&swap, 0o755);
    tree.hook(
        "10-swapper",
        &format!(
            "mv '{}' '{}'",
            swap.display(),
            tree.hooks().join("20-original").display()
        ),
    );
    tree.hook(
        "20-original",
        &format!("echo original >> '{}'", out.display()),
    );
    let outcome = tree.run(Phase::Prepare);
    assert!(matches!(outcome, Outcome::Ran(2)), "{outcome}");
    assert_eq!(tree.said(), "original\n");
}
