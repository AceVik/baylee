//! Scratch-tree agent execution with a mandatory macOS filesystem sandbox.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use std::fmt::Write as _;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{Job, policy};

#[derive(Clone, Copy, Debug, Deserialize)]
pub(super) struct Metrics {
    pub steps: u64,
    pub sent: u64,
    pub refused: u64,
}

pub(super) fn transcript(db: &Path, deadline: Instant) -> Result<Metrics> {
    let metadata = fs::symlink_metadata(db).context("transcript does not exist")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "transcript must be a regular file"
    );
    let query = "SELECT count(*) AS steps, coalesce(sum(prior),0) AS sent, coalesce(sum(refused),0) AS refused FROM (SELECT sum(length(cast(step_payload AS blob))+coalesce(length(cast(metadata AS blob)),0)) OVER (ORDER BY idx ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) AS prior, CASE WHEN step_payload LIKE '%RESOURCE_EXHAUSTED%' OR metadata LIKE '%RESOURCE_EXHAUSTED%' OR step_payload LIKE '%API error (attempt %' THEN 1 ELSE 0 END AS refused FROM steps);";
    let output = policy::process(
        Command::new("sqlite3")
            .args(["-readonly", "-json"])
            .arg(db)
            .arg(query),
        deadline.min(Instant::now() + Duration::from_secs(5)),
        || Ok(()),
    )?;
    ensure!(
        output.status.success(),
        "cannot read agent telemetry; sqlite3 is required"
    );
    let mut rows: Vec<Metrics> = serde_json::from_str(&output.stdout)
        .map_err(|_| anyhow::anyhow!("invalid transcript metrics"))?;
    ensure!(rows.len() == 1, "invalid transcript metric row count");
    Ok(rows.remove(0))
}

fn conversations() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is unavailable")?;
    let directory = PathBuf::from(home).join(".gemini/antigravity-cli/conversations");
    ensure!(
        directory.is_dir(),
        "agent conversations directory is missing; sign in with agy first"
    );
    directory
        .canonicalize()
        .context("cannot resolve conversations directory")
}

fn databases(directory: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "db") {
            paths.insert(path);
        }
    }
    Ok(paths)
}

fn fresh_metrics(
    directory: &Path,
    before: &BTreeSet<PathBuf>,
    deadline: Instant,
) -> Result<Option<Metrics>> {
    let now = databases(directory)?;
    let fresh: Vec<_> = now.difference(before).collect();
    ensure!(
        fresh.len() <= 1,
        "multiple agent sessions appeared; telemetry attribution is ambiguous"
    );
    fresh.first().map(|db| transcript(db, deadline)).transpose()
}

fn hash_tree(root: &Path) -> Result<BTreeMap<PathBuf, [u8; 32]>> {
    fn walk(
        root: &Path,
        at: &Path,
        output: &mut BTreeMap<PathBuf, [u8; 32]>,
        budget: &mut u64,
    ) -> Result<()> {
        for entry in fs::read_dir(at)? {
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "agent created a symlink"
            );
            if metadata.is_dir() {
                walk(root, &path, output, budget)?;
            } else {
                ensure!(metadata.is_file(), "agent created a special file");
                *budget = budget
                    .checked_add(metadata.len())
                    .context("scratch tree size overflow")?;
                ensure!(*budget <= 512 * 1024 * 1024, "scratch tree exceeds 512 MiB");
                let text = policy::read(&path, policy::FILE_LIMIT)?;
                output.insert(
                    path.strip_prefix(root)?.to_path_buf(),
                    Sha256::digest(text.as_bytes()).into(),
                );
            }
        }
        Ok(())
    }
    let mut output = BTreeMap::new();
    walk(root, root, &mut output, &mut 0)?;
    Ok(output)
}

fn stage_repository(
    root: &Path,
    stage: &Path,
    deadline: Instant,
) -> Result<BTreeMap<PathBuf, [u8; 32]>> {
    // Copy only the text sources the agent needs; no corpus, credentials,
    // binary assets, target directory, Git metadata or untracked material.
    let listed = policy::process(
        Command::new("git")
            .args(["ls-files", "-z"])
            .current_dir(root),
        deadline,
        || Ok(()),
    )?;
    ensure!(
        listed.status.success(),
        "cannot enumerate tracked repository inputs"
    );
    let mut bytes = 0_u64;
    for rel in listed.stdout.split('\0').filter(|rel| !rel.is_empty()) {
        let path = Path::new(rel);
        let eligible = path
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "md" || ext == "toml")
            && (rel.starts_with("crates/")
                || rel.starts_with("docs/")
                || rel.starts_with("scripts/llm/prompts/")
                || ["CLAUDE.md", "AGENTS.md", "Cargo.toml"].contains(&rel));
        if !eligible {
            continue;
        }
        let source = policy::relative(root, rel)?;
        let text = policy::read(&source, policy::FILE_LIMIT)?;
        bytes += text.len() as u64;
        ensure!(
            bytes <= 512 * 1024 * 1024,
            "repository context exceeds scratch limit"
        );
        let target = stage.join(path);
        fs::create_dir_all(target.parent().context("stage target has no parent")?)?;
        fs::write(target, text)?;
    }
    hash_tree(stage)
}

fn sandbox_quote(path: &Path) -> Result<String> {
    let path = path.to_str().context("sandbox path is not UTF-8")?;
    ensure!(
        !path.chars().any(char::is_control),
        "control character in sandbox path"
    );
    Ok(format!(
        "\"{}\"",
        path.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn sandbox(root: &Path, stage: &Path, conversations: &Path, io: &Path) -> Result<String> {
    ensure!(
        Path::new("/usr/bin/sandbox-exec").is_file(),
        "Gemini lane requires macOS sandbox-exec; refusing unsandboxed execution"
    );
    Ok(format!(
        "(version 1)\n(allow default)\n(deny file-write*)\n(deny file-read* (subpath {}))\n(allow file-write* (subpath {}) (subpath {}) (subpath {}) (literal \"/dev/null\"))\n",
        sandbox_quote(root)?,
        sandbox_quote(stage)?,
        sandbox_quote(conversations)?,
        sandbox_quote(io)?
    ))
}

fn validate_changes(
    before: &BTreeMap<PathBuf, [u8; 32]>,
    after: &BTreeMap<PathBuf, [u8; 32]>,
    allowed: &BTreeSet<PathBuf>,
) -> Result<()> {
    for path in before.keys().chain(after.keys()) {
        if before.get(path) != after.get(path) {
            ensure!(
                allowed.contains(path),
                "agent changed a file outside its worklist; nothing imported"
            );
        }
    }
    Ok(())
}

fn secrets() -> Vec<String> {
    let mut values: Vec<String> = std::env::vars()
        .filter(|(name, value)| {
            value.len() >= 6
                && ["KEY", "TOKEN", "SECRET", "PASSWORD"]
                    .iter()
                    .any(|part| name.to_uppercase().contains(part))
        })
        .map(|(_, value)| value)
        .collect();
    if let Some(home) = std::env::var_os("HOME")
        && let Ok(auth) = policy::read(
            &PathBuf::from(home).join(".local/share/opencode/auth.json"),
            1024 * 1024,
        )
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&auth)
    {
        fn collect(value: &serde_json::Value, output: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (name, value) in map {
                        if ["key", "token", "secret", "password", "access", "refresh"]
                            .iter()
                            .any(|part| name.to_lowercase().contains(part))
                        {
                            if let Some(value) = value.as_str().filter(|value| value.len() >= 6) {
                                output.push(value.to_owned());
                            }
                        } else {
                            collect(value, output);
                        }
                    }
                }
                serde_json::Value::Array(array) => {
                    for value in array {
                        collect(value, output);
                    }
                }
                _ => (),
            }
        }
        collect(&value, &mut values);
    }
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values
}

fn redact(mut text: String, values: &[String]) -> String {
    for value in values {
        text = text.replace(value, "[REDACTED]");
    }
    // Cover credentials emitted by an auth helper even when they were never
    // present in this process's environment or opencode file.
    let re = regex::Regex::new(r#"(?i)(?:bearer\s+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)[\s\"':=]+)[A-Za-z0-9_./+\-=]{6,}|\bsk-[A-Za-z0-9_-]{6,}"#).expect("constant credential regex");
    text = re.replace_all(&text, "[REDACTED]").into_owned();
    text
}

// Session setup, sandbox launch and verified import form one transaction.
#[allow(clippy::too_many_lines)]
pub(super) fn run(
    root: &Path,
    jobs: &[Job],
    stem: &str,
    output: &Path,
    tests: bool,
    minutes: u64,
) -> Result<()> {
    ensure!(
        (1..=240).contains(&minutes),
        "minutes must be between 1 and 240"
    );
    let deadline = Instant::now() + Duration::from_secs(minutes * 60);
    let conversations = conversations()?;
    let before_db = databases(&conversations)?;
    let token_limit = policy::bound("BAYLEE_LLM_MAX_INPUT_TOKENS", 250_000_000, 1_000_000_000)?;
    let temp = tempfile::tempdir()?;
    let base = temp.path().canonicalize()?;
    let stage = base.join("repository");
    let io = base.join("runtime");
    fs::create_dir(&stage)?;
    fs::create_dir(&io)?;
    let before = stage_repository(root, &stage, deadline)?;
    let generated = stage.join(".lane-tests");
    if tests {
        fs::create_dir(&generated)?;
    }
    let contract = if tests {
        "scripts/llm/prompts/tests-gemini.md"
    } else {
        "scripts/llm/prompts/cards-gemini.md"
    };
    let mut prompt = policy::read(&policy::relative(root, contract)?, policy::FILE_LIMIT)?;
    prompt = prompt
        .replace("/Users/viktor/Projects/baylee", &stage.to_string_lossy())
        .replace("<SCRATCH>", &generated.to_string_lossy());
    prompt.push_str("\nOnly edit the listed files in this scratch repository. Do not run cargo or codegen, create commits, change Git metadata, or access the original repository.\n");
    for (i, job) in jobs.iter().enumerate() {
        writeln!(prompt, "{}. {}\n    `{}`", i + 1, job.name, job.rel)?;
    }
    let suffix = if tests { "-tests" } else { "" };
    let sensitive = secrets();
    ensure!(
        sensitive.iter().all(|secret| !prompt.contains(secret)),
        "prompt contains a credential; refusing execution"
    );
    policy::write(
        output,
        Path::new(&format!("{stem}{suffix}-sent.md")),
        &prompt,
    )?;
    let profile = sandbox(root, &stage, &conversations, &io)?;
    let profile_path = base.join("agent.sb");
    fs::write(&profile_path, profile)?;
    let model = std::env::var("BAYLEE_LLM_AGY_MODEL")
        .unwrap_or_else(|_| "gemini-3.8-flash-high".to_owned());
    ensure!(
        !model.is_empty()
            && model.len() <= 128
            && model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.[]".contains(c)),
        "invalid agent model name"
    );
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .arg("-f")
        .arg(&profile_path)
        .arg("agy")
        .args([
            "-p",
            &prompt,
            "--model",
            &model,
            "--print-timeout",
            &format!("{minutes}m"),
        ])
        .current_dir(&stage)
        .env_clear();
    for name in ["HOME", "PATH", "LANG", "TERM"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command.env("TMPDIR", &io);
    let interactive = match std::env::var("BAYLEE_LLM_AGY_PERMISSIONS").as_deref() {
        Ok("ask") => true,
        Ok("skip") | Err(std::env::VarError::NotPresent) => {
            command.arg("--dangerously-skip-permissions");
            false
        }
        _ => bail!("BAYLEE_LLM_AGY_PERMISSIONS must be ask or skip"),
    };
    let result = policy::process_with_stdin(&mut command, deadline, interactive, || {
        if let Some(metrics) = fresh_metrics(&conversations, &before_db, deadline)? {
            ensure!(
                metrics.refused == 0,
                "agent API refused the request; output suppressed"
            );
            ensure!(
                metrics.sent / 4 <= token_limit,
                "agent exceeded input token estimate limit"
            );
        }
        Ok(())
    })?;
    let answer = redact(result.stdout, &sensitive);
    policy::write(
        output,
        Path::new(&format!("{stem}{suffix}-answer.md")),
        &answer,
    )?;
    ensure!(
        result.status.success(),
        "agent subprocess failed; stderr suppressed"
    );
    let metrics = fresh_metrics(&conversations, &before_db, deadline)?
        .context("agent produced no new transcript; completion cannot be verified")?;
    ensure!(
        metrics.steps > 0 && metrics.refused == 0 && metrics.sent / 4 <= token_limit,
        "agent telemetry indicates refusal or token limit"
    );
    let after = hash_tree(&stage)?;
    let allowed: BTreeSet<PathBuf> = jobs
        .iter()
        .map(|job| {
            if tests {
                PathBuf::from(".lane-tests").join(format!("{}.rs", job.slug))
            } else {
                PathBuf::from(&job.rel)
            }
        })
        .collect();
    validate_changes(&before, &after, &allowed)?;
    let mut pending = Vec::new();
    let mut lines = Vec::new();
    let mut missing = 0;
    for job in jobs {
        let path = if tests {
            generated.join(format!("{}.rs", job.slug))
        } else {
            stage.join(&job.rel)
        };
        if !path.exists()
            || (!tests && before.get(Path::new(&job.rel)) == after.get(Path::new(&job.rel)))
        {
            missing += 1;
            lines.push(format!(
                "{} | {} | no verified output",
                job.name,
                if tests { "skip" } else { "stub" }
            ));
            continue;
        }
        let written = policy::read(&path, policy::FILE_LIMIT)?;
        ensure!(
            sensitive.iter().all(|secret| !written.contains(secret)),
            "credential appeared in generated source; nothing imported"
        );
        if tests {
            policy::test_source(&written)?;
            pending.push((job, written));
        } else {
            let written = if policy::keeps_header(&job.source, &written).is_ok() {
                written
            } else {
                policy::restore_generated(&job.source, &written)
                    .context("agent changed a card's generated identity; nothing imported")?
            };
            pending.push((job, written));
        }
    }
    for job in jobs {
        ensure!(
            policy::read(&policy::card_path(root, &job.rel)?, policy::FILE_LIMIT)? == job.source,
            "card changed during agent run; nothing imported"
        );
    }
    for (job, written) in &pending {
        if tests {
            policy::write(
                output,
                &PathBuf::from(format!("{stem}-tests")).join(format!("{}.rs", job.slug)),
                written,
            )?;
        } else {
            policy::write(root, Path::new(&job.rel), written)?;
        }
        lines.push(format!(
            "{} | {} | verified",
            job.name,
            if tests {
                "written"
            } else if written.contains("Coverage::Partial") {
                "partial"
            } else {
                "implemented"
            }
        ));
    }
    let report = format!(
        "{} items ({model}), {:.1} min, {} steps, ~{} input tokens\n{}\n",
        jobs.len(),
        result.seconds / 60.0,
        metrics.steps,
        metrics.sent / 4,
        lines.join("\n")
    );
    let report_path = policy::write(
        output,
        Path::new(&format!("{stem}{suffix}-report.md")),
        &report,
    )?;
    println!(
        "{} of {} files verified; report in {}",
        pending.len(),
        jobs.len(),
        report_path.display()
    );
    ensure!(
        missing == 0,
        "agent left items without verified output; see report"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_output_must_match_worklist() {
        let path = PathBuf::from("card.rs");
        let before = BTreeMap::from([(path.clone(), [0; 32])]);
        let after = BTreeMap::from([(path.clone(), [1; 32])]);
        assert!(validate_changes(&before, &after, &BTreeSet::new()).is_err());
        assert!(validate_changes(&before, &after, &BTreeSet::from([path])).is_ok());
        let after = BTreeMap::from([(PathBuf::from("extra.rs"), [1; 32])]);
        assert!(validate_changes(&before, &after, &BTreeSet::new()).is_err());
    }
    #[test]
    fn credentials_are_redacted_from_reports() {
        let text =
            "known-value Bearer abcdefghijkl sk-anothersecret api_key=secret-value".to_owned();
        let text = redact(text, &["known-value".into()]);
        for secret in [
            "known-value",
            "abcdefghijkl",
            "sk-anothersecret",
            "secret-value",
        ] {
            assert!(!text.contains(secret));
        }
    }
    #[test]
    fn profile_paths_are_escaped() {
        assert_eq!(
            sandbox_quote(Path::new("/tmp/a\"b")).unwrap(),
            "\"/tmp/a\\\"b\""
        );
        assert!(sandbox_quote(Path::new("/tmp/a\nb")).is_err());
    }
    #[test]
    fn sqlite_fixture_measures_prefix_sum_without_reading_text() {
        // A missing optional telemetry executable is an explicit runtime
        // error, not a reason for an offline transport test to contact a model.
        if Command::new("sqlite3").arg("-version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("session.db");
        let created = Command::new("sqlite3").arg(&db).arg("CREATE TABLE steps(idx INTEGER,step_payload TEXT,metadata TEXT); INSERT INTO steps VALUES(0,'abc',NULL),(1,'de','x'),(2,'é',NULL);").output().unwrap();
        assert!(created.status.success());
        let metrics = transcript(&db, Instant::now() + Duration::from_secs(5)).unwrap();
        assert_eq!((metrics.steps, metrics.sent, metrics.refused), (3, 9, 0));
        let updated = Command::new("sqlite3")
            .arg(&db)
            .arg("INSERT INTO steps VALUES(3,'RESOURCE_EXHAUSTED',NULL);")
            .output()
            .unwrap();
        assert!(updated.status.success());
        assert_eq!(
            transcript(&db, Instant::now() + Duration::from_secs(5))
                .unwrap()
                .refused,
            1
        );
    }
}
