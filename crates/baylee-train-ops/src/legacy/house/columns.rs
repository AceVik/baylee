use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

pub(super) struct Columns {
    pub rows: u64,
    pub entities: u64,
    pub options: u64,
    pub width: usize,
    pub info: BufReader<File>,
    pub ent_offset: BufReader<File>,
    pub opt_offset: BufReader<File>,
    pub ent_card: File,
    pub option: File,
}
fn file(root: &Path, name: &str, count: u64, bytes: u64) -> Result<File> {
    let path = root.join(name);
    let file = File::open(&path)?;
    ensure!(
        file.metadata()?.len() == count.checked_mul(bytes).context("column size overflow")?,
        "legacy column size mismatch: {}",
        path.display()
    );
    Ok(file)
}
pub(super) fn i64(reader: &mut impl Read) -> Result<u64> {
    let mut raw = [0; 8];
    reader.read_exact(&mut raw)?;
    Ok(u64::try_from(i64::from_le_bytes(raw))?)
}
impl Columns {
    pub fn open(root: &Path, shard: &Value, width: usize) -> Result<Self> {
        let name = shard["dir"].as_str().context("legacy shard directory")?;
        let path = root.join(name);
        ensure!(
            path.canonicalize()?.starts_with(root.canonicalize()?),
            "legacy shard escapes dataset"
        );
        let rows = shard["samples"].as_u64().context("legacy shard samples")?;
        let entities = shard["entities"]
            .as_u64()
            .context("legacy shard entities")?;
        let options = shard["opts"].as_u64().context("legacy shard options")?;
        let offsets = rows.checked_add(1).context("offset count overflow")?;
        Ok(Self {
            rows,
            entities,
            options,
            width,
            info: BufReader::new(file(
                &path,
                "meta.i32",
                rows,
                u64::try_from(width)?
                    .checked_mul(4)
                    .context("metadata width overflow")?,
            )?),
            ent_offset: BufReader::new(file(&path, "ent_off.i64", offsets, 8)?),
            opt_offset: BufReader::new(file(&path, "opt_off.i64", offsets, 8)?),
            ent_card: file(&path, "ent_card.i32", entities, 4)?,
            option: file(&path, "opt.i16", options, 6)?,
        })
    }
    pub fn metadata(&mut self) -> Result<Vec<i32>> {
        let mut values = Vec::with_capacity(self.width);
        for _ in 0..self.width {
            let mut raw = [0; 4];
            self.info.read_exact(&mut raw)?;
            values.push(i32::from_le_bytes(raw));
        }
        Ok(values)
    }
    pub fn cards(&mut self, start: u64, end: u64) -> Result<Vec<u32>> {
        self.ent_card.seek(SeekFrom::Start(
            start
                .checked_mul(4)
                .context("entity byte offset overflow")?,
        ))?;
        let mut cards = Vec::new();
        for _ in start..end {
            let mut raw = [0; 4];
            self.ent_card.read_exact(&mut raw)?;
            cards.push(u32::try_from(i32::from_le_bytes(raw))?);
        }
        Ok(cards)
    }
    pub fn options(&mut self, start: u64, end: u64) -> Result<Vec<[i16; 3]>> {
        self.option.seek(SeekFrom::Start(
            start
                .checked_mul(6)
                .context("option byte offset overflow")?,
        ))?;
        let mut options = Vec::new();
        for _ in start..end {
            let mut raw = [0; 6];
            self.option.read_exact(&mut raw)?;
            options.push([
                i16::from_le_bytes(raw[..2].try_into()?),
                i16::from_le_bytes(raw[2..4].try_into()?),
                i16::from_le_bytes(raw[4..6].try_into()?),
            ]);
        }
        Ok(options)
    }
}
