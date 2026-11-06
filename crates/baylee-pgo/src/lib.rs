//! A profile-guided build, as a release makes one.
//!
//! The steps, each of which stops the build if it fails:
//!
//! 1. Build `engine_perf` instrumented.
//! 2. Play the fixed games with it ([`TRAINED_ON`]), which writes the counts.
//! 3. Merge the counts into a profile. It belongs to this commit alone: nothing
//!    reads a profile another commit made.
//! 4. Build what ships with the profile, and `engine_perf` beside it in the same
//!    `cargo` run, so that the two share every crate and every feature.
//! 5. Build `engine_perf` as a plain build.
//! 6. Hold the two to each other: their `--verify` output must be byte-equal,
//!    state hashes included, in every shape of [`PROVEN_ON`].
//! 7. Require the profile to have taken, from what the compiler reports in step
//!    4: at most [`MOST_UNMATCHED_PERCENT`] of the profile's functions may be
//!    left without counts. A profile that matches nothing still plays the same
//!    games, so step 6 cannot see it, and it is an unprofiled build under another
//!    name: selecting different crates in steps 1 and 4 did that, gaining 3 to 6 %
//!    where a working profile gains about 30 %. This is the compiler's own count
//!    and not a stopwatch, so a busy machine cannot stop a good release.
//!
//! Every build starts from empty target directories. The flag reaches rustc past
//! cargo, which cannot see it, so a crate no commit touched would otherwise keep
//! the code an earlier commit's profile made.
//!
//! There is no fallback. [`Plan::build`] answers the directory of the profiled
//! binaries or a [`Failure`], and a caller that gets a failure stops the release:
//! an unprofiled build never ships silently. What does not link the engine learns
//! nothing here and is left out of [`Plan::ship`] (`docs/releasing.md`).
//!
//! The flags reach the workspace's own crates only, through
//! `RUSTC_WORKSPACE_WRAPPER` ([`wrap_rustc`]), and never through `RUSTFLAGS`, which
//! would change every dependency's build as well.

use std::{
    env,
    ffi::{OsStr, OsString},
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

/// How the flags reach [`wrap_rustc`], parted by [`FLAG_SEPARATOR`].
const FLAGS: &str = "BAYLEE_PGO_RUSTC_FLAGS";
const FLAG_SEPARATOR: char = '\x1f';

/// What the compiler says of a function it found no counts for, when asked.
const REPORT_UNMATCHED: &str = "-Cllvm-args=-pgo-warn-missing-function";
const UNMATCHED: &str = "no profile data available for function";

/// The workload's package and binary (`crates/baylee-perf`).
const PACKAGE: &str = "baylee-perf";
const WORKLOAD: &str = "engine_perf";

/// The shapes a profile trains on: a live table first, then the house's games.
pub const TRAINED_ON: [&str; 2] = ["frames", "house"];

/// The shapes a profiled build is held to the plain one on.
pub const PROVEN_ON: [&str; 3] = ["house", "frames", "decide"];

/// The share of a profile's functions the compiler may find no counts for.
///
/// Measured on the engine server: a working profile leaves 1 518 of 14 978
/// functions without counts, 10 %, all of them in `baylee-engine-server`'s own
/// crate, which the workload does not run. A selection that differs between the
/// two builds leaves 14 156, 95 %, across every crate.
pub const MOST_UNMATCHED_PERCENT: usize = 25;

/// The crate whose hash the client's two builds share ([`ClientPlan`]), and how
/// a function of it reads in a mangled name.
const PINNED_CRATE: &str = "baylee_client";
const OWN_CRATE: &str = "13baylee_client";

/// Tells [`wrap_rustc`] to pin that crate and to leave what is not built for the
/// target alone.
const PIN: &str = "BAYLEE_PGO_PIN_CRATE";

/// The fewest functions a client's training may reach. A run through the lobby
/// alone reaches more than this; a client that died at the door does not.
const FEWEST_CLIENT_FUNCTIONS: usize = 50_000;

/// The share of a client's functions the compiler may find no counts for.
pub const MOST_UNMATCHED_CLIENT_PERCENT: usize = 5;

/// Lines of a failed build's output kept in its [`Failure`].
const ERROR_LINES: usize = 30;

/// Why a profile-guided build stopped.
#[derive(Debug, PartialEq, Eq)]
pub struct Failure(pub String);

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

/// One profile-guided build.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The `cargo` to run.
    pub cargo: PathBuf,
    /// The `llvm-profdata` of this toolchain ([`profdata`]).
    pub profdata: PathBuf,
    /// A program that calls [`wrap_rustc`] first: cargo starts it in place of
    /// rustc for the workspace's crates.
    pub wrapper: PathBuf,
    /// The checkout, at the commit being built.
    pub root: PathBuf,
    /// Where this commit's counts and profile go.
    pub work: PathBuf,
    /// Where the instrumented and the profiled builds keep their target
    /// directories. Emptied first: see the crate's comment.
    pub targets: PathBuf,
    /// The plain build's target directory.
    pub plain: PathBuf,
    /// The cargo profile, `release` or `dist`.
    pub profile: String,
    /// The `cargo build` arguments naming what ships, `-p baylee-engine-server`.
    pub ship: Vec<String>,
}

impl Plan {
    /// Trains, builds and proves; the directory of the profiled binaries.
    ///
    /// # Errors
    /// [`Failure`] at the first step that fails, when the profiled build plays
    /// anything differently from the plain one, or when the profile took no
    /// effect.
    pub fn build(&self) -> Result<PathBuf, Failure> {
        let raw = self.work.join("raw");
        let _ = fs::remove_dir_all(&raw);
        let _ = fs::remove_dir_all(&self.targets);
        fs::create_dir_all(&raw).map_err(|e| Failure(format!("{}: {e}", raw.display())))?;

        // The same selection as the profiled build below, or the counts match
        // nothing: a function is found by its mangled name, which holds a hash
        // of its crate's features, and what a `cargo` run selects unifies them.
        let generate = self.targets.join("generate");
        let flag = format!("-Cprofile-generate={}", raw.display());
        self.cargo(&generate, &[&flag], &self.ship)?;
        for shape in TRAINED_ON {
            run(
                Command::new(self.workload(&generate))
                    .arg(shape)
                    .current_dir(&self.root)
                    .env("LLVM_PROFILE_FILE", raw.join("%m_%p.profraw")),
                &format!("training on {shape}"),
            )?;
        }
        let merged = self.work.join("merged.profdata");
        run(
            Command::new(&self.profdata)
                .arg("merge")
                .arg("-o")
                .arg(&merged)
                .arg(&raw),
            "merging the counts",
        )?;

        let profiled = self.targets.join("profiled");
        let flag = format!("-Cprofile-use={}", merged.display());
        let report = self.cargo(&profiled, &[&flag, REPORT_UNMATCHED], &self.ship)?;
        self.require_counts_used(&merged, &report)?;
        self.cargo(&self.plain, &[], &[])?;
        for shape in PROVEN_ON {
            let plain = self.verify(&self.plain, shape)?;
            let guided = self.verify(&profiled, shape)?;
            if plain != guided {
                return Err(Failure(format!(
                    "the profiled build does not play {shape} as the plain build does"
                )));
            }
        }
        Ok(self.out(&profiled))
    }

    /// Holds the compiler's `report` of the profiled build to the profile.
    fn require_counts_used(&self, merged: &Path, report: &str) -> Result<(), Failure> {
        let output = Command::new(&self.profdata)
            .arg("show")
            .arg(merged)
            .output()
            .map_err(|e| Failure(format!("reading the profile: {e}")))?;
        let shown = String::from_utf8_lossy(&output.stdout);
        let total: usize = shown
            .lines()
            .find_map(|line| line.strip_prefix("Total functions: "))
            .and_then(|count| count.trim().parse().ok())
            .ok_or_else(|| Failure(format!("the profile names no function count: {shown}")))?;
        let unmatched = report.matches(UNMATCHED).count();
        if total == 0 || unmatched * 100 > total * MOST_UNMATCHED_PERCENT {
            return Err(Failure(format!(
                "the profile took no effect: the compiler found no counts for {unmatched} functions \
                 against {total} in the profile, and {MOST_UNMATCHED_PERCENT} % is the most allowed"
            )));
        }
        Ok(())
    }

    /// Where a target directory's binaries are.
    fn out(&self, target: &Path) -> PathBuf {
        target.join(if self.profile == "dev" {
            "debug"
        } else {
            &self.profile
        })
    }

    fn workload(&self, target: &Path) -> PathBuf {
        self.out(target)
            .join(format!("{WORKLOAD}{}", env::consts::EXE_SUFFIX))
    }

    /// Builds the workload and `ship` in `target`, with `flags` on the
    /// workspace's crates, and answers what the compiler said.
    fn cargo(&self, target: &Path, flags: &[&str], ship: &[String]) -> Result<String, Failure> {
        let what = format!("building in {}", target.display());
        let mut build = Command::new(&self.cargo);
        build
            .args(["build", "--locked", "--profile", &self.profile])
            .args(["-p", PACKAGE, "--bin", WORKLOAD])
            .args(ship)
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", target);
        if !flags.is_empty() {
            build
                .env("RUSTC_WORKSPACE_WRAPPER", &self.wrapper)
                .env(FLAGS, flags.join(&FLAG_SEPARATOR.to_string()));
        }
        output_of(&mut build, &what)
    }

    fn verify(&self, target: &Path, shape: &str) -> Result<Vec<u8>, Failure> {
        let what = format!("verifying {shape} in {}", target.display());
        let output = Command::new(self.workload(target))
            .args([shape, "--verify"])
            .current_dir(&self.root)
            .output()
            .map_err(|e| Failure(format!("{what}: {e}")))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Failure(format!("{what}: {}", output.status)))
        }
    }
}

/// One profile-guided build of the client, on a machine with a screen.
///
/// The engine server's [`Plan`] trains on a workload that needs nothing but a
/// CPU. A client's time goes through a window and a GPU, so a person's machine
/// trains it, and the time goes into its dependencies (`bevy`, `wgpu`) as much
/// as into this workspace: the flags reach every crate built for the target,
/// through `RUSTC_WRAPPER`, and no build script or macro.
///
/// The harness that drives the training is the `dev-control` feature, and what
/// ships never has it. A feature changes its crate's hash and with it every name
/// in that crate, so the two builds would share nothing of `baylee-client`, where
/// a good part of the frame's code is instantiated. [`wrap_rustc`] pins that one
/// hash in both builds ([`PIN`]): the feature forwards to no dependency, so the
/// other crates are the same crates anyway. What differs between the two builds
/// then is only the harness's own code and the functions it is compiled into.
/// [`ClientPlan::build`] requires that from the compiler's report, for the
/// workspace's own crate as well as for the whole.
#[derive(Clone, Debug)]
pub struct ClientPlan {
    /// The `cargo` to run.
    pub cargo: PathBuf,
    /// The `llvm-profdata` of this toolchain ([`profdata`]).
    pub profdata: PathBuf,
    /// A program that calls [`wrap_rustc`] first.
    pub wrapper: PathBuf,
    /// The checkout, at the commit being built.
    pub root: PathBuf,
    /// Where this commit's counts and profile go.
    pub work: PathBuf,
    /// Where the two builds keep their target directories. Emptied first.
    pub targets: PathBuf,
    /// The cargo profile: `dist` for what ships.
    pub profile: String,
    /// The target triple, which has to be this machine's: it runs the build.
    pub triple: String,
}

/// What a finished [`ClientPlan`] made and what the compiler said of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientBuild {
    /// The directory of the profiled binaries.
    pub binaries: PathBuf,
    /// The merged profile.
    pub profile: PathBuf,
    /// Functions in the profile.
    pub functions: usize,
    /// Functions the compiler found no counts for.
    pub unmatched: usize,
    /// The functions of the client's own crate in the profile.
    pub own_functions: usize,
    /// Those of them the compiler found no counts for.
    pub own_unmatched: usize,
}

impl ClientPlan {
    /// Builds instrumented, has `train` run the binary, and builds what ships
    /// from the counts it left.
    ///
    /// `train` gets the instrumented client and the directory its runs write to
    /// through `LLVM_PROFILE_FILE`.
    ///
    /// # Errors
    /// [`Failure`] at the first step that fails, when training left nothing, and
    /// when the counts did not reach the build that ships.
    pub fn build(
        &self,
        train: impl FnOnce(&Path, &Path) -> Result<(), Failure>,
    ) -> Result<ClientBuild, Failure> {
        let raw = self.work.join("raw");
        let _ = fs::remove_dir_all(&raw);
        let _ = fs::remove_dir_all(&self.targets);
        fs::create_dir_all(&raw).map_err(|e| Failure(format!("{}: {e}", raw.display())))?;

        let generate = self.targets.join("generate");
        let flag = format!("-Cprofile-generate={}", raw.display());
        self.cargo(&generate, &[&flag], true)?;
        let client = self
            .out(&generate)
            .join(format!("baylee-client{}", env::consts::EXE_SUFFIX));
        train(&client, &raw)?;
        require_counts_written(&raw)?;

        let merged = self.work.join("merged.profdata");
        run(
            Command::new(&self.profdata)
                .arg("merge")
                .arg("-o")
                .arg(&merged)
                .arg(&raw),
            "merging the counts",
        )?;
        let functions = functions_in(&self.profdata, &merged)?;
        if functions < FEWEST_CLIENT_FUNCTIONS {
            return Err(Failure(format!(
                "the training reached {functions} functions, and {FEWEST_CLIENT_FUNCTIONS} is \
                 the fewest a client's run touches"
            )));
        }
        let own_functions = own_functions_in(&self.profdata, &merged)?;

        let profiled = self.targets.join("profiled");
        let flag = format!("-Cprofile-use={}", merged.display());
        let report = self.cargo(&profiled, &[&flag, REPORT_UNMATCHED], false)?;
        let built = ClientBuild {
            binaries: self.out(&profiled),
            profile: merged,
            functions,
            unmatched: report.matches(UNMATCHED).count(),
            own_functions,
            own_unmatched: report
                .lines()
                .filter(|line| line.contains(UNMATCHED) && line.contains(OWN_CRATE))
                .count(),
        };
        built.require_counts_used()?;
        Ok(built)
    }

    /// Builds what `cargo build-client` selects, in `target`.
    fn cargo(&self, target: &Path, flags: &[&str], harness: bool) -> Result<String, Failure> {
        let what = format!("building in {}", target.display());
        let mut build = Command::new(&self.cargo);
        build
            .args(["build-client", "--profile", &self.profile])
            .args(["--target", &self.triple])
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", target)
            .env("RUSTC_WRAPPER", &self.wrapper)
            .env(FLAGS, flags.join(&FLAG_SEPARATOR.to_string()))
            .env(PIN, PINNED_CRATE);
        if harness {
            build.args(["--features", "baylee-client/dev-control"]);
        } else {
            // What ships; the training must not look for updates.
            build.env("BAYLEE_RELEASE_BUILD", "1");
        }
        output_of(&mut build, &what)
    }

    /// Where a target directory's binaries are.
    fn out(&self, target: &Path) -> PathBuf {
        target.join(&self.triple).join(&self.profile)
    }
}

impl ClientBuild {
    /// Holds the compiler's report to the profile: as a whole, and for the
    /// client's own crate, which a hash that differs would empty alone.
    fn require_counts_used(&self) -> Result<(), Failure> {
        for (what, unmatched, total) in [
            ("the client", self.unmatched, self.functions),
            ("its own crate", self.own_unmatched, self.own_functions),
        ] {
            if total == 0 || unmatched * 100 > total * MOST_UNMATCHED_CLIENT_PERCENT {
                return Err(Failure(format!(
                    "the profile did not reach {what}: the compiler found no counts for \
                     {unmatched} functions against {total} in the profile, and \
                     {MOST_UNMATCHED_CLIENT_PERCENT} % is the most it may"
                )));
            }
        }
        Ok(())
    }
}

/// Stops when the training left no counts: a client killed instead of closed
/// writes none, and an empty merge would make a build with no profile.
fn require_counts_written(raw: &Path) -> Result<(), Failure> {
    let entries = fs::read_dir(raw).map_err(|e| Failure(format!("{}: {e}", raw.display())))?;
    let written = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.metadata().is_ok_and(|file| file.len() > 0))
        .count();
    if written == 0 {
        return Err(Failure(format!(
            "the training left no counts in {}",
            raw.display()
        )));
    }
    Ok(())
}

/// The functions a merged profile holds.
fn functions_in(profdata: &Path, merged: &Path) -> Result<usize, Failure> {
    let shown = shown_by(profdata, merged, &[])?;
    shown
        .lines()
        .find_map(|line| line.strip_prefix("Total functions: "))
        .and_then(|count| count.trim().parse().ok())
        .ok_or_else(|| Failure(format!("the profile names no function count: {shown}")))
}

/// The functions of the client's own crate a merged profile holds.
fn own_functions_in(profdata: &Path, merged: &Path) -> Result<usize, Failure> {
    let shown = shown_by(profdata, merged, &["--all-functions"])?;
    Ok(shown
        .lines()
        .filter(|line| line.contains(OWN_CRATE) && line.trim_end().ends_with(':'))
        .count())
}

fn shown_by(profdata: &Path, merged: &Path, flags: &[&str]) -> Result<String, Failure> {
    let output = Command::new(profdata)
        .arg("show")
        .args(flags)
        .arg(merged)
        .output()
        .map_err(|e| Failure(format!("reading the profile: {e}")))?;
    if !output.status.success() {
        return Err(Failure(format!("reading the profile: {}", output.status)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs a build and answers what the compiler said.
fn output_of(build: &mut Command, what: &str) -> Result<String, Failure> {
    let output = build
        .output()
        .map_err(|e| Failure(format!("{what}: {e}")))?;
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        return Ok(said);
    }
    let last: Vec<&str> = said.lines().rev().take(ERROR_LINES).collect();
    let last: Vec<&str> = last.into_iter().rev().collect();
    Err(Failure(format!(
        "{what}: {}\n{}",
        output.status,
        last.join("\n")
    )))
}

fn run(command: &mut Command, what: &str) -> Result<(), Failure> {
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(Failure(format!("{what}: {status}"))),
        Err(e) => Err(Failure(format!("{what}: {e}"))),
    }
}

/// This toolchain's `llvm-profdata`, which has to be the one rustc's LLVM wrote
/// the counts for: the toolchain component `llvm-tools`.
///
/// # Errors
/// [`Failure`] when `rustc` cannot be asked or the component is missing.
pub fn profdata(rustc: &OsStr) -> Result<PathBuf, Failure> {
    let ask = |args: &[&str]| -> Result<String, Failure> {
        let output = Command::new(rustc)
            .args(args)
            .output()
            .map_err(|e| Failure(format!("asking rustc: {e}")))?;
        if !output.status.success() {
            return Err(Failure(format!("asking rustc: {}", output.status)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    };
    let sysroot = ask(&["--print", "sysroot"])?;
    let version = ask(&["-vV"])?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| Failure("rustc names no host".to_owned()))?;
    let tool = Path::new(sysroot.trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin")
        .join(format!("llvm-profdata{}", env::consts::EXE_SUFFIX));
    if tool.is_file() {
        Ok(tool)
    } else {
        Err(Failure(format!(
            "{} is missing: rustup component add llvm-tools",
            tool.display()
        )))
    }
}

/// Runs rustc with the build's flags when cargo started this process in its place,
/// and answers its exit status; `None` when it did not, and the program goes on.
///
/// Call it first in `main` of a program named as [`Plan::wrapper`].
#[must_use]
pub fn wrap_rustc() -> Option<i32> {
    let flags = env::var(FLAGS).ok()?;
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();
    if args.is_empty() {
        return None;
    }
    let rustc = args.remove(0);
    let pin = env::var(PIN).ok();
    if let Some(name) = &pin {
        pin_metadata(&mut args, name);
    }
    let mut compile = Command::new(rustc);
    compile.args(&args);
    if pin.is_none() || built_for_the_target(&args) {
        compile.args(flags.split(FLAG_SEPARATOR).filter(|flag| !flag.is_empty()));
    }
    Some(
        compile
            .status()
            .map_or(1, |status| status.code().unwrap_or(1)),
    )
}

/// Whether this is a crate that ends up in the binary. Cargo names the target
/// only there: not for a build script or a macro, which run on this machine
/// during the build and would leave their own counts, nor for a question to
/// rustc, which compiles nothing.
fn built_for_the_target(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--crate-name")
        && args
            .iter()
            .any(|arg| arg == "--target" || arg.to_string_lossy().starts_with("--target="))
}

/// Gives the library `name` the same hash in every build, by what it is called
/// rather than by the features cargo hashed.
fn pin_metadata(args: &mut [OsString], name: &str) {
    let named = args
        .windows(2)
        .any(|pair| pair[0] == "--crate-name" && pair[1] == name);
    let binary = args
        .windows(2)
        .any(|pair| pair[0] == "--crate-type" && pair[1] == "bin");
    if !named || binary {
        return;
    }
    for arg in args.iter_mut() {
        if arg.to_string_lossy().starts_with("metadata=") {
            *arg = OsString::from(format!("metadata=baylee-pgo-{name}"));
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// `llvm-profdata`: merges into an empty file, and shows a hundred functions.
    const PROFDATA: &str = r#"[ "$1" = merge ] && : > "$3"
[ "$1" = show ] && echo "Total functions: 100"
exit 0"#;

    /// A stand-in for a program: a file of shell, written by the test.
    fn program(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A plan whose cargo drops an `engine_perf` into the target directory it is
    /// told, printing `plain_says` or `profiled_says`, and logs what it is asked.
    fn plan(root: &Path, plain_says: &str, profiled_says: &str, cargo_fails_on: &str) -> Plan {
        let cargo = root.join("bin/cargo");
        let log = root.join("log");
        program(
            &cargo,
            &format!(
                r#"echo "$CARGO_TARGET_DIR|$RUSTC_WORKSPACE_WRAPPER|$BAYLEE_PGO_RUSTC_FLAGS|$*" >> {log}
if [ -n "{cargo_fails_on}" ]; then
  case "$CARGO_TARGET_DIR" in *{cargo_fails_on}*) exit 3;; esac
fi
mkdir -p "$CARGO_TARGET_DIR/release"
case "$CARGO_TARGET_DIR" in
  *profiled*) says='{profiled_says}';;
  *) says='{plain_says}';;
esac
printf '#!/bin/sh\necho "$1" >> {log}.played\necho %s\n' "$says" > "$CARGO_TARGET_DIR/release/engine_perf"
chmod +x "$CARGO_TARGET_DIR/release/engine_perf""#,
                log = log.display(),
            ),
        );
        let profdata = root.join("bin/llvm-profdata");
        program(&profdata, PROFDATA);
        Plan {
            cargo,
            profdata,
            wrapper: root.join("bin/wrapper"),
            root: root.to_owned(),
            work: root.join("work"),
            targets: root.join("targets"),
            plain: root.join("plain"),
            profile: "release".to_owned(),
            ship: vec!["-p".to_owned(), "baylee-engine-server".to_owned()],
        }
    }

    #[test]
    fn a_build_trains_on_this_commit_and_proves_itself_against_the_plain_build() {
        let dir = tempfile::tempdir().unwrap();
        let out = plan(dir.path(), "same", "same", "").build().unwrap();
        assert_eq!(out, dir.path().join("targets/profiled/release"));
        let log = fs::read_to_string(dir.path().join("log")).unwrap();
        let builds: Vec<&str> = log.lines().collect();
        assert_eq!(builds.len(), 3, "{log}");
        assert!(builds[0].contains("generate|") && builds[0].contains("-Cprofile-generate="));
        assert!(builds[1].contains("profiled|") && builds[1].contains("-Cprofile-use="));
        // Selected alike, or the counts would name other functions.
        assert!(builds[0].contains("-p baylee-engine-server"), "{log}");
        assert!(builds[1].contains("-p baylee-engine-server"), "{log}");
        assert!(
            builds[2].contains("plain||"),
            "no wrapper on a plain build: {log}"
        );
        let played = fs::read_to_string(dir.path().join("log.played")).unwrap();
        assert_eq!(
            played.lines().collect::<Vec<_>>(),
            [
                "frames", "house", "house", "house", "frames", "frames", "decide", "decide"
            ]
        );
    }

    #[test]
    fn a_profiled_build_that_plays_differently_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let failure = plan(dir.path(), "same", "other", "").build().unwrap_err();
        assert!(failure.0.contains("does not play house"), "{failure}");
    }

    /// A plan whose profiled build reports `unmatched` functions without counts.
    fn reporting(root: &Path, unmatched: usize) -> Plan {
        let mut plan = plan(root, "same", "same", "");
        let reports = root.join("bin/cargo-reports");
        program(
            &reports,
            &format!(
                r#"{} "$@" || exit $?
case "$CARGO_TARGET_DIR" in *profiled*)
  i=0; while [ $i -lt {unmatched} ]; do
    echo "warning: no profile data available for function f$i" >&2; i=$((i+1))
  done;;
esac"#,
                plan.cargo.display()
            ),
        );
        plan.cargo = reports;
        plan
    }

    /// A profile that matched nothing plays the same games, so the proof cannot
    /// see it; the failure names the compiler's figures.
    #[test]
    fn a_profile_the_compiler_found_no_counts_for_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let failure = reporting(dir.path(), 60).build().unwrap_err();
        assert!(failure.0.contains("took no effect"), "{failure}");
        assert!(failure.0.contains("60 functions against 100"), "{failure}");
    }

    #[test]
    fn a_few_functions_without_counts_are_a_working_profile() {
        let dir = tempfile::tempdir().unwrap();
        reporting(dir.path(), MOST_UNMATCHED_PERCENT)
            .build()
            .unwrap();
    }

    /// The flag reaches rustc past cargo, so a target directory kept from an
    /// earlier commit would keep that commit's profile in every crate left alone.
    #[test]
    fn a_build_starts_from_empty_target_directories() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan(dir.path(), "same", "same", "");
        let stale = plan.targets.join("profiled/stale");
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, b"").unwrap();
        plan.build().unwrap();
        assert!(!stale.exists());
    }

    #[test]
    fn every_failed_build_stops_it() {
        for step in ["generate", "profiled", "plain"] {
            let dir = tempfile::tempdir().unwrap();
            let failure = plan(dir.path(), "same", "same", step).build().unwrap_err();
            assert!(failure.0.contains(step), "{step}: {failure}");
        }
    }

    #[test]
    fn a_failed_merge_stops_the_build() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan(dir.path(), "same", "same", "");
        program(&plan.profdata, "exit 7");
        let failure = plan.build().unwrap_err();
        assert!(failure.0.contains("merging"), "{failure}");
    }

    #[test]
    fn a_failed_training_run_stops_the_build() {
        let dir = tempfile::tempdir().unwrap();
        let mut plan = plan(dir.path(), "same", "same", "");
        let cargo = dir.path().join("bin/cargo-no-workload");
        program(&cargo, r#"mkdir -p "$CARGO_TARGET_DIR/release""#);
        plan.cargo = cargo;
        let failure = plan.build().unwrap_err();
        assert!(failure.0.contains("training on frames"), "{failure}");
    }

    #[test]
    fn a_toolchain_without_the_component_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let rustc = dir.path().join("rustc");
        program(
            &rustc,
            &format!(
                r#"[ "$1" = --print ] && echo {}; [ "$1" = -vV ] && echo "host: x"; exit 0"#,
                dir.path().display()
            ),
        );
        let failure = profdata(rustc.as_os_str()).unwrap_err();
        assert!(
            failure.0.contains("rustup component add llvm-tools"),
            "{failure}"
        );
        let tool = dir.path().join("lib/rustlib/x/bin/llvm-profdata");
        program(&tool, "exit 0");
        assert_eq!(profdata(rustc.as_os_str()).unwrap(), tool);
    }
}

#[cfg(test)]
mod pin_tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    /// Two builds that differ by a feature end up with one hash for the library.
    #[test]
    fn the_client_library_gets_one_hash_in_both_builds() {
        let mut with_harness = args(&["--crate-name", "baylee_client", "-C", "metadata=aaa"]);
        let mut shipped = args(&["--crate-name", "baylee_client", "-C", "metadata=bbb"]);
        pin_metadata(&mut with_harness, "baylee_client");
        pin_metadata(&mut shipped, "baylee_client");
        assert_eq!(with_harness, shipped);
    }

    /// Every other crate keeps what cargo gave it, and so does the binary of the
    /// same name, which links the library.
    #[test]
    fn nothing_else_is_pinned() {
        let other = args(&["--crate-name", "bevy_ui", "-C", "metadata=aaa"]);
        let mut pinned = other.clone();
        pin_metadata(&mut pinned, "baylee_client");
        assert_eq!(pinned, other);
        let binary = args(&[
            "--crate-name",
            "baylee_client",
            "--crate-type",
            "bin",
            "-C",
            "metadata=aaa",
        ]);
        let mut pinned = binary.clone();
        pin_metadata(&mut pinned, "baylee_client");
        assert_eq!(pinned, binary);
    }

    #[test]
    fn a_build_script_and_a_question_to_rustc_get_no_flags() {
        let crate_for_the_target = args(&["--crate-name", "bevy_ui", "--target", "aarch64"]);
        assert!(built_for_the_target(&crate_for_the_target));
        assert!(!built_for_the_target(&args(&[
            "--crate-name",
            "build_script_build"
        ])));
        assert!(!built_for_the_target(&args(&[
            "--target", "aarch64", "--print", "cfg"
        ])));
    }

    fn build(unmatched: usize, own_unmatched: usize) -> ClientBuild {
        ClientBuild {
            binaries: PathBuf::new(),
            profile: PathBuf::new(),
            functions: 1000,
            unmatched,
            own_functions: 100,
            own_unmatched,
        }
    }

    /// The crate the pin exists for is held on its own: a hash that differs
    /// empties that crate alone, which the whole's share could not show.
    #[test]
    fn a_profile_that_missed_the_clients_crate_stops_the_build() {
        assert!(build(20, 2).require_counts_used().is_ok());
        let failure = build(20, 90).require_counts_used().unwrap_err();
        assert!(failure.0.contains("its own crate"), "{failure}");
        assert!(build(900, 2).require_counts_used().is_err());
    }

    #[test]
    fn a_training_that_wrote_nothing_stops_the_build() {
        let dir = tempfile::tempdir().unwrap();
        assert!(require_counts_written(dir.path()).is_err());
        fs::write(dir.path().join("empty.profraw"), b"").unwrap();
        assert!(require_counts_written(dir.path()).is_err());
        fs::write(dir.path().join("run.profraw"), b"counts").unwrap();
        assert!(require_counts_written(dir.path()).is_ok());
    }
}
