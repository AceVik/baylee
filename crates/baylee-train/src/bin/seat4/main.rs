//! A local neural opponent in an ordinary gateway room.
mod play;
mod presence;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail, ensure};
use baylee_core::ids::PlayerId;
use baylee_seat::Disclosure;
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{Chair, GuestSignIn, Lobby, Session, seat_name};
use baylee_train::live4::LiveSeat4;
use baylee_train::netplay4::NetPlayer4;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Play a frozen v4 checkpoint through a gateway; observes every received view")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:28766")]
    gateway: String,
    /// An exported checkpoint directory; the running trainer never writes here.
    #[arg(long)]
    model: PathBuf,
    #[arg(long, conflicts_with = "acceptance")]
    deck: Option<PathBuf>,
    /// A built-in acceptance deck, used when --deck is absent.
    #[arg(long)]
    acceptance: Option<String>,
    /// The public opponent name receives the NET- prefix.
    #[arg(long, default_value = "v4")]
    name: String,
    #[arg(long, default_value = "Play against neural v4")]
    room_name: String,
    /// Join a waiting room instead of hosting a two-player room.
    #[arg(long)]
    join: Option<String>,
    /// Fixed inference depth; 1 is the benchmarked quick response setting.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=8))]
    thinking_steps: u8,
    /// Completed games before exiting; zero keeps hosting new rooms.
    #[arg(long, default_value_t = 1)]
    games: u32,
    /// Local aggregate reports, without player identities or hidden game records.
    #[arg(long)]
    reports: Option<PathBuf>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        args.join.is_none() || args.games == 1,
        "--join requires --games 1"
    );
    // This local preview must not silently compete with the CUDA trainer.
    ensure!(
        std::env::var("BAYLEE_EP").as_deref() != Ok("cuda"),
        "seat4 uses CPU inference; unset BAYLEE_EP or set it to cpu"
    );
    let deck = match &args.deck {
        Some(path) => Deck::from_file(path)?,
        None => Deck::acceptance(args.acceptance.as_deref().unwrap_or("Victory"))?,
    };
    let lobby = Lobby::new(&args.gateway);
    let name = seat_name(Disclosure::Net, &args.name)?;
    // Validate/load the model before opening a room. A failed load leaves no chair.
    let mut first_brain = Some(load_brain(&args)?);
    let session = lobby
        .guest(&GuestSignIn {
            display_name: name.clone(),
            invite_key: None,
        })
        .await?;
    let mut presence = presence::Presence::start(lobby.connect_lobby(&session).await?);
    let result = Box::pin(run(
        &args,
        &lobby,
        &session,
        &deck,
        &name,
        first_brain.take(),
        &mut presence,
    ))
    .await;
    presence.close().await;
    result
}

async fn run(
    args: &Args,
    lobby: &Lobby,
    session: &Session,
    deck: &Deck,
    name: &str,
    mut first_brain: Option<NetPlayer4>,
    presence: &mut presence::Presence,
) -> anyhow::Result<()> {
    let deck_id = lobby.upload(session, deck).await?;
    let mut games = 0_u32;
    loop {
        presence.check()?;
        let brain = match first_brain.take() {
            Some(brain) => brain,
            None => load_brain(args)?,
        };
        let chair = if let Some(game_id) = &args.join {
            lobby.join(session, game_id, &deck_id, None, None).await?
        } else {
            lobby.open(session, &deck_id, 2, &args.room_name).await?
        };
        println!(
            "{}",
            serde_json::json!({
                "phase": "waiting-for-player", "name": name, "game": chair.game_id,
                "gateway": lobby.base(), "deck": deck.name, "device": "cpu",
                "model": args.model, "thinking_steps": args.thinking_steps,
            })
        );
        // Keep the adapter outside the cancellable future so both errors and
        // Ctrl-C retain the counters accumulated before the failure.
        let seat = PlayerId::new(u8::try_from(chair.seat).context("invalid seat")?);
        let mut core = LiveSeat4::new(brain, deck.list.clone(), seat);
        let game = async {
            wait_for_player(lobby, session, &chair, args.join.is_none()).await?;
            let mut link = SeatLink::new(lobby.clone(), chair.clone(), Some(session.clone()));
            play::play(&mut link, &mut core).await
        };
        let result = tokio::select! {
            result = game => result,
            result = presence.lost() => result,
            signal = tokio::signal::ctrl_c() => {
                match signal {
                    Ok(()) => Err(anyhow::anyhow!("seat stopped by operator")),
                    Err(_) => Err(anyhow::anyhow!("operator signal handler failed")),
                }
            }
        };
        let report = play::report_after(core.report(), result.is_err());
        let reporting = (|| -> anyhow::Result<()> {
            if let Some(dir) = &args.reports {
                std::fs::create_dir_all(dir)?;
                let path = dir.join(format!("{}.json", uuid::Uuid::now_v7()));
                std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
            }
            Ok(())
        })();
        println!(
            "{}",
            serde_json::json!({
                "phase": if result.is_ok() { "game-complete" } else { "game-failed" },
                "report": report,
            })
        );
        // A failed inference or interrupted connection must not leave a claimed
        // neural chair silently being played by a stand-in.
        if let Ok(Some(room)) = lobby.room(session, &chair.game_id).await
            && room.waiting()
            && lobby.leave(session, &chair.game_id).await.is_err()
        {
            eprintln!("could not release the waiting-room chair cleanly");
        }
        if reporting.is_err() {
            eprintln!("could not write the local aggregate report");
        }
        result?;
        reporting?;
        games += 1;
        if args.games != 0 && games >= args.games {
            return Ok(());
        }
    }
}

fn load_brain(args: &Args) -> anyhow::Result<NetPlayer4> {
    Ok(NetPlayer4::load(&args.model)?
        .with_temperature(0.0)
        .with_thinking_steps(args.thinking_steps))
}

async fn wait_for_player(
    lobby: &Lobby,
    session: &Session,
    chair: &Chair,
    host: bool,
) -> anyhow::Result<()> {
    loop {
        let room = lobby
            .room(session, &chair.game_id)
            .await?
            .context("room no longer exists")?;
        ensure!(
            room.seats.len() == 2 && room.seats.iter().all(|seat| seat.kind == "human"),
            "this preview requires two socket seats and no house-AI chairs"
        );
        if room.playing() {
            return Ok(());
        }
        if !room.waiting() {
            bail!("room ended before play");
        }
        if room.seats.iter().any(|seat| seat.you && !seat.ready) {
            lobby.ready(session, &chair.game_id).await?;
        }
        if host
            && room
                .seats
                .iter()
                .all(|seat| seat.taken && (seat.ready || seat.you))
        {
            lobby.start(session, &chair.game_id).await?;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
