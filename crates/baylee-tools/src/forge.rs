//! Import owner-provided Adventure saves and collection tracker lists.
mod java;
mod lists;
mod resolve;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use serde::Serialize;

const INPUT_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "forge")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Inspect a zlib-compressed owner-provided Adventure save.
    #[command(alias = "read_adventure_save")]
    ReadAdventureSave {
        save: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Inspect collection tracker deck rows.
    #[command(alias = "parse_friend_list")]
    ParseFriendList { list: PathBuf },
    /// Resolve rows against the local Docker Compose catalogue.
    #[command(alias = "resolve_printings")]
    ResolvePrintings {
        list: PathBuf,
        #[arg(long, default_value = "en")]
        lang: String,
        #[arg(long)]
        oldest: bool,
        #[arg(long)]
        nonfoil: bool,
        #[arg(long)]
        leave_open: bool,
        #[arg(long)]
        json: bool,
    },
    /// Write selected named decks using externally read edition codes.
    #[command(alias = "to_deck_rows")]
    ToDeckRows {
        save: PathBuf,
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Serialize)]
struct Deck {
    slot: usize,
    name: String,
    main: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    ensure!(file.metadata()?.is_file(), "input must be a regular file");
    let mut bytes = Vec::new();
    file.take(INPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= INPUT_LIMIT,
        "input exceeds 64 MiB limit"
    );
    Ok(bytes)
}

fn decks(save: &Path) -> Result<Vec<Deck>> {
    let compressed = read(save)?;
    let mut raw = Vec::new();
    ZlibDecoder::new(compressed.as_slice())
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut raw)
        .context("invalid zlib save")?;
    ensure!(
        raw.len() as u64 <= INPUT_LIMIT,
        "decompressed save exceeds 64 MiB"
    );
    let entries = java::entries(&raw)?;
    let mut decks = Vec::new();
    for slot in 0..100 {
        let Some(blob) = entries.get(&format!("deck_name_{slot}")) else {
            continue;
        };
        let section = |prefix: &str| -> Result<Vec<String>> {
            entries
                .get(&format!("{prefix}_{slot}"))
                .map_or_else(|| Ok(Vec::new()), |blob| java::string_array(blob))
        };
        decks.push(Deck {
            slot,
            name: java::utf_value(blob)?,
            main: section("deck")?,
            sideboard: section("sideBoardCards")?,
            commanders: section("commanderCards")?,
        });
    }
    ensure!(
        !decks.is_empty(),
        "no deck slot found; is this an Adventure save?"
    );
    Ok(decks)
}

fn total(rows: &[String]) -> Result<u64> {
    rows.iter().try_fold(0_u64, |sum, row| {
        Ok(sum + u64::from(lists::forge_row(row)?.count))
    })
}

fn codes(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut paths = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "txt"));
    ensure!(
        paths.len() >= 100,
        "editions directory holds fewer than 100 edition files; set BAYLEE_FORGE_EDITIONS"
    );
    paths.sort();
    let mut codes = BTreeMap::new();
    for path in paths {
        let bytes = read(&path)?;
        let text = std::str::from_utf8(&bytes).context("edition is not UTF-8")?;
        let meta: BTreeMap<_, _> = text
            .lines()
            .take_while(|line| !line.starts_with("[cards]"))
            .filter_map(|line| line.trim().split_once('='))
            .collect();
        if let Some(code) = meta.get("Code") {
            let mapped = meta
                .get("ScryfallCode")
                .filter(|value| !value.is_empty())
                .unwrap_or(code);
            ensure!(
                !code
                    .chars()
                    .chain(mapped.chars())
                    .any(|c| c.is_control() || "()[]/\\".contains(c)),
                "invalid edition code"
            );
            if let Some(previous) = codes.insert((*code).to_owned(), (*mapped).to_owned()) {
                ensure!(previous == *mapped, "conflicting edition code");
            }
        }
    }
    Ok(codes)
}

fn render(deck: &Deck, codes: &BTreeMap<String, String>) -> Result<String> {
    ensure!(
        !deck
            .name
            .chars()
            .any(|c| c.is_control() || "[]".contains(c)),
        "unsafe deck section name"
    );
    let mut lines = vec![format!("[deck:{}]", deck.name)];
    for (heading, rows) in [
        (None, &deck.main),
        (Some("sideboard"), &deck.sideboard),
        (Some("commander"), &deck.commanders),
    ] {
        if rows.is_empty() {
            continue;
        }
        if let Some(heading) = heading {
            lines.extend([String::new(), format!("[{heading}]")]);
        }
        for row in rows {
            lines.push(lists::deck_row(row, codes)?);
        }
    }
    Ok(lines.join("\n") + "\n")
}

fn write_decks(
    decks: &[Deck],
    names: &[String],
    output: &Path,
    codes: &BTreeMap<String, String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut pending = Vec::new();
    for name in names {
        let matches: Vec<_> = decks.iter().filter(|deck| &deck.name == name).collect();
        ensure!(
            matches.len() == 1,
            "requested deck is missing or its name is ambiguous"
        );
        let slug = lists::slug(name)?;
        ensure!(seen.insert(slug.clone()), "deck filenames collide");
        pending.push((
            slug,
            render(matches[0], codes)?,
            total(&matches[0].main)?,
            matches[0].sideboard.len(),
        ));
    }
    fs::create_dir_all(output)?;
    let output = output.canonicalize()?;
    let lock = output.join(".baylee-forge.lock");
    let _lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .context("deck output is locked; remove stale lock only after checking its owner")?;
    let result: Result<()> = (|| {
        for (slug, text, count, sideboard) in pending {
            let path = output.join(format!("{slug}.txt"));
            if let Ok(meta) = fs::symlink_metadata(&path) {
                ensure!(
                    meta.is_file() && !meta.file_type().is_symlink(),
                    "refusing non-regular deck output"
                );
            }
            let mut temp = tempfile::NamedTempFile::new_in(&output)?;
            temp.write_all(text.as_bytes())?;
            temp.as_file().sync_all()?;
            temp.persist(&path).map_err(|error| error.error)?;
            ensure!(
                read(&path)? == text.as_bytes(),
                "deck output verification failed"
            );
            println!(
                "{}: {count} main, {sideboard} sideboard rows",
                path.display()
            );
        }
        Ok(())
    })();
    let cleanup = fs::remove_file(lock);
    result?;
    cleanup?;
    Ok(())
}

/// Run an import subcommand. Inputs remain external; no corpus file is copied.
///
/// # Errors
/// Rejects malformed input, unsafe output names, failed catalogue queries and I/O errors.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let cli = Cli::try_parse_from(std::iter::once("forge".to_owned()).chain(args.iter().cloned()))?;
    match cli.command {
        Action::ReadAdventureSave { save, json } => {
            let found = decks(&save)?;
            if json {
                let rows = found.iter().map(|deck| -> Result<_> {
                    let section = |rows: &[String]| rows.iter().map(|row| -> Result<_> {
    let row = lists::forge_row(row)?;
    Ok(serde_json::json!({"count":row.count,"name":row.name,"foil":row.foil,"set":row.set,"number":row.number,"flags":row.flags}))
}).collect::<Result<Vec<_>>>();
                    Ok(serde_json::json!({"slot":deck.slot,"name":deck.name,"main":section(&deck.main)?,"sideboard":section(&deck.sideboard)?,"commanders":section(&deck.commanders)?}))
                }).collect::<Result<Vec<_>>>()?;
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                for deck in found {
                    let count = total(&deck.main)?;
                    if count != 0 || !deck.commanders.is_empty() {
                        println!(
                            "slot {:2}  {:?}: {count} main / {} sideboard / {} commander",
                            deck.slot,
                            deck.name,
                            total(&deck.sideboard)?,
                            deck.commanders.len()
                        );
                    }
                }
            }
        }
        Action::ParseFriendList { list } => {
            let rows = lists::friend_list(std::str::from_utf8(&read(&list)?)?)?;
            println!(
                "{} rows, {} cards, {} name a set, {} a number, {} foil",
                rows.len(),
                rows.iter().map(|row| u64::from(row.count)).sum::<u64>(),
                rows.iter().filter(|row| row.set.is_some()).count(),
                rows.iter().filter(|row| row.number.is_some()).count(),
                rows.iter().filter(|row| row.foil).count()
            );
        }
        Action::ResolvePrintings {
            list,
            lang,
            oldest,
            nonfoil,
            leave_open,
            json,
        } => {
            let rows = lists::friend_list(std::str::from_utf8(&read(&list)?)?)?;
            let rows = resolve::resolve(root, rows, &lang, oldest, nonfoil, leave_open)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                for row in rows {
                    println!("{}", resolve::deck_row(&row));
                }
            }
        }
        Action::ToDeckRows { save, names, out } => {
            let editions = std::env::var_os("BAYLEE_FORGE_EDITIONS").map_or_else(
                || PathBuf::from("/Users/viktor/Projects/forge/forge-gui/res/editions"),
                PathBuf::from,
            );
            write_decks(&decks(&save)?, &names, &out, &codes(&editions)?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writer_round_trip_and_collision_fences() {
        let temp = tempfile::tempdir().unwrap();
        let deck = Deck {
            slot: 0,
            name: "Test Deck".into(),
            main: vec!["2 Oak|OLD|[1]".into()],
            sideboard: vec!["1 Elm+|NEW|[2]".into()],
            commanders: vec![],
        };
        let codes = BTreeMap::from([("OLD".into(), "NEW".into())]);
        write_decks(&[deck], &["Test Deck".into()], temp.path(), &codes).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("test-deck.txt")).unwrap(),
            "[deck:Test Deck]\n2 Oak (NEW) 1\n\n[sideboard]\n1 Elm (NEW) 2 *F*\n"
        );
        assert!(write_decks(&[], &["missing".into()], temp.path(), &codes).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn refuses_symlink_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let external = temp.path().join("external");
        fs::write(&external, "unchanged").unwrap();
        std::os::unix::fs::symlink(&external, temp.path().join("deck.txt")).unwrap();
        let deck = Deck {
            slot: 0,
            name: "Deck".into(),
            main: vec![],
            sideboard: vec![],
            commanders: vec![],
        };
        assert!(write_decks(&[deck], &["Deck".into()], temp.path(), &BTreeMap::new()).is_err());
        assert_eq!(fs::read_to_string(external).unwrap(), "unchanged");
    }
}
