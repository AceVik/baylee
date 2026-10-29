//! Stop one native job supervisor and verify its owned children exited.
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    job: PathBuf,
    #[arg(long, default_value_t = 15.0)]
    timeout: f64,
    #[arg(long)]
    out: Option<PathBuf>,
}
fn main() -> Result<()> {
    let args = Arguments::parse();
    anyhow::ensure!(
        args.out.as_ref().is_none_or(|path| !path.exists()),
        "preserving existing stop receipt; choose a fresh output path"
    );
    #[cfg(target_os = "linux")]
    {
        let receipt = baylee_train_ops::legacy::stop::stop(&args.job, args.timeout)?;
        if let Some(out) = args.out {
            baylee_train_ops::files::atomic_json(&out, &receipt)?;
        }
        println!("{}", serde_json::to_string(&receipt)?);
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        anyhow::bail!("identity-bound native job stopping requires Linux")
    }
}
