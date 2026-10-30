use super::{Column, Kind, Shard};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};
fn entries(column: &Column, shard: &Shard) -> Result<u64> {
    Ok(match column.kind {
        Kind::Samples => shard.samples,
        Kind::Entities => shard.entities,
        Kind::Decks => shard.decks,
        Kind::Opts => shard.opts,
        Kind::OffsetEntities | Kind::OffsetDecks | Kind::OffsetOpts => shard
            .samples
            .checked_add(1)
            .context("offset count overflow")?,
    })
}
pub(super) fn source_bytes(column: &Column, shard: &Shard) -> Result<u64> {
    entries(column, shard)?
        .checked_mul(column.unit)
        .context("v3 byte count overflow")
}
fn offsets(
    input: &Path,
    out: &mut BufWriter<File>,
    samples: u64,
    payload: u64,
    base: u64,
) -> Result<()> {
    let mut reader = BufReader::new(File::open(input)?);
    let mut previous = 0;
    for row in 0..=samples {
        let mut bytes = [0; 8];
        reader.read_exact(&mut bytes)?;
        let value = i64::from_le_bytes(bytes);
        let value = u64::try_from(value).context("negative v3 offset")?;
        ensure!(
            value >= previous && value <= payload && (row != 0 || value == 0),
            "invalid v3 offset sequence"
        );
        if row == samples {
            ensure!(value == payload, "v3 terminal offset differs from payload");
        } else {
            out.write_all(
                &i64::try_from(base.checked_add(value).context("rebased offset overflow")?)?
                    .to_le_bytes(),
            )?;
        }
        previous = value;
    }
    Ok(())
}
pub(super) fn write(path: &Path, shards: &[Shard], columns: &[Column]) -> Result<Value> {
    let mut outputs: BTreeMap<_, _> = columns
        .iter()
        .map(|column| {
            Ok((
                column.name,
                BufWriter::new(File::create(path.join(column.name))?),
            ))
        })
        .collect::<Result<_>>()?;
    let (mut samples, mut entities, mut decks, mut opts) = (0u64, 0u64, 0u64, 0u64);
    for shard in shards {
        for column in columns {
            let output = outputs.get_mut(column.name).context("merge output")?;
            match column.kind {
                Kind::OffsetEntities => offsets(
                    &shard.directory.join(column.name),
                    output,
                    shard.samples,
                    shard.entities,
                    entities,
                )?,
                Kind::OffsetDecks => offsets(
                    &shard.directory.join(column.name),
                    output,
                    shard.samples,
                    shard.decks,
                    decks,
                )?,
                Kind::OffsetOpts => offsets(
                    &shard.directory.join(column.name),
                    output,
                    shard.samples,
                    shard.opts,
                    opts,
                )?,
                _ => {
                    let copied =
                        std::io::copy(&mut File::open(shard.directory.join(column.name))?, output)?;
                    ensure!(
                        copied == source_bytes(column, shard)?,
                        "v3 column changed while copying"
                    );
                }
            }
        }
        samples = samples
            .checked_add(shard.samples)
            .context("sample total overflow")?;
        entities = entities
            .checked_add(shard.entities)
            .context("entity total overflow")?;
        decks = decks
            .checked_add(shard.decks)
            .context("deck total overflow")?;
        opts = opts
            .checked_add(shard.opts)
            .context("option total overflow")?;
    }
    for (name, total) in [
        ("ent_off.i64", entities),
        ("deck_off.i64", decks),
        ("opt_off.i64", opts),
    ] {
        outputs
            .get_mut(name)
            .context("offset output")?
            .write_all(&i64::try_from(total)?.to_le_bytes())?;
    }
    for output in outputs.values_mut() {
        output.flush()?;
        output.get_ref().sync_all()?;
    }
    Ok(json!({"samples":samples,"entities":entities,"decks":decks,"opts":opts}))
}
