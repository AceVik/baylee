//! A local neural opponent in an ordinary gateway room.
mod audit;
mod model_selection;
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
use baylee_train::live4::{LiveReport4, LiveSeat4};
use baylee_train::netplay4::{NetPlayer4, PolicyMode4, valid_raw_temperature};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Play a frozen v4/v5 checkpoint through a gateway; observes every received view")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:28766")]
    gateway: String,
    /// An exported checkpoint directory; the running trainer never writes here.
    #[arg(long)]
    model: PathBuf,
    /// Use the evaluated unmodified neural policy instead of tactical overrides.
    #[arg(long)]
    raw_policy: bool,
    /// Raw sampler temperature: zero is greedy; stochastic sampling requires >0.35.
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    /// Independent policy RNG seed; never a deal/shuffle seed.
    #[arg(long)]
    policy_seed: Option<u64>,
    /// Local hash/probability audits; not game records or admitted training data.
    #[arg(long)]
    decision_audits: Option<PathBuf>,
    /// Supervisor selection checked only before the first view of each new game.
    #[arg(long)]
    model_selection: Option<PathBuf>,
    #[arg(long, conflicts_with = "acceptance")]
    deck: Option<PathBuf>,
    /// A built-in acceptance deck, used when --deck is absent.
    #[arg(long)]
    acceptance: Option<String>,
    /// The public opponent name receives the NET- prefix.
    #[arg(long, default_value = baylee_train::BAYLEE_KI_NAME)]
    name: String,
    #[arg(long, default_value = "Play against BayleeKI")]
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
    validate_sampling(&args)?;
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
    let mut selected = model_selection::Selected::fixed(&args.model)?;
    // Bind the initial fallback to its benchmark when the supervisor names the
    // same artifact. A different candidate is considered only after room start.
    if let Some(path) = &args.model_selection
        && let Ok(initial) = model_selection::proposal(path, &selected, args.thinking_steps)
        && initial.model == selected.model
        && initial.model_sha256 == selected.model_sha256
    {
        selected = initial;
    }
    let mut first_brain = Some((load_brain(&args, &selected.model)?, selected));
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
    mut first_brain: Option<(NetPlayer4, model_selection::Selected)>,
    presence: &mut presence::Presence,
) -> anyhow::Result<()> {
    let (initial_brain, mut selected) = first_brain.take().context("missing initial model")?;
    let mut first_brain = Some(initial_brain);
    let deck_id = lobby.upload(session, deck).await?;
    let mut games = 0_u32;
    loop {
        presence.check()?;
        let chair = if let Some(game_id) = &args.join {
            lobby.join(session, game_id, &deck_id, None, None).await?
        } else {
            lobby.open(session, &deck_id, 2, &args.room_name).await?
        };
        println!(
            "{}",
            serde_json::json!({
                "phase": "waiting-for-player", "name": name, "game": chair.game_id,
                "gateway": lobby.base(), "deck": deck.name, "device": "cpu", "policy_mode": policy_mode_name(args),
                "model": args.model, "thinking_steps": args.thinking_steps,
            })
        );
        // Keep the adapter outside the cancellable future so both errors and
        // Ctrl-C retain the counters accumulated before the failure.
        let seat = PlayerId::new(u8::try_from(chair.seat).context("invalid seat")?);
        let mut core = None;
        let mut audit = None;
        let reopen_expired = args.join.is_none() && args.games == 0;
        let game = async {
            match wait_for_player(lobby, session, &chair, args.join.is_none(), reopen_expired)
                .await?
            {
                WaitOutcome::Playing => {}
                WaitOutcome::ExpiredBeforePlay => return Ok(false),
            }
            let mut brain = select_brain(args, &mut selected, &mut first_brain)?;
            audit = prepare_audit(args, &mut brain, games, seat)?;
            announce_model(args, &selected, &chair, audit.as_ref());
            core = Some(LiveSeat4::new(brain, deck.list.clone(), seat));
            let mut link = SeatLink::new(lobby.clone(), chair.clone(), Some(session.clone()));
            play::play(
                &mut link,
                core.as_mut().context("missing pinned brain")?,
                &mut audit,
            )
            .await?;
            Ok(true)
        };
        let result = tokio::select! {
            result = game => result,
            result = presence.lost() => result.map(|()| true),
            signal = tokio::signal::ctrl_c() => {
                match signal {
                    Ok(()) => Err(anyhow::anyhow!("seat stopped by operator")),
                    Err(_) => Err(anyhow::anyhow!("operator signal handler failed")),
                }
            }
        };
        if matches!(result, Ok(false)) {
            println!(
                "{}",
                serde_json::json!({
                    "phase": "waiting-room-expired",
                    "game": chair.game_id,
                    "gateway": lobby.base(),
                })
            );
            continue;
        }
        let auditing = audit
            .as_mut()
            .map_or(Ok(()), |audit| audit.finish(result.is_err()));
        let reporting = write_report(
            args,
            &chair,
            &selected,
            core.as_ref(),
            audit.as_ref(),
            result.is_err(),
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
        auditing?;
        reporting?;
        games += 1;
        if args.games != 0 && games >= args.games {
            return Ok(());
        }
    }
}

fn announce_model(
    args: &Args,
    selected: &model_selection::Selected,
    chair: &Chair,
    audit: Option<&audit::Writer>,
) {
    println!(
        "{}",
        serde_json::json!({
            "policy_mode":policy_mode_name(args), "phase":"model-pinned", "game_id":chair.game_id,"neural_seat":chair.seat,
            "model":selected.model,"model_label":selected.label,
            "model_sha256":selected.model_sha256,"score":selected.score,
            "benchmark_id":selected.benchmark_id,
            "decision_audit_id":audit.map(audit::Writer::id),
        })
    );
}

fn prepare_audit(
    args: &Args,
    brain: &mut NetPlayer4,
    game: u32,
    seat: PlayerId,
) -> anyhow::Result<Option<audit::Writer>> {
    let policy_seed = game_policy_seed(args.policy_seed, game, seat);
    if let Some(seed) = policy_seed {
        brain.reseed(seed);
    }
    args.decision_audits
        .as_ref()
        .map(|directory| {
            audit::Writer::open(
                directory,
                &serde_json::json!({
                    "model_identity":brain.model_identity(),
                    "producer_build":baylee_build::short(),
                    "producer_revision":baylee_build::COMMIT,
                    "producer_dirty":baylee_build::DIRTY,
                    "producer_binary_sha256":audit::producer_sha256()?,
                    "policy_contract":brain.policy_mode().contract(),
                    "temperature":args.temperature,"thinking_steps":args.thinking_steps,
                    "base_policy_seed":args.policy_seed,"effective_policy_seed":policy_seed,
                    "game_index":game,"seat":seat,
                    "seat_rng_rule":"base_xor_game_rotation17_xor_physical_seat_v1",
                }),
            )
        })
        .transpose()
}

fn select_brain(
    args: &Args,
    selected: &mut model_selection::Selected,
    first_brain: &mut Option<NetPlayer4>,
) -> anyhow::Result<NetPlayer4> {
    if let Some(path) = &args.model_selection {
        match model_selection::proposal(path, selected, args.thinking_steps) {
            Ok(candidate) => match load_brain(args, &candidate.model) {
                Ok(brain) => {
                    *selected = candidate;
                    *first_brain = Some(brain);
                }
                Err(_) => {
                    eprintln!("model selection could not load; retaining previous valid candidate");
                }
            },
            Err(_) => eprintln!(
                "model selection unavailable, incompatible or not better; retaining previous valid candidate"
            ),
        }
    }
    match first_brain.take() {
        Some(brain) => Ok(brain),
        None => load_brain(args, &selected.model),
    }
}

fn write_report(
    args: &Args,
    chair: &Chair,
    selected: &model_selection::Selected,
    core: Option<&LiveSeat4>,
    audit: Option<&audit::Writer>,
    failed: bool,
) -> anyhow::Result<()> {
    let empty = LiveReport4 {
        kind: "local_neural_v4",
        ..LiveReport4::default()
    };
    let aggregate = play::report_after(core.map_or(&empty, LiveSeat4::report), failed);
    let mut report = serde_json::to_value(aggregate)?;
    let fields = report
        .as_object_mut()
        .context("aggregate report must be an object")?;
    fields.insert(
        "policy_mode".into(),
        serde_json::json!(policy_mode_name(args)),
    );
    fields.insert("game_id".into(), serde_json::json!(chair.game_id));
    fields.insert(
        "decision_audit_id".into(),
        serde_json::json!(audit.map(audit::Writer::id)),
    );
    fields.insert("temperature".into(), serde_json::json!(args.temperature));
    fields.insert(
        "base_policy_seed".into(),
        serde_json::json!(args.policy_seed),
    );
    fields.insert("neural_seat".into(), serde_json::json!(chair.seat));
    fields.insert("model".into(), serde_json::json!(selected.model));
    fields.insert("model_label".into(), serde_json::json!(selected.label));
    fields.insert(
        "model_sha256".into(),
        serde_json::json!(selected.model_sha256),
    );
    fields.insert("score".into(), serde_json::json!(selected.score));
    fields.insert(
        "benchmark_id".into(),
        serde_json::json!(selected.benchmark_id),
    );
    // Always emit the audit record, even when writing its file fails.
    println!(
        "{}",
        serde_json::json!({"phase":if failed {"game-failed"} else {"game-complete"},"report":report})
    );
    if let Some(dir) = &args.reports {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", uuid::Uuid::now_v7()));
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

fn policy_mode_name(args: &Args) -> &'static str {
    if args.raw_policy { "raw" } else { "tactical" }
}

fn load_brain(args: &Args, model: &std::path::Path) -> anyhow::Result<NetPlayer4> {
    anyhow::ensure!(
        baylee_train::netplay4::inspect_model_identity(model)?.encoder_version < 8,
        "Encoders 8 and 9 require versioned context on the wire; use offline eval4"
    );
    Ok(NetPlayer4::load(model)?
        .with_policy_mode(if args.raw_policy {
            PolicyMode4::Raw
        } else {
            PolicyMode4::Tactical
        })
        .with_temperature(args.temperature)
        .with_thinking_steps(args.thinking_steps))
}

fn validate_sampling(args: &Args) -> anyhow::Result<()> {
    ensure!(
        valid_raw_temperature(args.temperature),
        "--temperature must be 0 or finite and >0.35 under sampler version 2"
    );
    ensure!(
        args.temperature == 0.0
            || (args.raw_policy && args.policy_seed.is_some() && args.decision_audits.is_some()),
        "stochastic live play requires --raw-policy, --policy-seed and --decision-audits"
    );
    Ok(())
}

fn game_policy_seed(base: Option<u64>, game: u32, seat: PlayerId) -> Option<u64> {
    base.map(|base| base ^ u64::from(game).rotate_left(17) ^ u64::from(seat.get()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaitOutcome {
    Playing,
    ExpiredBeforePlay,
}

fn missing_room_outcome(reopen_expired: bool) -> anyhow::Result<WaitOutcome> {
    if reopen_expired {
        Ok(WaitOutcome::ExpiredBeforePlay)
    } else {
        bail!("room no longer exists")
    }
}

async fn wait_for_player(
    lobby: &Lobby,
    session: &Session,
    chair: &Chair,
    host: bool,
    reopen_expired: bool,
) -> anyhow::Result<WaitOutcome> {
    loop {
        let Some(room) = lobby.room(session, &chair.game_id).await? else {
            return missing_room_outcome(reopen_expired);
        };
        ensure!(
            room.seats.len() == 2 && room.seats.iter().all(|seat| seat.kind == "human"),
            "this preview requires two socket seats and no house-AI chairs"
        );
        if room.playing() {
            return Ok(WaitOutcome::Playing);
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

#[cfg(test)]
mod tests {
    use super::{WaitOutcome, missing_room_outcome};

    #[test]
    fn continuous_hosting_reopens_reaped_waiting_rooms_while_single_runs_fail() {
        assert_eq!(
            missing_room_outcome(true).unwrap(),
            WaitOutcome::ExpiredBeforePlay
        );
        assert_eq!(
            missing_room_outcome(false).unwrap_err().to_string(),
            "room no longer exists"
        );
    }
}

#[cfg(test)]
mod policy_cli_tests {
    use super::*;
    #[test]
    fn raw_is_explicit_and_default_remains_tactical() {
        let default = Args::try_parse_from(["seat4", "--model", "/model"]).unwrap();
        assert_eq!(policy_mode_name(&default), "tactical");
        let raw = Args::try_parse_from(["seat4", "--model", "/model", "--raw-policy"]).unwrap();
        assert_eq!(policy_mode_name(&raw), "raw");
    }

    #[test]
    fn stochastic_live_policy_requires_explicit_seed_and_durable_audit() {
        for flags in [
            vec![],
            vec!["--raw-policy"],
            vec!["--raw-policy", "--policy-seed", "77"],
            vec!["--policy-seed", "77", "--decision-audits", "/audit"],
        ] {
            let mut args = vec!["seat4", "--model", "/model", "--temperature", "0.7"];
            args.extend(flags);
            assert!(validate_sampling(&Args::try_parse_from(args).unwrap()).is_err());
        }
        let complete = Args::try_parse_from([
            "seat4",
            "--model",
            "/model",
            "--temperature",
            "0.7",
            "--raw-policy",
            "--policy-seed",
            "77",
            "--decision-audits",
            "/audit",
        ])
        .unwrap();
        assert!(validate_sampling(&complete).is_ok());
        assert!(
            validate_sampling(&Args::try_parse_from(["seat4", "--model", "/model"]).unwrap())
                .is_ok()
        );
        for temperature in ["0.1", "0.35", "NaN", "inf", "-1"] {
            let args = Args::try_parse_from([
                "seat4",
                "--model",
                "/model",
                &format!("--temperature={temperature}"),
                "--raw-policy",
                "--policy-seed",
                "77",
                "--decision-audits",
                "/audit",
            ])
            .unwrap();
            assert!(validate_sampling(&args).is_err());
        }
    }

    #[test]
    fn seeded_games_and_seats_have_repeatable_independent_policy_streams() {
        let seed = |game, seat| game_policy_seed(Some(77), game, PlayerId::new(seat)).unwrap();
        let seeds = [seed(0, 0), seed(0, 1), seed(1, 0), seed(1, 1)];
        assert_eq!(
            seeds
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            seeds.len()
        );
        let stream = |policy_seed| {
            let mut rng = baylee_train::deckgen::Rng::new(policy_seed);
            (0..4).map(|_| rng.unit().to_bits()).collect::<Vec<_>>()
        };
        assert_eq!(stream(seed(1, 0)), stream(seed(1, 0)));
        assert_ne!(stream(seed(1, 0)), stream(seed(1, 1)));
        assert_eq!(game_policy_seed(None, 0, PlayerId::new(0)), None);
    }
}
