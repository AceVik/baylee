//! Running the machine's own commands, the way the scripts did: each one's
//! status read off the process, failures that must stop a deploy turned
//! into [`Stop`], and every log line said on stdout and to syslog.
use std::{
    ffi::OsStr,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

/// Ends the command with this exit status; what it means was logged first.
#[derive(Debug, PartialEq, Eq)]
pub struct Stop(pub i32);

/// A command's result: `Err(Stop)` when it has to end the run.
pub type Run<T = ()> = Result<T, Stop>;

/// Says `msg` as `<tag>: msg` on stdout and to syslog (`logger`, best effort).
pub fn log(tag: &str, msg: &str) {
    println!("{tag}: {msg}");
    let _ = Command::new("logger")
        .args(["-t", tag, "--", msg])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// A command from its words.
pub fn cmd<S: AsRef<OsStr>>(words: &[S]) -> Command {
    let mut command = Command::new(&words[0]);
    command.args(&words[1..]);
    command
}

/// Runs with the caller's output; true when it exited 0.
pub fn ok(command: &mut Command) -> bool {
    command.status().is_ok_and(|s| s.success())
}

/// Runs with its output discarded; true when it exited 0.
pub fn quiet(command: &mut Command) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A command that must succeed: a failure stops the run with status 1, as
/// `set -e` stopped the script.
///
/// # Errors
/// [`Stop`]`(1)` when it did not run or exited non-zero.
pub fn must(command: &mut Command) -> Run {
    if ok(command) { Ok(()) } else { Err(Stop(1)) }
}

/// Its stdout when it exited 0 (stderr is the caller's).
pub fn out(command: &mut Command) -> Option<String> {
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Its stdout when it exited 0, its stderr discarded.
pub fn out_quiet(command: &mut Command) -> Option<String> {
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Feeds `input` on stdin; true when it exited 0. For secrets: what goes
/// through stdin shows on no command line.
pub fn feed(command: &mut Command, input: &[u8]) -> bool {
    let Ok(mut child) = command.stdin(Stdio::piped()).stdout(Stdio::null()).spawn() else {
        return false;
    };
    let written = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(input).is_ok());
    child.wait().is_ok_and(|s| s.success()) && written
}

/// Whether `program` would run: a path that is an executable file, or a
/// bare name found on PATH (`command -v`).
pub fn runnable(program: &str) -> bool {
    let executable = |path: &Path| path.is_file() && is_executable(path);
    if program.contains('/') {
        return executable(Path::new(program));
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| executable(&dir.join(program)))
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Seconds since the epoch (`date +%s`).
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The last value a root-only settings file gives `key`, read through sudo
/// (`sudo sed -n 's/^KEY=//p' file | tail -1`), surrounding quotes removed.
pub fn sudo_setting(file: &Path, key: &str) -> Option<String> {
    let text = out_quiet(
        Command::new("sudo")
            .args(["sed", "-n", &format!("s/^{key}=//p")])
            .arg(file),
    )?;
    let last = text.lines().last()?.to_owned();
    Some(last.trim_matches('"').to_owned())
}
