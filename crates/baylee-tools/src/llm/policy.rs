//! Local filesystem, identity, subprocess and answer validation policy.
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Seek, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};

pub(super) const FILE_LIMIT: u64 = 4 * 1024 * 1024;
pub(super) const OUTPUT_LIMIT: u64 = 16 * 1024 * 1024;
const FIELDS: [&str; 13] = [
    "index = index::",
    "oracle_id = ",
    "scryfall_id = ",
    "name = ",
    "mana_cost = ",
    "types = ",
    "supertypes = ",
    "subtypes = ",
    "power = ",
    "toughness = ",
    "loyalty = ",
    "defense = ",
    "color_identity = ",
];

pub(super) fn read(path: &Path, limit: u64) -> Result<String> {
    let file = fs::File::open(path).context("cannot open lane input")?;
    ensure!(
        file.metadata()?.is_file(),
        "lane input must be a regular file"
    );
    let mut text = String::new();
    file.take(limit + 1)
        .read_to_string(&mut text)
        .context("lane input is not UTF-8")?;
    ensure!(text.len() as u64 <= limit, "lane input exceeds size limit");
    Ok(text)
}

pub(super) fn relative(root: &Path, text: &str) -> Result<PathBuf> {
    let rel = Path::new(text);
    ensure!(
        !text.is_empty() && !text.contains('\\') && !text.chars().any(char::is_control),
        "invalid repository path"
    );
    ensure!(
        rel.components()
            .all(|part| matches!(part, Component::Normal(_))),
        "repository path must be relative without traversal"
    );
    let mut at = root.to_path_buf();
    for part in rel.components() {
        at.push(part);
        let meta = fs::symlink_metadata(&at).context("repository path does not exist")?;
        ensure!(!meta.file_type().is_symlink(), "symlink in repository path");
    }
    ensure!(
        at.is_file() && at.canonicalize()?.starts_with(root),
        "repository input escapes root or is not a file"
    );
    Ok(at)
}

pub(super) fn card_path(root: &Path, text: &str) -> Result<PathBuf> {
    ensure!(
        text.starts_with("crates/baylee-cards/src/cards/")
            && Path::new(text)
                .extension()
                .is_some_and(|extension| extension == "rs"),
        "worklist may only name card Rust files"
    );
    relative(root, text)
}

pub(super) fn scratch(worklist: &Path, root: &Path) -> Result<(String, PathBuf)> {
    let worklist = worklist.canonicalize().context("worklist does not exist")?;
    let directory = worklist
        .parent()
        .context("worklist has no directory")?
        .to_path_buf();
    ensure!(
        !directory.starts_with(root),
        "worklist and outputs must live outside the repository"
    );
    let stem = worklist
        .file_stem()
        .and_then(|s| s.to_str())
        .context("worklist stem is not UTF-8")?
        .to_owned();
    ensure!(
        !stem.is_empty() && !stem.chars().any(|c| c.is_control() || "/\\".contains(c)),
        "unsafe worklist stem"
    );
    Ok((stem, directory))
}

pub(super) fn write(parent: &Path, relative: &Path, text: &str) -> Result<PathBuf> {
    ensure!(
        relative
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "unsafe output path"
    );
    let parent = parent.canonicalize()?;
    let mut directory = parent.clone();
    let rel_parent = relative.parent().context("output has no parent")?;
    for part in rel_parent.components() {
        directory.push(part);
        if !directory.exists() {
            fs::create_dir(&directory)?;
        }
        let meta = fs::symlink_metadata(&directory)?;
        ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "symlink or non-directory in output path"
        );
    }
    let path = parent.join(relative);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "output is not a regular file"
        );
    }
    ensure!(
        directory.canonicalize()?.starts_with(&parent),
        "output escapes scratch directory"
    );
    let mut temp = tempfile::NamedTempFile::new_in(&directory)?;
    temp.write_all(text.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(&path).map_err(|error| error.error)?;
    ensure!(
        read(&path, OUTPUT_LIMIT)? == text,
        "written output did not verify"
    );
    Ok(path)
}

pub(super) struct Lock {
    path: PathBuf,
    _file: fs::File,
}
impl Lock {
    pub fn acquire(path: PathBuf) -> Result<Self> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .context("LLM lane is locked; inspect the lock owner before removing a stale lock")?;
        writeln!(file, "{}", std::process::id())?;
        Ok(Self { path, _file: file })
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn headers(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("//!"))
        .collect()
}
fn fields<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with(prefix))
        .collect()
}

pub(super) fn keeps_header(stub: &str, written: &str) -> Result<()> {
    syn::parse_file(written)
        .map_err(|_| anyhow::anyhow!("generated source is not syntactically valid Rust"))?;
    ensure!(
        headers(stub) == headers(written),
        "generated header changed"
    );
    for field in FIELDS {
        ensure!(
            fields(stub, field) == fields(written, field),
            "generated identity field changed"
        );
    }
    ensure!(written.contains("card!("), "no card macro in answer");
    ensure!(
        !written.contains("GENERATED STUB") && !written.contains("TODO(card)"),
        "answer still contains stub markers"
    );
    ensure!(!written.contains('\0'), "NUL in generated source");
    Ok(())
}

pub(super) fn restore_generated(stub: &str, written: &str) -> Option<String> {
    if !written.contains("card!(")
        || written.contains("GENERATED STUB")
        || written.contains("TODO(card)")
    {
        return None;
    }
    let mut lines: Vec<_> = written.lines().map(str::to_owned).collect();
    let head = headers(stub);
    let indices: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| line.starts_with("//!").then_some(i))
        .collect();
    if indices.len() != head.len() {
        return None;
    }
    for (i, header) in indices.into_iter().zip(head) {
        header.clone_into(&mut lines[i]);
    }
    for field in FIELDS {
        let want = fields(stub, field);
        let indices: Vec<_> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| line.trim().starts_with(field).then_some(i))
            .collect();
        if want.len() != indices.len() {
            return None;
        }
        for (i, want) in indices.into_iter().zip(want) {
            let indentation = lines[i].len() - lines[i].trim_start().len();
            lines[i] = format!("{}{want}", &lines[i][..indentation]);
        }
    }
    let fixed = lines.join("\n") + "\n";
    keeps_header(stub, &fixed).ok()?;
    Some(fixed)
}

pub(super) fn code(answer: &str) -> Result<&str> {
    let mut offset = 0;
    let mut found = None;
    while let Some(start) = answer[offset..].find("```") {
        let start = offset + start;
        let tail = &answer[start + 3..];
        let newline = tail.find('\n').context("unterminated code fence")?;
        let language = tail[..newline].trim();
        let body_start = start + 3 + newline + 1;
        let end = answer[body_start..]
            .find("```")
            .context("truncated code fence")?
            + body_start;
        if language.is_empty() || language == "rust" {
            ensure!(found.is_none(), "ambiguous multiple Rust code fences");
            found = Some(&answer[body_start..end]);
        }
        offset = end + 3;
    }
    let body = found.context("no Rust code fence in answer")?;
    ensure!(
        !body.trim().is_empty() && !body.contains('\0'),
        "empty or invalid Rust code"
    );
    Ok(body)
}

pub(super) fn test_source(body: &str) -> Result<()> {
    ensure!(!body.contains('\0'), "NUL in test source");
    let parsed = syn::parse_file(body)
        .map_err(|_| anyhow::anyhow!("test fragment is not syntactically valid Rust"))?;
    ensure!(
        parsed.attrs.is_empty() && parsed.items.len() == 2,
        "test fragment must contain one handle and one test function"
    );
    let mut tests = 0;
    for item in &parsed.items {
        let syn::Item::Fn(function) = item else {
            bail!("test fragment may contain only functions");
        };
        ensure!(
            matches!(function.sig.safety, syn::Safety::Default) && function.sig.abi.is_none(),
            "test fragment may not declare unsafe or foreign functions"
        );
        ensure!(
            function
                .attrs
                .iter()
                .all(|attr| attr.path().is_ident("test") || attr.path().is_ident("doc")),
            "test fragment may not disable or alter test execution through attributes"
        );

        tests += function
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("test"))
            .count();
    }
    ensure!(tests == 1, "answer must contain exactly one #[test]");
    ensure!(
        !body.contains("include!(")
            && !body.contains("include_str!(")
            && !body.contains("include_bytes!("),
        "test may not include external files"
    );
    ensure!(
        !body.lines().any(|line| {
            let line = line.trim();
            line.starts_with("mod ")
                || line.starts_with("use ")
                || line.starts_with("pub mod ")
                || line.starts_with("extern crate ")
        }),
        "test must be an assembly fragment without modules or imports"
    );
    Ok(())
}

pub(super) fn door(path: &str) -> &'static str {
    for name in [
        "lands",
        "creatures",
        "artifacts",
        "enchantments",
        "instants",
        "sorceries",
        "planeswalkers",
    ] {
        if path.contains(&format!("cards/{name}/")) {
            return name;
        }
    }
    "rules"
}

pub(super) fn bound(name: &str, default: u64, maximum: u64) -> Result<u64> {
    let value = match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .with_context(|| format!("{name} must be an integer"))?,
        Err(std::env::VarError::NotPresent) => default,
        Err(_) => bail!("configuration is not UTF-8"),
    };
    ensure!(
        (1..=maximum).contains(&value),
        "{name} must be between 1 and {maximum}"
    );
    Ok(value)
}

pub(super) struct Process {
    pub status: ExitStatus,
    pub stdout: String,
    pub seconds: f64,
}

pub(super) fn process(
    command: &mut Command,
    deadline: Instant,
    monitor: impl FnMut() -> Result<()>,
) -> Result<Process> {
    process_with_stdin(command, deadline, false, monitor)
}

pub(super) fn process_with_stdin(
    command: &mut Command,
    deadline: Instant,
    interactive: bool,
    mut monitor: impl FnMut() -> Result<()>,
) -> Result<Process> {
    let started = Instant::now();
    let capture_directory = command
        .get_envs()
        .find(|(name, _)| *name == std::ffi::OsStr::new("TMPDIR"))
        .and_then(|(_, value)| value)
        .map(PathBuf::from);
    let mut output = if let Some(directory) = &capture_directory {
        tempfile::tempfile_in(directory)?
    } else {
        tempfile::tempfile()?
    };
    let errors = if let Some(directory) = &capture_directory {
        tempfile::tempfile_in(directory)?
    } else {
        tempfile::tempfile()?
    };
    command
        .stdin(if interactive {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(errors.try_clone()?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|_| anyhow::anyhow!("could not start lane subprocess"))?;
    let mut next_monitor = Instant::now();
    let result = (|| {
        loop {
            ensure!(
                Instant::now() < deadline,
                "lane subprocess exceeded deadline"
            );
            ensure!(
                output.metadata()?.len() <= OUTPUT_LIMIT
                    && errors.metadata()?.len() <= OUTPUT_LIMIT,
                "lane subprocess exceeded output limit"
            );
            if Instant::now() >= next_monitor {
                monitor()?;
                next_monitor = Instant::now() + Duration::from_secs(2);
            }
            if let Some(status) = child.try_wait()? {
                break Ok(status);
            }
            thread::sleep(Duration::from_millis(50));
        }
    })();
    // Kill the process group too: a deadline must stop an agent's children.
    #[cfg(unix)]
    {
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let status = result?;
    output.rewind()?;
    let mut stdout = String::new();
    output
        .take(OUTPUT_LIMIT + 1)
        .read_to_string(&mut stdout)
        .context("subprocess output is not UTF-8")?;
    ensure!(
        stdout.len() as u64 <= OUTPUT_LIMIT,
        "subprocess output too large"
    );
    Ok(Process {
        status,
        stdout,
        seconds: started.elapsed().as_secs_f64(),
    })
}

pub(super) fn unique(paths: &[String]) -> Result<()> {
    let mut found = BTreeSet::new();
    for path in paths {
        ensure!(found.insert(path), "duplicate path in worklist");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const STUB: &str = "//! Oak\ncard!(\n index = index::OAK,\n oracle_id = \"real\",\n scryfall_id = \"print\",\n // GENERATED STUB\n);\n";
    #[test]
    fn identity_is_exact_and_repair_is_verified() {
        let good = STUB.replace(
            " // GENERATED STUB\n",
            " coverage = Coverage::Implemented,\n",
        );
        assert!(keeps_header(STUB, &good).is_ok());
        let bad = good
            .replace("\"real\"", "\"invented\"")
            .replace("//! Oak", "//! Elm");
        assert!(keeps_header(STUB, &bad).is_err());
        assert_eq!(restore_generated(STUB, &bad).unwrap(), good);
        assert!(keeps_header(STUB, &(good.clone() + "//! invented\n")).is_err());
        assert!(restore_generated(STUB, "fn unrelated() {}\n").is_none());
    }
    #[test]
    fn answer_fences_fail_closed() {
        assert_eq!(
            code("text\n```rust\nfn oak() {}\n```\n").unwrap(),
            "fn oak() {}\n"
        );
        for text in [
            "",
            "STUB: unsupported",
            "```rust\ntruncated",
            "```rust\na\n```\n```rust\nb\n```",
        ] {
            assert!(code(text).is_err());
        }
        assert!(
            test_source("fn oak() -> CardIndex { card_index(\"id\") }\n#[test]\nfn test() {}\n")
                .is_ok()
        );
        assert!(test_source("#[test]\nfn a() {}\n#[test]\nfn b() {}\n").is_err());
        assert!(test_source("use secret::*;\n#[test]\nfn a() {}\n").is_err());
    }
    #[test]
    fn path_and_scratch_fences_and_locks() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("card.rs"), "old").unwrap();
        let root_path = root.path().canonicalize().unwrap();
        assert!(relative(&root_path, "../card.rs").is_err());
        assert!(relative(&root_path, "/card.rs").is_err());
        assert!(card_path(&root_path, "card.rs").is_err());
        assert!(scratch(&root_path.join("card.rs"), &root_path).is_err());
        let lock = root_path.join("lane.lock");
        let owner = Lock::acquire(lock.clone()).unwrap();
        assert!(Lock::acquire(lock.clone()).is_err());
        drop(owner);
        assert!(Lock::acquire(lock).is_ok());
        assert!(write(&root_path, Path::new("../escape"), "bad").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_input_and_output_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "unchanged").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
        let root = root.path().canonicalize().unwrap();
        assert!(relative(&root, "linked/secret").is_err());
        assert!(write(&root, Path::new("linked/secret"), "bad").is_err());
        assert_eq!(
            fs::read_to_string(outside.path().join("secret")).unwrap(),
            "unchanged"
        );
    }
    #[test]
    fn command_aliases_route_test_doors() {
        assert_eq!(
            door("crates/baylee-cards/src/cards/lands/basic/oak.rs"),
            "lands"
        );
        assert_eq!(door("other"), "rules");
    }
    #[cfg(unix)]
    #[test]
    fn subprocess_deadline_and_exit_status_are_verified_offline() {
        let output = process(
            Command::new("/bin/sh").args(["-c", "printf fixture"]),
            Instant::now() + Duration::from_secs(5),
            || Ok(()),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, "fixture");
        let failed = process(
            Command::new("/bin/sh").args(["-c", "exit 7"]),
            Instant::now() + Duration::from_secs(5),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(failed.status.code(), Some(7));
        let expired = process(
            Command::new("/bin/sh").args(["-c", "sleep 10"]),
            Instant::now() + Duration::from_millis(50),
            || Ok(()),
        );
        assert!(expired.is_err());
    }
}
