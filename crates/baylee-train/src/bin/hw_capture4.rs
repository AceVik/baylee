//! `hw_capture4` — the rows a net is really asked, written to a row dump for
//! `hw_bench` (`baylee_driver::rowdump`).
//!
//! It replays recorded games (`games.jsonl` plus `records/`) with the net
//! sitting in every chair: before each recorded answer the asked seat
//! observes its own view and the net decides (its answer is thrown away; the
//! record's is applied), and every graph call this makes is written: the
//! observation updates (`K = 0`) and the decisions (`K > 0`), with the
//! recurrent memory the FP32 net carried to that point. Each seat sees only
//! its own view, so no hidden card reaches a row.
//!
//! ```text
//! hw_capture4 --model r1-4eb --records DIR [--records DIR ...] \
//!     --out rows.bin --max-decisions 12000 [--shard 0/4]
//! ```

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail, ensure};
use baylee_core::ids::PlayerId;
use baylee_gamehost::Session;
use baylee_gamehost::record::Line;
use baylee_train::netplay4::{CatalogPolicy4, NetPlayer4};
use baylee_train::netplay4::capture::RowCapture4;
use clap::Parser;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::json;

#[derive(Parser)]
struct Args {
    /// Exported model directory (`model.onnx.json`, `model.k*.onnx`), FP32.
    #[arg(long)]
    model: PathBuf,
    /// Run directories (`games.jsonl` and `records/`); repeat for several.
    /// Games are taken from them in turn.
    #[arg(long, required = true)]
    records: Vec<PathBuf>,
    /// The row dump to write.
    #[arg(long)]
    out: PathBuf,
    /// Stop after this many decision rows.
    #[arg(long, default_value_t = 12_000)]
    max_decisions: u64,
    /// `i/n`: only every `n`-th game, starting at `i`, so several processes
    /// can share the work.
    #[arg(long, default_value = "0/1")]
    shard: String,
    /// How the model's card catalog must relate to this build's: `exact`,
    /// `superset` or `drift` (`docs/trained-ai.md` §"Card catalog
    /// compatibility").
    #[arg(long, default_value = "exact")]
    catalog: String,
}

#[derive(Deserialize)]
struct Entry {
    i: u64,
    shard: String,
    offset: u64,
    len: u64,
}

fn entries(dir: &Path) -> anyhow::Result<Vec<Entry>> {
    let file = File::open(dir.join("games.jsonl"))
        .with_context(|| format!("{}/games.jsonl", dir.display()))?;
    BufReader::new(file)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect()
}

/// One game's record lines, read from its gzip member.
fn record(dir: &Path, entry: &Entry) -> anyhow::Result<Vec<Line>> {
    ensure!(
        Path::new(&entry.shard).components().count() == 1,
        "shard must be a file name"
    );
    ensure!(entry.len <= 128 * 1024 * 1024, "record over 128 MiB");
    let mut file = File::open(dir.join("records").join(&entry.shard))?;
    file.seek(SeekFrom::Start(entry.offset))?;
    let mut member = vec![0_u8; usize::try_from(entry.len)?];
    file.read_exact(&mut member)?;
    let mut bytes = Vec::new();
    GzDecoder::new(member.as_slice())
        .take(512 * 1024 * 1024)
        .read_to_end(&mut bytes)?;
    bytes
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| Ok(serde_json::from_slice(line)?))
        .collect()
}

#[derive(Default)]
struct Tally {
    games: u64,
    complete: u64,
    cut: u64,
    unbuildable: u64,
}

/// Replays one game with `net` in every chair; `Ok(true)` when the record
/// played to its end.
fn replay(net: &mut NetPlayer4, game: u32, lines: &[Line], max: u64) -> anyhow::Result<bool> {
    let Some(Line::Header { preset, .. }) = lines.first() else {
        bail!("a record starts with its header");
    };
    let mut session = Session::new(preset).context("the record's preset builds no game")?;
    for seat in 0..preset.seats.len() {
        session.take_over(PlayerId::new(seat as u8));
    }
    net.init_game(preset);
    let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
    for line in &lines[1..] {
        let Line::Input { seat, action, .. } = line else {
            continue;
        };
        let player = PlayerId::new(*seat);
        if session.pending().asked() == Some(player)
            && let Some((pending, view)) = session.view_for_with_context(player)
        {
            if let Some(capture) = net.capture_mut() {
                capture.set_context(game, *seat);
            }
            net.observe(&view)?;
            let _answer = net.act(&view, &pending, &teams);
        }
        if session.act(player, action.clone()).is_err() {
            return Ok(false);
        }
        if net.capture_mut().is_some_and(|c| c.decisions() >= max) {
            return Ok(true);
        }
    }
    Ok(true)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (shard_i, shard_n) = args
        .shard
        .split_once('/')
        .and_then(|(i, n)| Some((i.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))
        .filter(|&(i, n)| n > 0 && i < n)
        .context("--shard is i/n with i < n")?;
    let policy = match args.catalog.as_str() {
        "exact" => CatalogPolicy4::Exact,
        "superset" => CatalogPolicy4::Superset,
        "drift" => CatalogPolicy4::Drift,
        other => bail!("unknown catalog policy {other}"),
    };
    let mut net = NetPlayer4::load_with(&args.model, policy)?;
    let lists = args
        .records
        .iter()
        .map(|dir| entries(dir))
        .collect::<anyhow::Result<Vec<_>>>()?;
    // Games in turn from each run, so every run is represented.
    let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
    let mut order = Vec::new();
    for g in 0..longest {
        for (d, list) in lists.iter().enumerate() {
            if g < list.len() {
                order.push((d, g));
            }
        }
    }
    let header = json!({
        "model": args.model,
        "records": args.records,
        "shard": args.shard,
        "max_decisions": args.max_decisions,
        "catalog": args.catalog,
        "tool": "hw_capture4",
        "build": baylee_build::short(),
    });
    net.set_capture(Some(RowCapture4::create(&args.out, &header)?));
    let mut tally = Tally::default();
    for (n, &(d, g)) in order.iter().enumerate() {
        if n % shard_n != shard_i {
            continue;
        }
        let entry = &lists[d][g];
        let game = u32::try_from(d * 100_000)? + u32::try_from(entry.i)?;
        let lines = record(&args.records[d], entry)?;
        tally.games += 1;
        match replay(&mut net, game, &lines, args.max_decisions) {
            Ok(true) => tally.complete += 1,
            Ok(false) => tally.cut += 1,
            Err(err) => {
                tally.unbuildable += 1;
                eprintln!("game {game}: {err:#}");
            }
        }
        let capture = net.capture_mut().context("capture set")?;
        if capture.decisions() >= args.max_decisions {
            break;
        }
    }
    let capture = net.set_capture(None).context("capture set")?;
    println!(
        "{}",
        json!({
            "out": args.out,
            "games": tally.games,
            "complete": tally.complete,
            "cut_by_divergence": tally.cut,
            "failed": tally.unbuildable,
            "decision_rows": capture.decisions(),
            "observation_rows": capture.observations(),
        })
    );
    capture.finish()
}
