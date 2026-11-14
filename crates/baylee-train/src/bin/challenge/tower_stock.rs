//! Private durable complete graphs survive worker crashes and ambiguous HTTP replies.
use super::stock::{private_new_directory, private_write, sync_directory};
use anyhow::{Context, Result, ensure};
use baylee_challenges::tower::{MAX_BUNDLE_BYTES, Prepared};
use baylee_train::challenges::{Generator, Limits};
use central_api::{Token, challenges::Period};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

pub(super) fn stage(
    root: &Path,
    period: Period,
    candidates: u8,
    generator: &Generator,
) -> Result<Prepared> {
    let ready = root.join(format!("tower-{}.json", period.starts_at));
    if ready.try_exists()? {
        return read(&ready, period);
    }
    let random = Token::generate()?.to_secret_bytes();
    let seed = u64::from_le_bytes(random[..8].try_into().expect("32 random bytes"));
    let tower = generator.tower_with_progress(
        period,
        seed,
        candidates,
        Limits::default(),
        |floor, candidate, nodes| {
            eprintln!("Tower floor {floor}, candidate {candidate}: {nodes} certified encounters");
        },
    )?;
    let (manifest, nodes) = tower.publications()?;
    let evidence = root.join(uuid::Uuid::now_v7().to_string());
    private_new_directory(&evidence)?;
    for (node, certified) in tower.nodes.iter().enumerate() {
        let directory = evidence.join(format!("node-{node:02}"));
        private_new_directory(&directory)?;
        for (proof, witness) in certified.proofs.iter().enumerate() {
            private_write(
                &directory.join(format!("proof-{proof}.jsonl")),
                &witness.record,
            )?;
        }
        sync_directory(&directory)?;
    }
    let prepared = Prepared { manifest, nodes };
    prepared.validate(period)?;
    let bytes = serde_json::to_vec(&prepared)?;
    ensure!(
        bytes.len() <= MAX_BUNDLE_BYTES,
        "tower disk bundle exceeds bound"
    );
    let source = evidence.join("expedition.json");
    private_write(&source, &bytes)?;
    sync_directory(&evidence)?;
    match fs::hard_link(&source, &ready) {
        Ok(()) => {
            sync_directory(root)?;
            Ok(prepared)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => read(&ready, period),
        Err(error) => Err(error.into()),
    }
}

fn read(path: &Path, period: Period) -> Result<Prepared> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(u64::try_from(MAX_BUNDLE_BYTES)? + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_BUNDLE_BYTES,
        "tower disk bundle exceeds bound"
    );
    let prepared: Prepared =
        serde_json::from_slice(&bytes).context("invalid retained expedition")?;
    prepared.validate(period)?;
    Ok(prepared)
}
