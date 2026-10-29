use crate::{files, trajectory::cache};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};
pub(super) fn merge(
    parts: &[(PathBuf, PathBuf, PathBuf)],
    output: &Path,
    shards: &[String],
    frames: u64,
) -> Result<()> {
    let identity = cache::fingerprint(output, "train", shards)?;
    let directory = output.join(".trajectory-cache-v2");
    let stem = format!("train-{}", &identity[..24]);
    let data_path = directory.join(format!("{stem}.bin"));
    let index_path = directory.join(format!("{stem}.idx"));
    let mut data = File::create(&data_path)?;
    let mut index = File::create(&index_path)?;
    let mut offset = 0_u64;
    for (_, part_data, part_index) in parts {
        let size = part_index.metadata()?.len();
        ensure!(size.is_multiple_of(32), "truncated recovered index");
        let mut reader = BufReader::new(File::open(part_index)?);
        for _ in 0..size / 32 {
            let mut row = [0; 32];
            reader.read_exact(&mut row)?;
            let local = u64::from_le_bytes(row[16..24].try_into()?);
            row[16..24].copy_from_slice(
                &local
                    .checked_add(offset)
                    .context("cache offset overflow")?
                    .to_le_bytes(),
            );
            index.write_all(&row)?;
        }
        offset = offset
            .checked_add(std::io::copy(&mut File::open(part_data)?, &mut data)?)
            .context("cache data overflow")?;
        fs::remove_file(part_data)?;
        fs::remove_file(part_index)?;
    }
    data.sync_all()?;
    index.sync_all()?;
    files::atomic_json(
        &directory.join(format!("{stem}.json")),
        &json!({"fingerprint":identity,"records":frames}),
    )
}
