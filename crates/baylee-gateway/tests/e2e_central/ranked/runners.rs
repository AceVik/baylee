//! An AI player fills a lonely person's ranked search and the two finish a
//! ranked game on a verified host, through Central's relay
//! (`docs/llm-seat.md` §"The ranked runner", baylee-central
//! `docs/ai-players.md`).
//!
//! An administrator creates the AI on Central's website (no password),
//! switches it on and issues a runner token for it; the runner
//! (`baylee_seat::ranked`) trades the token for a session, accepts the
//! terms, offers its deck and reads Central's work; a person searches with
//! "AI opponents: house" and, after the fill-in wait, Central seats the AI
//! from its offer. The person's seat is played by the house through the
//! person's own queue entry, as the native client takes it; the AI's by its
//! runner, with a pilot that cannot resume: its first socket is cut
//! mid-game, it hands the seat over, and the runner's house fallback takes
//! the seat again with the game's id and a fresh core and plays on. Both see
//! the whole game, and Central's results agree with the table's.
//!
//! Run it from a clean commit: the host's engine advertises a ranked
//! compatibility key only for a clean, stamped build, and without one
//! Central finds no host for the match.
use super::{
    AccountStore, DatabaseConnection, Runtime, Served, capacity, common, fresh, registered,
};
use baylee_account::ranked::ai::{AiKind, AiOpponents};
use baylee_account::ranked::replica::{Phase, SeatRequest, SeatResponse};
use baylee_account::ranked::{Format, Id, QueueState, Request, Response};
use baylee_account::wire::{Request as Wire, Response as Answer};
use baylee_account::{Account, CentralHttp, Error, Token, sdk};
use baylee_client_core::reconnect::Window;
use baylee_core::ids::PlayerId;
use baylee_engine::win::Victor;
use baylee_seat::bridge::{PlayOptions, Played};
use baylee_seat::link::{Reporter, Socket, Table};
use baylee_seat::ranked::cli::{self, RankedArgs};
use baylee_seat::ranked::{
    self, Flight, Frozen, HousePilot, MatchReport, Pilot, Piloted, Player, RunnerToken, Seat,
};
use baylee_seat::transcript::Transcript;
use central_api::admin::{AdminAnswer, AdminRequest, AiPlayerSettings, AiSettings};
use central_crypto::{Context, opaque::Login};
use futures_util::{SinkExt as _, StreamExt as _};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

#[test]
#[ignore = "requires owned PostgreSQL 18 fixture from cargo dev central integration --ai-players-only"]
fn an_ai_runner_fills_a_persons_search_and_they_finish_a_ranked_game() {
    super::super::integration(super::super::Scenario::Runners);
}

/// The administrator's website session.
struct Desk<'a> {
    rt: &'a Runtime,
    served: &'a Served,
    cookie: Token,
}

impl Desk<'_> {
    /// An administrator signed in on the website with the password, as a
    /// change to an AI asks.
    fn open<'a>(
        rt: &'a Runtime,
        served: &'a Served,
        operator: &DatabaseConnection,
        username: &str,
    ) -> Desk<'a> {
        let _ = fresh::register(
            rt,
            &served.service,
            &served.endpoint,
            username,
            fresh::DOCUMENT,
        );
        rt.block_on(central_postgres::appoint_admin(operator, username))
            .unwrap();
        let http = CentralHttp::new(served.endpoint.clone());
        let baylee_account::wire::Discovery::Central { identity } =
            sdk::discover(&http, username.to_owned()).unwrap()
        else {
            panic!("the administrator is enrolled at Central");
        };
        let context =
            Context::new(served.endpoint.issuer(), identity.account, identity.epoch).unwrap();
        let (login, message) = Login::start(fresh::password().into(), context).unwrap();
        let website = |request: &Wire| {
            let request = serde_json::from_slice(&serde_json::to_vec(request).unwrap()).unwrap();
            rt.block_on(served.service.handle_browser(request, None))
                .unwrap()
        };
        let Answer::LoginChallenge { attempt, message } =
            website(&Wire::LoginStart { identity, message }).response
        else {
            panic!("a challenge");
        };
        let finish = Wire::LoginFinish {
            attempt,
            message: login.finish(&message).unwrap().finalization,
        };
        let cookie = website(&finish).session.expect("a session for the cookie");
        Desk { rt, served, cookie }
    }

    fn ask(&self, request: AdminRequest) -> Answer {
        let request = Wire::Admin { request };
        let request = serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        self.rt
            .block_on(
                self.served
                    .service
                    .handle_browser(request, Some(&self.cookie)),
            )
            .unwrap_or_else(|e| panic!("the administration refused: {e:?}"))
            .response
    }
}

/// The AI a run creates: the in-process house, or what an external
/// runner flies ([`External`]).
struct AiDef {
    kind: AiKind,
    profile: String,
    label: String,
    name: &'static str,
}

impl AiDef {
    fn house() -> Self {
        Self {
            kind: AiKind::House,
            profile: "house:expert".into(),
            label: "Haus-KI · Expert".into(),
            name: "KI-Haus-Expert",
        }
    }
}

/// The AI as an administrator sets it up: fill-in after Central's shortest
/// wait, no host slots held back for people (the fixture's host has few),
/// the AI created, switched on, and a runner token for it alone.
fn set_up_ai(desk: &Desk<'_>, ai: &AiDef) -> (Id, u32, Token) {
    ai_settings(desk, true);
    let (account, tag) = create_ai(desk, ai);
    (account, tag, runner_token(desk, vec![account]))
}

/// Every AI's settings: fill-in after Central's shortest wait where
/// `fill_in`, else the carousel only (one AI match at once, ten a day); no
/// host slots held back for people (the fixture's host has few).
fn ai_settings(desk: &Desk<'_>, fill_in: bool) {
    assert!(matches!(
        desk.ask(AdminRequest::SetAiSettings {
            settings: AiSettings {
                fill_in,
                fill_in_after: 30,
                drain_minutes: 30,
                global_seats: 8,
                host_reserve: 0,
                carousel_games: u8::from(!fill_in),
                carousel_daily: if fill_in { 0 } else { 10 },
            },
        }),
        Answer::Done
    ));
}

/// The AI `ai`, created and switched on: its account and tag.
fn create_ai(desk: &Desk<'_>, ai: &AiDef) -> (Id, u32) {
    let Answer::Admin {
        answer: AdminAnswer::AiPlayerCreated { account, tag },
    } = desk.ask(AdminRequest::CreateAiPlayer {
        settings: AiPlayerSettings {
            kind: ai.kind,
            profile: ai.profile.clone(),
            label: ai.label.clone(),
            max_games: 1,
            // Central requires a day's cap for a language model.
            daily_games: (ai.kind == AiKind::Model).then_some(10),
            display_name: ai.name.into(),
        },
        tag: None,
    })
    else {
        panic!("an AI player");
    };
    assert!(matches!(
        desk.ask(AdminRequest::SetAiEnabled {
            account,
            enabled: true,
        }),
        Answer::Done
    ));
    (account, tag)
}

/// A runner token for `accounts`.
fn runner_token(desk: &Desk<'_>, accounts: Vec<Id>) -> Token {
    let Answer::Admin {
        answer: AdminAnswer::AiRunnerTokenCreated { token, .. },
    } = desk.ask(AdminRequest::CreateAiRunnerToken {
        name: "e2e runner".into(),
        accounts,
        days: 1,
    })
    else {
        panic!("a runner token");
    };
    token
}

/// The deck both sides play: sixty Forests, so the game runs to a library
/// that runs out.
fn forests() -> Frozen {
    Frozen::from_rows(
        "Forest",
        "standard",
        Format::Standard,
        &["60 Forest".into()],
        &[],
        &[],
    )
    .unwrap()
}

/// A ranked link whose first socket goes through a loopback pair that the
/// test cuts after `frames` frames from the table: the seat sees its socket
/// drop mid-game, as a network would drop it.
struct Cutting<'a, L> {
    link: &'a mut L,
    frames: Option<usize>,
}

impl<L: Table> Table for Cutting<'_, L> {
    fn hold_for(&mut self, window: Window) {
        self.link.hold_for(window);
    }

    async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        let Some(real) = self.link.connect().await? else {
            return Ok(None);
        };
        let Some(frames) = self.frames.take() else {
            return Ok(Some(real));
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let accepted = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            anyhow::Ok(tokio_tungstenite::accept_async(tcp).await?)
        });
        let (near, _) = tokio_tungstenite::connect_async(format!("ws://{address}/")).await?;
        let far = accepted.await??;
        tokio::spawn(async move {
            let (mut far_out, mut far_in) = far.split();
            let (mut real_out, mut real_in) = real.split();
            let up = async {
                while let Some(Ok(message)) = far_in.next().await {
                    if real_out.send(message).await.is_err() {
                        break;
                    }
                }
            };
            let down = async {
                for _ in 0..frames {
                    let Some(Ok(message)) = real_in.next().await else {
                        break;
                    };
                    if far_out.send(message).await.is_err() {
                        break;
                    }
                }
            };
            // Whichever ends first ends both: the halves drop, and with them
            // both connections, without a closing frame.
            tokio::select! {
                () = up => {}
                () = down => {}
            }
        });
        Ok(Some(near))
    }

    async fn room_closed(&mut self) -> bool {
        self.link.room_closed().await
    }

    async fn selected_deck(&self) -> anyhow::Result<Option<baylee_seat::mind::DeckList>> {
        self.link.selected_deck().await
    }

    fn reporter(&self) -> Option<Reporter<'_>> {
        self.link.reporter()
    }

    fn seat_secret(&self) -> &str {
        self.link.seat_secret()
    }

    fn dials(&self) -> u32 {
        self.link.dials()
    }
}

/// A pilot that cannot resume, as the trained net cannot rebuild its memory
/// from a snapshot: it plays the AI's seat with the house through a link
/// that cuts its first socket and lets it dial no other, then hands the
/// seat over. The runner's house fallback must take the seat again, with a
/// fresh core, mid-game, on the same link, and play it to the end.
struct CuttingHouse {
    house: HousePilot,
    frames: Mutex<Option<usize>>,
}

/// [`Cutting`], dialling once only: the socket after the cut is never
/// opened, so the seat's bridge sees the game gone.
struct Once<'a, L> {
    cutting: Cutting<'a, L>,
    dialed: bool,
}

impl<L: Table> Table for Once<'_, L> {
    fn hold_for(&mut self, window: Window) {
        self.cutting.hold_for(window);
    }

    async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        if std::mem::replace(&mut self.dialed, true) {
            return Ok(None);
        }
        self.cutting.connect().await
    }

    async fn room_closed(&mut self) -> bool {
        self.cutting.room_closed().await
    }

    fn selected_deck(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<baylee_seat::mind::DeckList>>> + Send {
        std::future::ready(Ok(None))
    }

    fn reporter(&self) -> Option<Reporter<'_>> {
        None
    }

    fn seat_secret(&self) -> &str {
        self.cutting.seat_secret()
    }

    fn dials(&self) -> u32 {
        self.cutting.dials()
    }
}

impl Pilot for CuttingHouse {
    fn declared(&self) -> baylee_protocol::v1::SeatMind {
        self.house.declared()
    }

    fn kind(&self) -> AiKind {
        self.house.kind()
    }

    fn profile(&self) -> String {
        self.house.profile()
    }

    fn fly<'a>(&'a self, seat: Seat<'a>) -> Flight<'a> {
        let frames = self
            .frames
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        Box::pin(async move {
            let mut once = Once {
                cutting: Cutting {
                    link: seat.link,
                    frames,
                },
                dialed: false,
            };
            let played = self
                .house
                .fly_through(&mut once, seat.deck, seat.transcript, seat.play)
                .await?;
            Ok(match played.result {
                Some(_) => Piloted::Played(Box::new(played)),
                None => Piloted::HandOver(format!(
                    "the socket dropped in turn {} and this pilot cannot resume",
                    played.stats.turns
                )),
            })
        })
    }
}

/// The person's seat, through their queue entry as the native client takes
/// it: the relay seats them with the frozen deck, then with the game's id.
struct PersonLink {
    account: Arc<Mutex<Account>>,
    key: Id,
    deck: Arc<Frozen>,
    pending: Arc<Mutex<Option<baylee_account::RankedHost>>>,
    ticket: zeroize::Zeroizing<String>,
    dials: u32,
}

impl PersonLink {
    async fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut Account) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let account = Arc::clone(&self.account);
        tokio::task::spawn_blocking(move || {
            call(&mut account.lock().unwrap_or_else(PoisonError::into_inner))
        })
        .await
        .unwrap()
    }

    async fn finished(&self) -> bool {
        self.call(|account| match account.ranked(Request::Status)? {
            Response::Queue {
                state: QueueState::Assigned { assignment, .. },
            } => {
                let SeatResponse::Status { status } = account.replica(SeatRequest::Status {
                    match_id: assignment.match_id,
                })?
                else {
                    return Err(Error::Rejected);
                };
                Ok(matches!(status.phase, Phase::Finished | Phase::Void))
            }
            _ => Ok(true),
        })
        .await
        .unwrap_or(false)
    }
}

impl Table for PersonLink {
    fn hold_for(&mut self, _: Window) {}

    async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        let until = tokio::time::Instant::now() + Duration::from_secs(180);
        loop {
            let (key, deck, pending) =
                (self.key, Arc::clone(&self.deck), Arc::clone(&self.pending));
            let asked = self
                .call(move |account| {
                    let mut pending = pending.lock().unwrap_or_else(PoisonError::into_inner);
                    match account.play_ranked(key, Some((deck.digest, &deck.deck)), &mut pending) {
                        Err(Error::TermsRequired) => {
                            let QueueState::Assigned { assignment, .. } =
                                (match account.ranked(Request::Status)? {
                                    Response::Queue { state } => state,
                                    _ => return Err(Error::Rejected),
                                })
                            else {
                                return Err(Error::Conflict);
                            };
                            account
                                .ranked_host_documents(assignment.match_id)?
                                .accept(account)?;
                            Ok(None)
                        }
                        other => other,
                    }
                })
                .await;
            match asked {
                Ok(Some(play)) => {
                    let url = ranked::socket_url(&play.relay_origin)?;
                    let mut socket = baylee_seat::link::open_socket(&url)
                        .await
                        .map_err(|why| anyhow::anyhow!("{why}"))?;
                    socket
                        .send(Message::Binary(
                            play.seat.seat_token.as_bytes().to_vec().into(),
                        ))
                        .await?;
                    self.ticket = zeroize::Zeroizing::new(play.seat.seat_token.as_str().to_owned());
                    self.dials += 1;
                    return Ok(Some(socket));
                }
                Ok(None) => {}
                Err(Error::Conflict) if self.finished().await => return Ok(None),
                Err(e) => tracing::info!(error = %e, "the person has no seat yet"),
            }
            anyhow::ensure!(
                tokio::time::Instant::now() < until,
                "the person found no seat"
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn room_closed(&mut self) -> bool {
        self.finished().await
    }

    fn selected_deck(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<baylee_seat::mind::DeckList>>> + Send {
        std::future::ready(Ok(None))
    }

    fn reporter(&self) -> Option<Reporter<'_>> {
        None
    }

    fn seat_secret(&self) -> &str {
        &self.ticket
    }

    fn dials(&self) -> u32 {
        self.dials
    }
}

/// The person's side: a search that accepts the house, the assignment
/// Central makes, the whole game with the house at their seat, and their
/// receipt.
async fn person(
    account: Account,
    transcripts: &std::path::Path,
    deck: Frozen,
    ai: &AiDef,
) -> (Id, Played, QueueState) {
    let deck = Arc::new(deck);
    let accepts = if ai.kind == AiKind::Model {
        AiOpponents::Any
    } else {
        AiOpponents::Local
    };
    let key = *uuid::Uuid::now_v7().as_bytes();
    let account = Arc::new(Mutex::new(account));
    let mut link = PersonLink {
        account,
        key,
        deck: Arc::clone(&deck),
        pending: Arc::default(),
        ticket: zeroize::Zeroizing::new(String::new()),
        dials: 0,
    };
    let digest = deck.digest;
    let joined = link
        .call(move |account| {
            account.replica(SeatRequest::Join {
                key,
                format: Format::Standard,
                deck: digest,
                ai: accepts,
            })
        })
        .await
        .unwrap();
    assert!(matches!(joined, SeatResponse::Queue { .. }));
    let mut fill_in_at = None;
    let match_id = loop {
        let state = link
            .call(|account| match account.ranked(Request::Status)? {
                Response::Queue { state } => Ok(state),
                _ => Err(Error::Rejected),
            })
            .await
            .unwrap();
        match state {
            QueueState::Waiting { ai_fill_in_at, .. } => fill_in_at = fill_in_at.or(ai_fill_in_at),
            QueueState::Assigned { assignment, .. } => {
                assert!(
                    fill_in_at.is_some(),
                    "the search said when an AI may be seated"
                );
                let badge = assignment
                    .opponent_ai
                    .clone()
                    .expect("the person is told the opponent is an AI");
                assert_eq!(badge.kind, ai.kind);
                assert_eq!(badge.label, ai.label);
                break assignment.match_id;
            }
            other => panic!("the person's search ended as {other:?}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    let mut transcript =
        Transcript::file(&transcripts.join("person.jsonl")).expect("the person's transcript");
    let play = PlayOptions {
        min_think: Duration::ZERO,
        ..PlayOptions::default()
    };
    let played = HousePilot::new("expert")
        .unwrap()
        .fly_through(&mut link, &deck, &mut transcript, &play)
        .await
        .expect("the person's seat played to the end");
    transcript.flush();
    let until = tokio::time::Instant::now() + Duration::from_secs(120);
    let receipt = loop {
        let state = link
            .call(|account| match account.ranked(Request::Status)? {
                Response::Queue { state } => Ok(state),
                _ => Err(Error::Rejected),
            })
            .await
            .unwrap();
        if matches!(state, QueueState::Finished { .. }) || tokio::time::Instant::now() >= until {
            break state;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    let account = Arc::try_unwrap(link.account)
        .ok()
        .expect("the person's account is free again")
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    let _ = tokio::task::spawn_blocking(move || account.logout()).await;
    (match_id, played, receipt)
}

pub(in super::super) fn run(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
) {
    // The host belongs to a third account: neither player owns where it is
    // played.
    let _owner = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        "runner_host",
        fresh::DOCUMENT,
    );
    let host = registered(rt, served, store, operator, "runner_host", true);
    let agent = rt.block_on(common::ranked::provision(&host.gateway));
    capacity(rt, operator, &host);
    let desk = Desk::open(rt, served, operator, "runner_admin");
    let external = External::from_env();
    let def = external.as_ref().map_or_else(AiDef::house, External::def);
    let (ai, tag, token) = set_up_ai(&desk, &def);
    let person_account = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        "runner_person",
        fresh::DOCUMENT,
    );
    if let Some(external) = external {
        external.run(rt, served, &host.origin, ai, &def, &token, person_account);
        agent.abort();
        return;
    }
    let transcripts = fresh_transcripts();
    let (state, token_file) = runner_state(&token, None);
    // The runner's own command line, by the account's public handle.
    let args = RankedArgs {
        central: served.endpoint.origin().to_owned(),
        account: format!("KI-Haus-Expert#{tag:04x}"),
        token_file: Some(token_file.clone()),
        token_key: None,
        state: Some(state.clone()),
        decks: None,
        transcripts: Some(transcripts.clone()),
        fallback_level: "expert".into(),
        games: Some(1),
        accept_hosts: vec![host.origin.clone()],
        facts_from: None,
        facts_file: None,
        min_think_ms: 0,
        no_carousel: false,
    };
    let (report, person) = rt.block_on(async {
        let pilot: Arc<dyn Pilot> = Arc::new(CuttingHouse {
            house: HousePilot::new("expert").unwrap(),
            frames: Mutex::new(Some(40)),
        });
        let mut reports = Vec::new();
        let (played, person) = tokio::time::timeout(Duration::from_secs(540), async {
            tokio::join!(
                async {
                    cli::play(&args, pilot, |r| reports.push(r))
                        .await
                        .expect("the runner plays")
                },
                person(person_account, &transcripts, forests(), &def)
            )
        })
        .await
        .expect("both sides finish their ranked game");
        assert_eq!(played, 1, "the runner played one match");
        // The runner withdrew its offer and signed out when it stopped: a
        // fresh session of the AI finds no live match and no offer to keep.
        let player = Player::open(
            served.endpoint.clone(),
            ai,
            RunnerToken::from_file(token_file.clone()),
        )
        .await
        .expect("the token is still good");
        assert_eq!(player.name(), "KI-Haus-Expert");
        let after = player
            .call(|account| account.ai(baylee_account::ranked::ai::Request::Work))
            .await
            .unwrap();
        player.sign_out().await;
        let baylee_account::ranked::ai::Response::Work { assignments, .. } = after else {
            panic!("work");
        };
        assert!(assignments.is_empty(), "no live match is left");
        let report = reports
            .pop()
            .expect("the runner told how its match went")
            .expect("the runner played its match to the end");
        (report, person)
    });
    chronicled(&state, &report);
    agreed(&report, &person);
    agent.abort();
}

/// The game is in the deck's chronicle with Central's own result.
fn chronicled(state: &std::path::Path, report: &MatchReport) {
    let games = baylee_seat::deckbuild::history::games(&state.join("decks"), Format::Standard);
    assert_eq!(games.len(), 1, "one game in the chronicle");
    assert_eq!(games[0].version, 1);
    assert_eq!(games[0].turns, Some(report.played.stats.turns));
    let receipt = report.receipt.expect("Central settled the AI's match");
    assert!(
        !receipt.inferred,
        "Central's own result, not the standing's"
    );
    assert_eq!(
        (games[0].elo, games[0].delta),
        (Some(receipt.elo), receipt.delta)
    );
    assert!(
        games[0]
            .note
            .as_deref()
            .is_none_or(|n| !n.contains("inferred")),
        "{:?}",
        games[0].note
    );
}

/// Both sides saw a whole game to its end, the AI through a socket it lost
/// and took again, and they and Central agree on the result.
fn agreed(ai: &MatchReport, (match_id, person, receipt): &(Id, Played, QueueState)) {
    assert_eq!(ai.match_id, *match_id, "the AI filled the person's search");
    let receipt_ai = ai.receipt.expect("Central settled the AI's match");
    let QueueState::Finished {
        match_id: settled,
        seat,
        outcome,
        opponent_ai,
        ..
    } = receipt
    else {
        panic!("the person's receipt: {receipt:?}");
    };
    assert_eq!(settled, match_id);
    assert!(opponent_ai.is_some(), "the receipt names the AI as an AI");
    assert_ne!(*seat, receipt_ai.seat);
    assert_eq!(
        *outcome, receipt_ai.outcome,
        "Central counted one result for both"
    );
    for (played, seat) in [(&ai.played, receipt_ai.seat), (person, *seat)] {
        let result = played
            .result
            .expect("the seat saw the game's end on its socket");
        let won = match result.winner {
            Some(Victor::Player(winner)) => Some(winner == PlayerId::new(seat)),
            Some(Victor::Team(_)) => panic!("a duel has no teams"),
            None => None,
        };
        let central = match outcome {
            baylee_account::ranked::Outcome::FirstWins => Some(seat == 0),
            baylee_account::ranked::Outcome::SecondWins => Some(seat == 1),
            baylee_account::ranked::Outcome::Draw => None,
        };
        assert_eq!(won, central, "the table and Central agree on the result");
        // Two decks of sixty Forests end when a library runs out (turn 114
        // in the run of 26.11.2026, a clock forfeit far sooner), and no
        // question is left to its clock.
        assert!(
            played.stats.turns >= 100,
            "a whole game, not a forfeit: {} turns, {result:?}",
            played.stats.turns
        );
        assert_eq!(
            played.stats.fallbacks.expired, 0,
            "no question ran out of time"
        );
    }
    assert!(
        ai.played.dials >= 2,
        "the cut socket was replaced: {} sockets",
        ai.played.dials
    );
    let why = ai
        .handed_over
        .as_deref()
        .expect("the pilot handed the seat to the house after the cut");
    assert!(why.contains("cannot resume"), "{why}");
}

/// The runner's state as an operator lays it out: the token in a file only
/// the runner reads, and one Standard deck version with its record. The
/// state directory and the token file.
fn runner_state(
    token: &Token,
    decks: Option<&std::path::Path>,
) -> (std::path::PathBuf, std::path::PathBuf) {
    state_named("ai-players-runner-state", token, decks)
}

/// [`runner_state`] under `<target>/tmp/<name>`.
fn state_named(
    name: &str,
    token: &Token,
    decks: Option<&std::path::Path>,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::io::Write as _;
    let state = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&state);
    if let Some(decks) = decks {
        copy_dir(decks, &state.join("decks"));
    } else {
        let standard = state.join("decks").join("standard");
        std::fs::create_dir_all(&standard).unwrap();
        std::fs::write(
            standard.join("v0001-20261127T000000Z.txt"),
            "# name: Forest\n# format: standard\n60 Forest\n",
        )
        .unwrap();
        std::fs::write(standard.join("v0001-20261127T000000Z.json"), "{}").unwrap();
    }
    let token_file = state.join("runner-token");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let text = baylee_account::sdk::ai_runner::token_text(token);
    writeln!(options.open(&token_file).unwrap(), "{}", *text).unwrap();
    (state, token_file)
}

/// Copies the directory `from` into `to`, whole.
fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// An AI runner process the environment names, flying the AI instead of the
/// in-process house: the proof game of a real pilot (a language model on its
/// CLIs, the trained net's `seat4`) on the machine whose CLIs are signed in.
///
/// - `BAYLEE_E2E_AI_RUNNER`: the binary (`baylee-seat`, `seat4`);
/// - `BAYLEE_E2E_AI_KIND`, `BAYLEE_E2E_AI_PROFILE`: the account Central makes
///   (`model` and `model:haiku-high`, `net` and `net:baylee`; default
///   `house` and `house:expert`);
/// - `BAYLEE_E2E_AI_ARGS`: the runner's own flags, split at whitespace
///   (`--profile haiku-high --config /path/llm-seat.json`);
/// - `BAYLEE_E2E_AI_DECKS`: a deck directory copied into the runner's state;
///   the person plays its newest Standard deck too (default: sixty Forests).
struct External {
    runner: std::path::PathBuf,
    kind: AiKind,
    profile: String,
    args: Vec<String>,
    decks: Option<std::path::PathBuf>,
}

/// A process the test started, killed and waited for when dropped.
struct Running(std::process::Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl External {
    fn from_env() -> Option<Self> {
        let runner = std::env::var_os("BAYLEE_E2E_AI_RUNNER")?.into();
        let kind = std::env::var("BAYLEE_E2E_AI_KIND").unwrap_or_else(|_| "house".into());
        let kind = AiKind::from_key(&kind).expect("BAYLEE_E2E_AI_KIND is house, model or net");
        Some(Self {
            runner,
            kind,
            profile: std::env::var("BAYLEE_E2E_AI_PROFILE")
                .unwrap_or_else(|_| "house:expert".into()),
            args: std::env::var("BAYLEE_E2E_AI_ARGS")
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            decks: std::env::var_os("BAYLEE_E2E_AI_DECKS").map(Into::into),
        })
    }

    fn def(&self) -> AiDef {
        let label: String = format!("E2E · {}", self.profile).chars().take(40).collect();
        AiDef {
            kind: self.kind,
            profile: self.profile.clone(),
            label: label.trim_end().to_owned(),
            name: "KI-E2E-Runner",
        }
    }

    /// The person's search, the runner's process for the AI, the game, and
    /// one line on how the AI played.
    #[allow(clippy::too_many_arguments)]
    fn run(
        self,
        rt: &Runtime,
        served: &Served,
        host: &str,
        ai: Id,
        def: &AiDef,
        token: &Token,
        person_account: Account,
    ) {
        let transcripts = fresh_transcripts();
        let (state, token_file) = runner_state(token, self.decks.as_deref());
        let deck = self.decks.as_deref().map_or_else(forests, |dir| {
            let newest = baylee_seat::deckbuild::history::newest(dir, Format::Standard)
                .expect("BAYLEE_E2E_AI_DECKS holds a Standard deck");
            Frozen::from_file(&newest, Format::Standard).expect("the Standard deck freezes")
        });
        let mut command = std::process::Command::new(&self.runner);
        command
            .arg("ranked")
            .arg("--central")
            .arg(served.endpoint.origin())
            .arg("--account")
            .arg(uuid::Uuid::from_bytes(ai).to_string())
            .arg("--token-file")
            .arg(&token_file)
            .arg("--state")
            .arg(&state)
            .arg("--accept-host")
            .arg(host)
            .arg("--games")
            .arg("1")
            .args(&self.args);
        let mut runner = Running(command.spawn().expect("the AI runner starts"));
        let (match_id, played, receipt) = rt.block_on(async {
            tokio::time::timeout(
                Duration::from_mins(15),
                person(person_account, &transcripts, deck, def),
            )
            .await
            .expect("the person's game ends")
        });
        // The runner ends by itself once its one game settled.
        let until = std::time::Instant::now() + Duration::from_mins(4);
        let status = loop {
            if let Some(status) = runner.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                std::time::Instant::now() < until,
                "the AI runner did not stop after its game"
            );
            std::thread::sleep(Duration::from_millis(500));
        };
        assert!(status.success(), "the AI runner ended with {status}");
        let path = state.join("transcripts").join(format!(
            "ranked-{}-{}.jsonl",
            uuid::Uuid::from_bytes(match_id),
            def.name
        ));
        let seen = Seen::read(&path);
        let QueueState::Finished {
            seat,
            outcome,
            opponent_ai,
            ..
        } = &receipt
        else {
            panic!("the person's receipt: {receipt:?}");
        };
        assert!(opponent_ai.is_some(), "the receipt names the AI as an AI");
        let person_won = match played.result.and_then(|r| r.winner) {
            Some(Victor::Player(winner)) => Some(winner == PlayerId::new(*seat)),
            _ => None,
        };
        let central = match outcome {
            baylee_account::ranked::Outcome::FirstWins => Some(*seat == 0),
            baylee_account::ranked::Outcome::SecondWins => Some(*seat == 1),
            baylee_account::ranked::Outcome::Draw => None,
        };
        assert_eq!(
            person_won, central,
            "the table and Central agree on the result"
        );
        let line = seen.line(&def.profile, &played, 1 - *seat);
        // Past the test harness's capture, which keeps a passing test's
        // prints, and kept beside the transcripts.
        {
            use std::io::Write as _;
            let _ = writeln!(std::io::stdout(), "{line}");
        }
        std::fs::write(transcripts.join("summary.txt"), format!("{line}\n")).unwrap();
        assert!(
            seen.ended.contains("GameResult"),
            "the AI seat saw the game's end"
        );
    }
}

/// What the AI seat's transcript says of its game. Its answers are told
/// apart: those its pilot's mind gave, whatever mind it is (a model's call,
/// the house's own choice), with their think time; those the standing
/// orders gave with no mind asked; those the declared fallback gave after
/// the pilot handed the seat over (the transcript's `handed_over` line);
/// and those the seat's own fallback gave before any hand-over, where the
/// pilot's mind failed or ran out of time (the bridge's house or least
/// answer).
struct Seen {
    turns: u64,
    ended: String,
    expired: u64,
    /// The pilot's mind's think times, in milliseconds, rising.
    decided: Vec<u64>,
    instant: usize,
    house: usize,
    fallback: usize,
}

impl Seen {
    fn read(path: &std::path::Path) -> Self {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("the AI's transcript {}: {e}", path.display()));
        let mut seen = Self {
            turns: 0,
            ended: String::new(),
            expired: 0,
            decided: Vec::new(),
            instant: 0,
            house: 0,
            fallback: 0,
        };
        let mut handed_over = false;
        for line in text.lines() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            handed_over |= value.get("handed_over").is_some();
            match (value["event"].as_str(), value["by"].as_str()) {
                (Some("over"), _) => value["result"]
                    .as_str()
                    .unwrap_or_default()
                    .clone_into(&mut seen.ended),
                (Some("standing"), _) => seen.instant += 1,
                (Some("answered"), _) if handed_over => seen.house += 1,
                (Some("answered"), Some("mind")) => {
                    seen.decided.push(value["model_ms"].as_u64().unwrap_or(0));
                }
                (Some("answered"), Some(_)) => seen.fallback += 1,
                _ => {}
            }
            if let Some(summary) = value.get("summary") {
                seen.turns = summary["turns"].as_u64().unwrap_or(seen.turns);
                seen.expired = summary["fallbacks"]["expired"]
                    .as_u64()
                    .unwrap_or(seen.expired);
            }
        }
        seen.decided.sort_unstable();
        seen
    }

    /// The summary line: from the AI's side (it plays `ai_seat`), how the
    /// person's table `played` it ended, and how the AI answered.
    fn line(&self, profile: &str, played: &Played, ai_seat: u8) -> String {
        let said = match played.result.and_then(|r| r.winner) {
            Some(Victor::Player(winner)) if winner == PlayerId::new(ai_seat) => "AI won",
            Some(_) => "AI lost",
            None => "draw",
        };
        let reason = played
            .result
            .map_or_else(|| "no end".to_owned(), |r| format!("{:?}", r.reason));
        format!(
            "ai-players e2e: {profile} {said} as seat {ai_seat} after {} turns ({reason}); {} \
             questions expired; decided {} (median {} ms, P95 {} ms, max {} ms), instant {}, \
             house {}, fallback {}",
            self.turns,
            self.expired,
            self.decided.len(),
            self.at(0.5),
            self.at(0.95),
            self.decided.last().copied().unwrap_or(0),
            self.instant,
            self.house,
            self.fallback
        )
    }

    /// The decided think time at the share `q` of them (0.5 the median).
    fn at(&self, q: f64) -> u64 {
        if self.decided.is_empty() {
            return 0;
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let at = ((self.decided.len() - 1) as f64 * q).round() as usize;
        self.decided[at.min(self.decided.len() - 1)]
    }
}

/// Where every question and answer of both seats goes, emptied: for a run
/// that fails or ends sooner than a whole game,
/// `<target>/tmp/ai-players-runners/`.
fn fresh_transcripts() -> std::path::PathBuf {
    let transcripts = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ai-players-runners");
    let _ = std::fs::remove_dir_all(&transcripts);
    std::fs::create_dir_all(&transcripts).unwrap();
    transcripts
}

#[test]
#[ignore = "requires owned PostgreSQL 18 fixture from cargo dev central integration --ai-players-only"]
fn two_ai_runners_meet_in_the_carousel_and_finish_a_ranked_game() {
    super::super::integration(super::super::Scenario::Carousel);
}

/// Two house AIs, each its own runner, with fill-in off and the carousel on
/// (calibration): the carousel pairs their standing offers in Standard,
/// they finish the game, and Central's results for both agree with the
/// table's.
pub(in super::super) fn carousel(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
) {
    let _owner = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        "carousel_host",
        fresh::DOCUMENT,
    );
    let host = registered(rt, served, store, operator, "carousel_host", true);
    let agent = rt.block_on(common::ranked::provision(&host.gateway));
    capacity(rt, operator, &host);
    let desk = Desk::open(rt, served, operator, "carousel_admin");
    ai_settings(&desk, false);
    let names = ["KI-Haus-Eins", "KI-Haus-Zwei"];
    let ais = names.map(|name| {
        create_ai(
            &desk,
            &AiDef {
                name,
                ..AiDef::house()
            },
        )
    });
    let token = runner_token(&desk, ais.iter().map(|(account, _)| *account).collect());
    let transcripts = fresh_transcripts();
    let args = [0, 1].map(|at| {
        let (state, token_file) = state_named(&format!("ai-players-carousel-{at}"), &token, None);
        RankedArgs {
            central: served.endpoint.origin().to_owned(),
            account: format!("{}#{:04x}", names[at], ais[at].1),
            token_file: Some(token_file),
            token_key: None,
            state: Some(state),
            decks: None,
            transcripts: Some(transcripts.clone()),
            fallback_level: "expert".into(),
            games: Some(1),
            accept_hosts: vec![host.origin.clone()],
            facts_from: None,
            facts_file: None,
            min_think_ms: 0,
            no_carousel: false,
        }
    });
    let reports = rt.block_on(async {
        let fly = |args: RankedArgs| async move {
            let mut reports = Vec::new();
            let pilot: Arc<dyn Pilot> = Arc::new(HousePilot::new("expert").unwrap());
            let played = cli::play(&args, pilot, |r| reports.push(r))
                .await
                .expect("the runner plays");
            assert_eq!(played, 1, "each runner played one match");
            reports
                .pop()
                .expect("the runner told how its match went")
                .expect("the runner played its match to the end")
        };
        let [one, two] = args;
        tokio::time::timeout(Duration::from_secs(540), async {
            tokio::join!(fly(one), fly(two))
        })
        .await
        .expect("both AIs finish their carousel game")
    });
    let (one, two) = reports;
    assert_eq!(
        one.match_id, two.match_id,
        "the carousel paired the two AIs"
    );
    let receipts = [&one, &two].map(carousel_receipt);
    assert_ne!(receipts[0].seat, receipts[1].seat);
    assert_eq!(receipts[0].outcome, receipts[1].outcome);
    agent.abort();
}

/// One carousel AI's whole game, Central's result for it (not inferred),
/// and the two agreeing.
fn carousel_receipt(report: &MatchReport) -> ranked::Receipt {
    let receipt = report.receipt.expect("Central settled the match");
    assert!(
        !receipt.inferred,
        "Central's own result, not the standing's"
    );
    let result = report
        .played
        .result
        .expect("the seat saw the game's end on its socket");
    let won = match result.winner {
        Some(Victor::Player(winner)) => Some(winner == PlayerId::new(receipt.seat)),
        Some(Victor::Team(_)) => panic!("a duel has no teams"),
        None => None,
    };
    assert_eq!(
        won,
        receipt.won(),
        "the table and Central agree on the result"
    );
    assert!(
        report.played.stats.turns >= 100,
        "a whole game, not a forfeit: {} turns, {result:?}",
        report.played.stats.turns
    );
    assert_eq!(
        report.played.stats.fallbacks.expired, 0,
        "no question ran out of time"
    );
    assert_eq!(
        report.opponent.as_deref(),
        Some("Haus-KI · Expert"),
        "each was told its opponent is an AI"
    );
    receipt
}
