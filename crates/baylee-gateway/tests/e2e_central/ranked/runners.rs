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
use baylee_seat::ranked::{
    self, Flight, Frozen, HousePilot, LinkOptions, MatchOptions, MatchReport, Pilot, Piloted,
    Player, RunnerToken, Seat, Settings, Shelf, Stop,
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

/// The AI as an administrator sets it up: fill-in after Central's shortest
/// wait, no host slots held back for people (the fixture's host has few),
/// the AI created, switched on, and a runner token for it alone.
fn set_up_ai(desk: &Desk<'_>) -> (Id, Token) {
    assert!(matches!(
        desk.ask(AdminRequest::SetAiSettings {
            settings: AiSettings {
                fill_in_after: 30,
                drain_minutes: 30,
                global_seats: 8,
                host_reserve: 0,
                carousel_games: 0,
                carousel_daily: 0,
            },
        }),
        Answer::Done
    ));
    let Answer::Admin {
        answer: AdminAnswer::AiPlayerCreated { account, .. },
    } = desk.ask(AdminRequest::CreateAiPlayer {
        settings: AiPlayerSettings {
            kind: AiKind::House,
            profile: "house:expert".into(),
            label: "Haus-KI · Expert".into(),
            max_games: 1,
            daily_games: None,
            display_name: "KI-Haus-Expert".into(),
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
    let Answer::Admin {
        answer: AdminAnswer::AiRunnerTokenCreated { token, .. },
    } = desk.ask(AdminRequest::CreateAiRunnerToken {
        name: "e2e runner".into(),
        accounts: vec![account],
        days: 1,
    })
    else {
        panic!("a runner token");
    };
    (account, token)
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
async fn person(account: Account, transcripts: &std::path::Path) -> (Id, Played, QueueState) {
    let deck = Arc::new(forests());
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
                ai: AiOpponents::Local,
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
                assert_eq!(badge.kind, AiKind::House);
                assert_eq!(badge.label, "Haus-KI · Expert");
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
    let (ai, token) = set_up_ai(&desk);
    let person_account = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        "runner_person",
        fresh::DOCUMENT,
    );
    // Every question and answer of both seats, for a run that fails or
    // ends sooner than a whole game: `<target>/tmp/ai-players-runners/`.
    let transcripts = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ai-players-runners");
    let _ = std::fs::remove_dir_all(&transcripts);
    std::fs::create_dir_all(&transcripts).unwrap();
    let settings = Settings {
        games: Some(1),
        poll: Duration::from_millis(500),
        play: MatchOptions {
            play: PlayOptions {
                min_think: Duration::ZERO,
                ..PlayOptions::default()
            },
            link: LinkOptions {
                accept_hosts: vec![host.origin.clone()],
                // A renewal within the game, whatever `renew_in` says.
                renew_cap: Some(Duration::from_secs(2)),
                ..LinkOptions::default()
            },
            settle: Duration::from_secs(120),
            transcripts: Some(transcripts.clone()),
        },
        ..Settings::default()
    };
    let secret = Token::from_secret_bytes(token.to_secret_bytes());
    let runner_token =
        RunnerToken::new(move || Ok(Token::from_secret_bytes(secret.to_secret_bytes())));
    let (report, person) = rt.block_on(async {
        let player = Player::open(served.endpoint.clone(), ai, runner_token)
            .await
            .expect("the runner trades its token for a session of the AI");
        assert_eq!(player.name(), "KI-Haus-Expert");
        let alive = player.keep_alive(Duration::from_secs(60));
        let mut shelf = Shelf::fixed([forests()]).unwrap();
        let pilot: Arc<dyn Pilot> = Arc::new(CuttingHouse {
            house: HousePilot::new("expert").unwrap(),
            frames: Mutex::new(Some(40)),
        });
        let fallback: Arc<dyn Pilot> = Arc::new(HousePilot::new("expert").unwrap());
        let (_stop, stopping) = tokio::sync::watch::channel(Stop::Run);
        let mut reports = Vec::new();
        let ((played, report), person) = tokio::time::timeout(Duration::from_secs(540), async {
            tokio::join!(
                async {
                    let played = ranked::run(
                        &player,
                        &mut shelf,
                        pilot,
                        fallback,
                        &settings,
                        stopping,
                        |r| reports.push(r),
                    )
                    .await
                    .unwrap();
                    (played, reports.pop())
                },
                person(person_account, &transcripts)
            )
        })
        .await
        .expect("both sides finish their ranked game");
        assert_eq!(played, 1, "the runner played one match");
        // The runner withdrew its offer when it stopped.
        let after = player
            .call(|account| account.ai(baylee_account::ranked::ai::Request::Work))
            .await
            .unwrap();
        drop(alive);
        player.sign_out().await;
        let report = report
            .expect("the runner told how its match went")
            .expect("the runner played its match to the end");
        let baylee_account::ranked::ai::Response::Work { assignments, .. } = after else {
            panic!("work");
        };
        assert!(assignments.is_empty(), "no live match is left");
        (report, person)
    });
    agreed(&report, &person);
    agent.abort();
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
    assert!(
        ai.renewals > 0,
        "the AI's admission was renewed during the game"
    );
    let why = ai
        .handed_over
        .as_deref()
        .expect("the pilot handed the seat to the house after the cut");
    assert!(why.contains("cannot resume"), "{why}");
}
