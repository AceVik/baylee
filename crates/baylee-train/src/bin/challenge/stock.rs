use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use baylee_account::{Bearer, CentralHttp, Endpoint, Token, wire};
use baylee_challenges::{Package, stock};
use baylee_train::challenges::{self, Limits};
use central_api::challenges::{Period, Publication};
use zeroize::Zeroizing;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("endpoint").required(true).args(["live", "central_dev"])))]
pub(super) struct Args {
    /// Matching source checkout; required explicitly for live publication.
    #[arg(long, required_if_eq("live", "true"))]
    source_dir: Option<PathBuf>,
    /// Use the compiled production Central endpoint (explicit operator selection).
    #[arg(long)]
    live: bool,
    /// Isolated Central fixture; numeric loopback HTTP origin with a port.
    #[arg(long)]
    central_dev: Option<String>,
    /// Private persistent staging and proof directory, reused after restarts.
    #[arg(long)]
    state_dir: PathBuf,
    /// Days to keep ahead, including the current Daily and Weekly.
    #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u8).range(1..=24))]
    days: u8,
    /// Candidate budget per empty slot. Exhaustion is an error, never a fallback.
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u8).range(1..=32))]
    candidates: u8,
}

pub(super) fn run(args: &Args) -> Result<()> {
    let source = args
        .source_dir
        .clone()
        .unwrap_or_else(baylee_train::working::repo_root);
    if args.live {
        super::source::verify_live(&source)?;
    }
    let generator = challenges::Generator::from_source(&source)?;
    if args.live {
        super::source::verify_live(&source)?;
    }
    let endpoint = args.central_dev.as_ref().map_or_else(
        || Ok(Endpoint::live()),
        |value| Endpoint::development(value),
    )?;
    let secret = Zeroizing::new(
        std::env::var("BAYLEE_CHALLENGE_BRIDGE_TOKEN")
            .context("BAYLEE_CHALLENGE_BRIDGE_TOKEN is required")?,
    );
    let credential = Token::from_hex(&secret).context("invalid bridge credential encoding")?;
    drop(secret);
    let http = CentralHttp::new(endpoint);
    private_directory(&args.state_dir)?;
    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let report = stock::reconcile(
        now,
        args.days,
        |request| match http.exchange(
            &wire::Request::ChallengeHost { request },
            Some(Bearer::Capability(&credential)),
        )? {
            wire::Response::ChallengeHost { response } => Ok(response),
            _ => Err(central_api::Error::Unavailable),
        },
        |period| {
            stage(&args.state_dir, period, args.candidates, &generator).map_err(|error| {
                // Errors never contain the credential, a package or a private record.
                eprintln!("{} slot {}: {error}", period.kind.name(), period.starts_at);
                stock::Error::Uncertified
            })
        },
    )?;
    println!(
        "Challenge stock: {} already present, {} published",
        report.present, report.published
    );
    Ok(())
}

fn stage(
    root: &Path,
    period: Period,
    candidates: u8,
    generator: &challenges::Generator,
) -> Result<Publication> {
    let ready = root.join(format!("{}-{}.json", period.kind.name(), period.starts_at));
    if ready.try_exists()? {
        return read_ready(&ready, period);
    }
    for _ in 0..candidates {
        // Independent OS randomness; neither dates nor public task IDs reveal the shuffle.
        let random = Token::generate()?.to_secret_bytes();
        let mut seed = u64::from_le_bytes(random[..8].try_into().expect("32 random bytes"));
        if period.kind == central_api::challenges::Kind::Daily {
            // Rotate resource positions; only two template bits are public.
            // The remaining 62 shuffle/policy bits remain independently random.
            let template = period.starts_at.div_euclid(86_400).rem_euclid(4) as u64;
            seed = (seed & !3) | ((template + 41) % 4);
        }
        let Ok(candidate) = generator.generate(period.kind, seed, Limits::default()) else {
            continue;
        };
        let publication = candidate.publication(period)?;
        let evidence = root.join(uuid::Uuid::now_v7().to_string());
        private_directory(&evidence)?;
        for (index, proof) in candidate.proofs.iter().enumerate() {
            private_write(
                &evidence.join(format!("proof-{index}.jsonl")),
                &proof.record,
            )?;
        }
        let source = evidence.join("publication.json");
        private_write(&source, &central_api::encode(&publication)?)?;
        sync_directory(&evidence)?;
        // A second worker cannot replace a slot's prepared content. Both workers
        // retain their evidence; Central still enforces the final immutable slot.
        match fs::hard_link(&source, &ready) {
            Ok(()) => {
                sync_directory(root)?;
                return Ok(publication);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return read_ready(&ready, period);
            }
            Err(error) => return Err(error.into()),
        }
    }
    anyhow::bail!("candidate budget exhausted without three diverse winning proofs")
}

fn read_ready(path: &Path, period: Period) -> Result<Publication> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(central_api::MAX_WIRE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let publication: Publication = central_api::decode(&bytes)?;
    ensure!(
        publication.description.period == period,
        "staged calendar mismatch"
    );
    Package::from_publication(&publication)?;
    Ok(publication)
}

fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let metadata = fs::symlink_metadata(path)?;
        ensure!(
            metadata.is_dir() && metadata.permissions().mode().trailing_zeros() >= 6,
            "private evidence directory must be a real directory with owner-only access"
        );
    }
    Ok(())
}

pub(super) fn private_new_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

pub(super) fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::challenges::Kind;

    #[test]
    fn staged_proofs_survive_a_worker_restart_without_regeneration() {
        let root = tempfile::tempdir().unwrap();
        let period = Period::containing(Kind::Daily, 1_794_182_400).unwrap();
        let generator =
            challenges::Generator::from_source(&baylee_train::working::repo_root()).unwrap();
        let first = stage(root.path(), period, 1, &generator).unwrap();
        let again = stage(root.path(), period, 0, &generator).unwrap();
        assert_eq!(
            first.digest, again.digest,
            "zero candidate budget proves the durable cache was used"
        );
        let paths = fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(
            paths.len(),
            2,
            "one slot pointer and one evidence directory"
        );
        let evidence = paths.iter().find(|path| path.is_dir()).unwrap();
        assert_eq!(
            fs::read_dir(evidence).unwrap().count(),
            4,
            "three complete records and their publication"
        );
        for proof in &first.certificate.witnesses {
            use sha2::{Digest, Sha256};
            assert!((0..3).any(|index| {
                let bytes = fs::read(evidence.join(format!("proof-{index}.jsonl"))).unwrap();
                let digest: [u8; 32] = Sha256::digest(&bytes).into();
                digest == proof.record_digest
            }));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                fs::metadata(evidence).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(evidence.join("publication.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}
