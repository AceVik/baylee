//! CLI boundary for native training operations.
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Parser)]
#[command(about = "Native bounded training operations")]
struct Args {
    #[command(subcommand)]
    command: Task,
}
#[derive(Subcommand)]
enum Task {
    /// Account for every assigned CPU research attempt after bounded child cleanup.
    CollectResearch {
        /// Retain only a verified cleaned-up deadline as incomplete collection accounting.
        #[arg(long)]
        retain_deadline: bool,
        #[arg(long)]
        job: PathBuf,
        #[arg(long)]
        run: PathBuf,
        #[arg(long, default_value_t = 3600.0)]
        timeout: f64,
        #[arg(long, default_value_t = 48.0)]
        memory_gib: f64,
        #[arg(long)]
        input: Vec<PathBuf>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    /// Finalize a preserved research source only after verified owned-group cleanup.
    FinalizeResearch {
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        job: PathBuf,
    },
    /// Execute one immutable attempt. Failures always require a fresh directory.
    Run {
        #[arg(long)]
        job: PathBuf,
        #[arg(long, default_value_t = 3600.0)]
        timeout: f64,
        #[arg(long, default_value_t = 48.0)]
        memory_gib: f64,
        #[arg(long)]
        input: Vec<PathBuf>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    /// Stream SHA256 seals without allocating whole datasets.
    Hash { paths: Vec<PathBuf> },
    /// Atomically publish a JSON value supplied on stdin by a tensor worker.
    WriteJson {
        #[arg(long)]
        out: PathBuf,
    },
    /// Append one complete JSON metric record under a native file lock.
    AppendJson {
        #[arg(long)]
        out: PathBuf,
    },
    /// Atomically publish a verified byte-identical artifact.
    CopyFile {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Seal a tensor implementation tree, including module directory replacements.
    SourceDigest {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, required = true)]
        include: Vec<PathBuf>,
    },
    /// Certify roles, complete histories and content identities before PPO.
    AdmitPolicy {
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        source_checkpoint: PathBuf,
        #[arg(long)]
        collection_manifest: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Revalidate a receipt against the actual immutable dataset and source.
    VerifyAdmission {
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        source_checkpoint: PathBuf,
        #[arg(long)]
        admission: PathBuf,
    },
    /// Execute an explicit ordered experiment with bounded jobs and required outputs.
    Pipeline {
        #[arg(long)]
        manifest: PathBuf,
    },
}
#[cfg(target_os = "linux")]
fn interrupt_flag() -> Result<Arc<AtomicBool>> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&interrupted);
    ctrlc::set_handler(move || {
        signal.store(true, std::sync::atomic::Ordering::Relaxed);
    })?;
    Ok(interrupted)
}

fn publish_json(out: &std::path::Path, append: bool) -> Result<()> {
    let value: serde_json::Value = serde_json::from_reader(std::io::stdin().lock())?;
    if append {
        baylee_train_ops::files::append_json(out, &value)
    } else {
        baylee_train_ops::files::atomic_json(out, &value)
    }
}

fn finalize_research(run: &std::path::Path, job: &std::path::Path) -> Result<()> {
    // One block per system is the function's value, so neither leaves an
    // unreachable `Ok(())` behind the other's `bail!` (clippy on macOS).
    #[cfg(target_os = "linux")]
    {
        println!(
            "{}",
            serde_json::to_string(&baylee_train_ops::research_attempts::finalize(run, job)?)?
        );
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (run, job);
        anyhow::bail!("research attempt recovery requires Linux")
    }
}

fn run_native(
    job: PathBuf,
    timeout: f64,
    memory_gib: f64,
    input: Vec<PathBuf>,
    command: Vec<String>,
    research: Option<(PathBuf, bool)>,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let interrupted = interrupt_flag()?;
        let spec = baylee_train_ops::guard::Job {
            directory: job,
            cwd: std::env::current_dir()?,
            command,
            timeout_seconds: timeout,
            memory_gib,
            inputs: input,
        };
        if let Some((root, retain_deadline)) = research {
            let collect = if retain_deadline {
                baylee_train_ops::research_attempts::collect_retaining_deadline
            } else {
                baylee_train_ops::research_attempts::collect
            };
            println!(
                "{}",
                serde_json::to_string(&collect(&spec, &root, &interrupted)?)?
            );
        } else if !baylee_train_ops::guard::run(&spec, &interrupted)? {
            std::process::exit(1);
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (job, timeout, memory_gib, input, command, research);
        anyhow::bail!("bounded process-group jobs require Linux")
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Task::FinalizeResearch { run, job } => finalize_research(&run, &job)?,
        Task::CollectResearch {
            retain_deadline,
            job,
            run,
            timeout,
            memory_gib,
            input,
            command,
        } => run_native(
            job,
            timeout,
            memory_gib,
            input,
            command,
            Some((run, retain_deadline)),
        )?,
        Task::CopyFile { source, out } => {
            println!("{}", baylee_train_ops::files::atomic_copy(&source, &out)?);
        }
        Task::SourceDigest { root, include } => {
            println!(
                "{}",
                baylee_train_ops::digests::source_digest(&root, &include)?
            );
        }
        Task::WriteJson { out } => publish_json(&out, false)?,
        Task::AppendJson { out } => publish_json(&out, true)?,
        Task::Pipeline { manifest } => {
            #[cfg(target_os = "linux")]
            {
                let interrupted = interrupt_flag()?;
                println!(
                    "{}",
                    serde_json::to_string(&baylee_train_ops::pipeline::run(
                        &manifest,
                        &interrupted
                    )?)?
                );
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = manifest;
                anyhow::bail!("bounded pipeline requires Linux");
            }
        }
        Task::AdmitPolicy {
            data,
            source_checkpoint,
            collection_manifest,
            out,
        } => {
            let result = baylee_train_ops::admission::admit(
                &data,
                &source_checkpoint,
                &collection_manifest,
            )?;
            baylee_train_ops::files::atomic_json(&out, &result)?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Task::VerifyAdmission {
            data,
            source_checkpoint,
            admission,
        } => {
            println!(
                "{}",
                serde_json::to_string(&baylee_train_ops::admission::verify(
                    &data,
                    &source_checkpoint,
                    &admission
                )?)?
            );
        }
        Task::Hash { paths } => {
            for path in paths {
                println!(
                    "{}  {}",
                    baylee_train_ops::files::sha256(&path)?,
                    path.display()
                );
            }
        }
        Task::Run {
            job,
            timeout,
            memory_gib,
            input,
            command,
        } => run_native(job, timeout, memory_gib, input, command, None)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_commands_require_explicit_source_and_keep_literal_child_arguments() {
        let parsed = Args::try_parse_from([
            "train_ops",
            "collect-research",
            "--run",
            "/run",
            "--job",
            "/job",
            "--timeout",
            "10",
            "--memory-gib",
            "1",
            "--",
            "eval4",
            "--collection-purpose",
            "failure_terminal_research_v1",
        ])
        .unwrap();
        let Task::CollectResearch {
            run,
            job,
            command,
            retain_deadline,
            ..
        } = parsed.command
        else {
            panic!("wrong task")
        };
        assert_eq!(run, PathBuf::from("/run"));
        assert!(!retain_deadline);
        assert_eq!(job, PathBuf::from("/job"));
        assert_eq!(
            command,
            [
                "eval4",
                "--collection-purpose",
                "failure_terminal_research_v1"
            ]
        );
        assert!(Args::try_parse_from(["train_ops", "collect-research", "--run", "/run"]).is_err());
        let opted_in = Args::try_parse_from([
            "train_ops",
            "collect-research",
            "--retain-deadline",
            "--run",
            "/run",
            "--job",
            "/job",
            "--",
            "eval4",
        ])
        .unwrap();
        assert!(matches!(
            opted_in.command,
            Task::CollectResearch {
                retain_deadline: true,
                ..
            }
        ));
        assert!(matches!(
            Args::try_parse_from([
                "train_ops",
                "finalize-research",
                "--run",
                "/run",
                "--job",
                "/job/job.json"
            ])
            .unwrap()
            .command,
            Task::FinalizeResearch { .. }
        ));
        assert!(Args::try_parse_from(["train_ops", "finalize-research", "--run", "/run"]).is_err());
    }
}
