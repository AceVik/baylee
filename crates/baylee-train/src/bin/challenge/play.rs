//! Blind local difficulty check. Only the ordinary human seat's view leaves the session.
use anyhow::{Context, Result, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_train::challenges::{Generator, Kind, Limits};
use std::io::{self, BufRead as _, Write as _};

#[derive(clap::Args)]
pub(super) struct Args {
    #[arg(long, default_value_os_t = baylee_train::working::repo_root())]
    source_dir: std::path::PathBuf,
    #[arg(long, value_parser = ["daily", "weekly"])]
    kind: String,
}

pub(super) fn run(args: &Args) -> Result<()> {
    let generator = Generator::from_source(&args.source_dir)?;
    let kind = if args.kind == "daily" {
        Kind::Daily
    } else {
        Kind::Weekly
    };
    let mut selected = None;
    for attempt in 1..=32 {
        let random = baylee_account::Token::generate()?.to_secret_bytes();
        let seed = u64::from_le_bytes(random[..8].try_into().expect("32 random bytes"));
        if let Ok(candidate) = generator.generate(kind, seed, Limits::default()) {
            selected = Some(candidate);
            break;
        }
        eprintln!("Candidate {attempt} rejected; private seed and proofs withheld");
    }
    let candidate = selected.context("no certified candidate in blind-play budget")?;
    // Deck text is public, like the client's supplied-deck panel. Neither the
    // opponent's hidden cards, the seed nor witness moves are printed.
    println!("Supplied deck:\n{}", candidate.deck);
    let mut session =
        baylee_gamehost::Session::new_recorded(&candidate.package.preset, baylee_build::short())
            .context("blind session")?;
    drop(candidate);
    eprintln!(
        "Local practice only, no ranking. Enter a PlayerAction JSON value; empty line passes priority; quit ends the check."
    );
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let seat = PlayerId::new(0);
    loop {
        session.pump_at_most(128);
        session.discard_frames();
        let _ = session.take_record();
        if let Pending::GameOver(result) = session.pending() {
            println!(
                "{}",
                serde_json::json!({"finished": true, "won": session.winning_seats(*result) == [seat], "turn": session.state().turn.number})
            );
            return Ok(());
        }
        ensure!(!session.needs_pump(), "opponent exceeded bounded pump");
        let (pending, view) = session.view_for(seat).context("no human decision")?;
        println!("{}", serde_json::json!({"pending": pending, "view": view}));
        io::stdout().flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 || line.trim() == "quit" {
            return Ok(());
        }
        let action = if line.trim().is_empty() {
            Ok(PlayerAction::PassPriority)
        } else {
            serde_json::from_str::<PlayerAction>(line.trim())
        };
        match action {
            Ok(action) => {
                if let Err(error) = session.act_slice(seat, action, 0) {
                    eprintln!("Refused: {error}");
                }
            }
            Err(error) => eprintln!("Invalid action: {error}"),
        }
    }
}
