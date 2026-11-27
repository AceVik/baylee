//! `seat4 ranked`: Baylee AI's account in Ranked, the trained net flying
//! the ranked runner's seat (`baylee_seat::ranked`, `docs/trained-ai.md`
//! §"Baylee AI in Ranked").
//!
//! The runner signs in with its runner token, offers the account's decks,
//! takes each match's admission and keeps its sessions as it does for the
//! house; [`NetPilot`] only plays a match's seat, at what Baylee AI is
//! measured at ([`super::Policy::measured`]: greedy, the tactical layer with
//! the bundle, depth K). The net's recurrent memory is the game it saw from
//! its opening, so a socket that drops, an observation that fails, or a
//! first socket that opens mid-game (a runner that restarted) hands the seat
//! over ([`Piloted::HandOver`]), and the runner's house fallback plays the
//! rest, declared to the table as the house.
//!
//! The net itself declares nothing to the table: the wire has no kind for a
//! trained net (`v1::SeatMind`), and none is borrowed. People see Baylee AI
//! through Central's badge, kind `net`.
//!
//! Nothing here is training data. The ranked path keeps no journal, no
//! decision audit and no report file; each match's transcript (the runner's,
//! under `--state`) gets one line of the net's counts, and standard output
//! one line a game, neither with a view, a card or a name in it.

pub mod flight;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::{Context as _, ensure};
use baylee_account::ranked::ai::{AiKind, ranked_profile};
use baylee_core::ids::PlayerId;
use baylee_protocol::v1::SeatMind;
use baylee_seat::bridge::Played;
use baylee_seat::link::Table as _;
use baylee_seat::ranked::cli::RankedArgs;
use baylee_seat::ranked::{Flight, Pilot, Piloted, Seat};
use baylee_seat::seat::{AnsweredCounts, Outcome};
use baylee_seat::{DeckList, Stats};
use baylee_train::cardstructure::{compat, digest};
use baylee_train::live4::{Brain4, LiveSeat4};
use baylee_train::netplay4::{CatalogPolicy4, NetPlayer4};
use clap::Parser as _;
use flight::Flown;

/// The account key's family for a trained net (Central's `AiKind::Net`).
pub const FAMILY: &str = "net";

/// The runner's key for the net as Central's administrator holds it for
/// the account: `net:<label>`, a key that names the account's mind and not
/// one export, so the account keeps its rating as the champion moves on.
///
/// # Errors
/// A label Central would not take.
pub fn profile_for(label: &str) -> anyhow::Result<String> {
    let profile = format!("{FAMILY}:{label}");
    ensure!(
        ranked_profile(&profile),
        "`{profile}` is no key Central takes: a lowercase label of letters, digits, `_`, `.` and `-`"
    );
    Ok(profile)
}

/// The trained net at what it is measured at, flying an AI account's seats.
pub struct NetPilot {
    /// The export directory, canonical.
    model: PathBuf,
    /// The export's name (its directory's), for the records.
    label: String,
    /// The digest of its `net.pt`.
    sha256: String,
    /// The runner's key ([`profile_for`]).
    profile: String,
    /// How its card catalog binds to this build's.
    catalog: CatalogPolicy4,
    /// This build's cards it does not read as it was trained.
    unread: compat::Unread,
    /// Why it cannot take a game, once a load failed.
    broken: Mutex<Option<String>>,
}

impl NetPilot {
    /// The net in `model`, under the key `net:<net_label>`: its catalog
    /// bound the closest way this build allows (exact, else the strict
    /// superset, else drift), and loaded once here, so a model that cannot
    /// play never offers.
    ///
    /// # Errors
    /// A label Central would not take, or a model that does not load.
    pub fn new(model: &Path, net_label: &str) -> anyhow::Result<Self> {
        let profile = profile_for(net_label)?;
        let selected = super::model_selection::Selected::fixed(model)?;
        let compared = compat::compare(&selected.model).context("compare the model's catalog")?;
        let catalog = if digest(&compared.model)? == digest(&compared.compiled)? {
            CatalogPolicy4::Exact
        } else if compared.report.holds() {
            CatalogPolicy4::Superset
        } else {
            CatalogPolicy4::Drift
        };
        let unread = compat::Unread::of(&compared);
        drop(compared);
        drop(load(&selected.model, catalog).context("load the net")?);
        Ok(Self {
            model: selected.model,
            label: selected.label,
            sha256: selected.model_sha256,
            profile,
            catalog,
            unread,
            broken: Mutex::new(None),
        })
    }

    /// What the runner's start log says of the net.
    #[must_use]
    pub fn told(&self) -> String {
        let reads = match self.catalog {
            CatalogPolicy4::Exact => "this build's own catalog".to_owned(),
            CatalogPolicy4::Superset => format!(
                "an older export on a newer catalog (superset, {} cards appended)",
                self.unread.appended.len()
            ),
            CatalogPolicy4::Drift => format!(
                "an older export on a newer catalog (drift, {} cards withdrawn, {} appended)",
                self.unread.withdrawn.len(),
                self.unread.appended.len()
            ),
        };
        format!(
            "the net {} (net.pt {}) at depth {} with the measured rule groups, greedy; {reads}",
            self.label,
            &self.sha256[..12.min(self.sha256.len())],
            baylee_train_ops::measured::THINKING_STEPS,
        )
    }

    /// Cards of `deck` the net does not read as it was trained, by name.
    fn unread_in(&self, deck: &DeckList) -> Vec<String> {
        let mut names: Vec<String> = deck
            .main
            .iter()
            .chain(&deck.commanders)
            .chain(&deck.sideboard)
            .filter(|card| self.unread.contains(card.card.get()))
            .filter_map(|card| baylee_cards::by_index(card.card).map(|c| c.name().to_owned()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    fn break_with(&self, why: &str) {
        *self.broken.lock().unwrap_or_else(PoisonError::into_inner) = Some(why.to_owned());
    }

    /// What a match's records say of how the net played it.
    fn record<B: Brain4>(
        &self,
        core: &LiveSeat4<B>,
        rule_refs_left_out: u64,
        own_unread: &[String],
        flown: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "model": self.label,
            "model_sha256": self.sha256,
            "profile": self.profile,
            "thinking_steps": baylee_train_ops::measured::THINKING_STEPS,
            "rule_groups": baylee_train_ops::measured::RULE_GROUPS,
            "catalog": self.catalog,
            "flown": flown,
            "turns": core.turn(),
            "rule_refs_left_out": rule_refs_left_out,
            "own_cards_unread": own_unread,
            "report": core.report(),
        })
    }
}

/// The measured net in `model`, its catalog bound by `catalog`.
fn load(model: &Path, catalog: CatalogPolicy4) -> anyhow::Result<NetPlayer4> {
    super::load_brain(
        &super::Policy {
            catalog,
            ..super::Policy::measured()
        },
        model,
    )
}

impl Pilot for NetPilot {
    /// Never sent: the wire has no kind for a trained net, and the seat
    /// declares nothing rather than something it is not.
    fn declared(&self) -> SeatMind {
        SeatMind::default()
    }

    fn kind(&self) -> AiKind {
        AiKind::Net
    }

    fn profile(&self) -> String {
        self.profile.clone()
    }

    fn ready(&self) -> Result<(), String> {
        self.broken
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map_or(Ok(()), |why| Err(why.clone()))
    }

    fn fly<'a>(&'a self, seat: Seat<'a>) -> Flight<'a> {
        Box::pin(async move {
            let match_id = uuid::Uuid::from_bytes(seat.link.assignment().match_id);
            let (model, catalog) = (self.model.clone(), self.catalog);
            let loaded = tokio::task::spawn_blocking(move || load(&model, catalog)).await;
            let Ok(Ok(brain)) = loaded else {
                let why = "the net's model could not be loaded";
                self.break_with(why);
                seat.transcript.write_value(
                    &serde_json::json!({ "net": { "model": self.label, "failure": why } }),
                );
                return Ok(Piloted::HandOver(why.to_owned()));
            };
            let me = PlayerId::new(seat.link.assignment().seat);
            let deck = seat.deck.bridge.list.clone();
            let own_unread = self.unread_in(&deck);
            if !own_unread.is_empty() {
                eprintln!(
                    "match {match_id}: {} cards of the deck are read by their summary alone, \
                     not as the net was trained: {}",
                    own_unread.len(),
                    own_unread.join(", ")
                );
            }
            let mut core = LiveSeat4::new(brain, deck, me);
            core.require_opening();
            let pace = flight::Pace {
                min_think: seat.play.min_think,
                room_check: seat.play.room_check,
            };
            let flown = flight::fly(&mut *seat.link, &mut core, pace).await;
            let how = match &flown {
                Ok(Flown::Ended) => "ended",
                Ok(Flown::Closed) => "closed",
                Ok(Flown::HandOver(_)) => "handed_over",
                Err(_) => "no_seat",
            };
            let left_out = core.brain().rule_refs_left_out();
            let record = self.record(&core, left_out, &own_unread, how);
            seat.transcript
                .write_value(&serde_json::json!({ "net": &record }));
            println!(
                "{}",
                serde_json::json!({ "phase": "ranked-game", "match": match_id, "net": record })
            );
            Ok(match flown? {
                Flown::Ended | Flown::Closed => {
                    Piloted::Played(Box::new(played(&core, me, seat.link.dials())))
                }
                Flown::HandOver(why) => Piloted::HandOver(why),
            })
        })
    }
}

/// The runner's account of a game the net played to its end.
fn played<B: Brain4>(core: &LiveSeat4<B>, me: PlayerId, dials: u32) -> Played {
    let report = core.report();
    let count = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
    let stats = Stats {
        questions: count(report.answers),
        answered: AnsweredCounts {
            // The net's policy, raw or under its tactical layer.
            mind: count(report.neural_answers + report.search_answers + report.tactical_answers),
            // The runtime's own fallbacks and the live draw rule.
            house: count(
                report
                    .fallback_answers
                    .saturating_sub(report.tactical_answers),
            ),
            standing: count(report.forced_answers),
            least: 0,
        },
        turns: core.turn(),
        outcome: core.game_result().map(|result| match result.winner {
            None => Outcome::Draw,
            Some(winner) if winner.includes(me, None) => Outcome::Won,
            Some(_) => Outcome::Lost,
        }),
        ..Stats::default()
    };
    Played {
        stats,
        result: core.game_result().copied(),
        dials,
        fallbacks: Vec::new(),
        reports: None,
    }
}

#[derive(clap::Parser)]
#[command(
    name = "seat4 ranked",
    bin_name = "seat4 ranked",
    about = "Baylee AI's account in Ranked: the trained net flies the ranked runner's seats"
)]
struct Args {
    #[command(flatten)]
    ranked: RankedArgs,
    /// The exported checkpoint the net plays, read only (its `model*.onnx`,
    /// `model.onnx.json`, `card_structure.json` and `net.pt`).
    #[arg(long)]
    model: PathBuf,
    /// The account key's label: Central holds the account as
    /// `net:<label>`. A key for the account, not for one export, so the
    /// account keeps its rating as the champion moves on; the export's own
    /// name goes into every match's record.
    #[arg(long, default_value = "baylee")]
    net_label: String,
}

/// `seat4 ranked …`: the net loaded and checked first, then the ranked
/// runner, which arms the stop signals and does the rest
/// ([`baylee_seat::ranked::cli::run`]).
pub fn main() -> anyhow::Result<()> {
    let args = Args::parse_from(std::env::args_os().skip(1));
    // The net plays on the CPU; the GPU belongs to training.
    ensure!(
        std::env::var("BAYLEE_EP").as_deref() != Ok("cuda"),
        "seat4 uses CPU inference; unset BAYLEE_EP or set it to cpu"
    );
    let pilot = NetPilot::new(&args.model, &args.net_label)?;
    eprintln!("{}", pilot.told());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let done = runtime.block_on(baylee_seat::ranked::cli::run(&args.ranked, Arc::new(pilot)));
    runtime.shutdown_timeout(Duration::from_secs(5));
    done
}
