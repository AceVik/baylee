use super::*;
use std::io::Write;
fn fixture(root: &Path) -> Result<()> {
    let meta = json!({"encoder_version":3,"ent_cols":["a","b"],"seat_cols":["a","b"],"glob_cols":["turn"],"meta_cols":["game","seat"],"deck_cols":["n"],"max_seats":2,"shards":[{"dir":"a","samples":1,"entities":2,"decks":1,"opts":2},{"dir":"b","samples":1,"entities":1,"decks":2,"opts":1}]});
    files::atomic_json(&root.join("dataset.json"), &meta)?;
    let columns = columns(&meta)?;
    for (i, value) in meta["shards"]
        .as_array()
        .context("shards")?
        .iter()
        .enumerate()
    {
        let directory = root.join(value["dir"].as_str().context("dir")?);
        fs::create_dir(&directory)?;
        let shard = Shard {
            directory: directory.clone(),
            samples: 1,
            entities: count(value, "entities")?,
            decks: count(value, "decks")?,
            opts: count(value, "opts")?,
        };
        for column in &columns {
            let mut file = fs::File::create(directory.join(column.name))?;
            let terminal = match column.kind {
                Kind::OffsetEntities => Some(shard.entities),
                Kind::OffsetDecks => Some(shard.decks),
                Kind::OffsetOpts => Some(shard.opts),
                _ => None,
            };
            if let Some(terminal) = terminal {
                file.write_all(&0i64.to_le_bytes())?;
                file.write_all(&i64::try_from(terminal)?.to_le_bytes())?;
            } else {
                file.write_all(&vec![
                    u8::try_from(i + 1)?;
                    usize::try_from(stream::source_bytes(column, &shard)?)?
                ])?;
            }
        }
    }
    Ok(())
}
#[test]
fn streaming_rebase_reuse_and_content_seals() -> Result<()> {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    fs::create_dir(root.path().join("merged"))?;
    fs::write(root.path().join("merged/legacy-proof"), "preserved")?;
    let first = merge3(root.path())?;
    ensure!(first["reused"] == false);
    let cache = Path::new(first["cache_dir"].as_str().context("cache path")?);
    let bytes = fs::read(cache.join("ent_off.i64"))?;
    let offsets: Vec<_> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|s| i64::from_le_bytes(*s))
        .collect();
    ensure!(offsets == [0, 2, 3]);
    ensure!(fs::read(cache.join("ent_card.i32"))? == [vec![1; 8], vec![2; 4]].concat());
    ensure!(merge3(root.path())?["reused"] == true);
    fs::write(cache.join("ent_card.i32"), "corrupted")?;
    ensure!(merge3(root.path())?["reused"] == false);
    ensure!(fs::read_to_string(root.path().join("merged/legacy-proof"))? == "preserved");
    Ok(())
}
#[test]
fn malformed_offsets_lengths_and_source_changes_fail_closed() -> Result<()> {
    for problem in 0..3 {
        let root = tempfile::tempdir()?;
        fixture(root.path())?;
        match problem {
            0 => fs::write(
                root.path().join("a/ent_off.i64"),
                [0i64.to_le_bytes(), 3i64.to_le_bytes()].concat(),
            )?,
            1 => fs::write(root.path().join("a/ent_card.i32"), [0u8; 4])?,
            _ => {
                let mut meta = files::read_json(&root.path().join("dataset.json"))?;
                meta["shards"][0]["dir"] = json!("../escape");
                files::atomic_json(&root.path().join("dataset.json"), &meta)?;
            }
        }
        ensure!(merge3(root.path()).is_err());
    }
    Ok(())
}
