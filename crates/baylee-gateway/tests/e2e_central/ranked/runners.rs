//! Two AI accounts, each a ranked runner with a fair house mind
//! (`baylee_seat::ranked`), find each other in Central's queue and finish a
//! ranked game on a verified host, through Central's relay, as two people's
//! native clients would (`docs/llm-seat.md` §"The ranked runner"). Nothing
//! here drives the queue, the seat or the socket for them, except that the
//! first runner's first socket is cut mid-game: it must take its seat again
//! with the game's id and play on.
//!
//! Run it from a clean commit: the host's engine advertises a ranked
//! compatibility key only for a clean, stamped build, and without one
//! Central finds no host for the match ("no opponent").
use super::{
    AccountStore, DatabaseConnection, Format, Runtime, Served, capacity, common, fresh, registered,
};
use baylee_client_core::reconnect::Window;
use baylee_core::ids::PlayerId;
use baylee_engine::win::Victor;
use baylee_seat::bridge::PlayOptions;
use baylee_seat::link::{Reporter, Socket, Table};
use baylee_seat::ranked::{
    self, Assignments as _, Credentials, Frozen, LinkOptions, MatchOptions, MatchReport, Pilot,
    Player, Queue, RankedLink, Settings, play_through,
};
use futures_util::{SinkExt as _, StreamExt as _};
use std::sync::Arc;
use std::time::Duration;

#[test]
#[ignore = "requires owned PostgreSQL 18 fixture from cargo dev central integration --ai-players-only"]
fn house_runners_find_each_other_and_finish_a_ranked_game() {
    super::super::integration(super::super::Scenario::Runners);
}

/// An AI account at Central, labelled as the house in its public name,
/// with one private Forest deck; the fixture's own session is ended, so
/// the runner's sign-in is the account's only one.
fn ai_account(rt: &Runtime, served: &Served, username: &str, name: &str) -> uuid::Uuid {
    let mut account = fresh::register_as(
        rt,
        &served.service,
        &served.endpoint,
        username,
        Some(name),
        fresh::DOCUMENT,
    );
    let deck = account
        .save_deck(
            None,
            None,
            baylee_account::decks::DeckDraft {
                name: "Forest".into(),
                format: "standard".into(),
                cards: vec!["60 Forest".into()],
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    account.logout().unwrap();
    deck
}

/// A ranked link whose first socket goes through a loopback pair that the
/// test cuts after `frames` frames from the table: the bridge sees its
/// socket drop mid-game, as a network would drop it.
struct Cutting {
    link: RankedLink,
    frames: Option<usize>,
}

impl Table for Cutting {
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

/// The first runner: its source and the match played by hand, so the match
/// goes through [`Cutting`].
async fn cut_once(
    player: &Player,
    queue: &mut Queue,
    pilot: &Pilot,
    options: &MatchOptions,
) -> MatchReport {
    let game = loop {
        if let Some(game) = queue.live().await.unwrap().pop() {
            break game;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    let match_id = game.assignment.match_id;
    let mut link = Cutting {
        link: RankedLink::new(
            player.clone(),
            game.key,
            game.assignment,
            Arc::clone(&game.deck),
            options.link.clone(),
        ),
        frames: Some(40),
    };
    let (played, receipt) = play_through(player, &mut link, match_id, &game.deck, pilot, options)
        .await
        .unwrap();
    MatchReport {
        match_id,
        played,
        receipt,
        renewals: link.link.renewals(),
    }
}

pub(in super::super) fn run(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
) {
    // The host belongs to a third account: neither AI owns where it plays.
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
    let accounts = [("house_one", "HOUSE-One"), ("house_two", "HOUSE-Two")]
        .map(|(username, name)| (username, ai_account(rt, served, username, name)));
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
            transcripts: None,
        },
    };
    let pilot = Pilot::house("expert").unwrap();
    let [one, two] = rt.block_on(async {
        let play = |(username, deck): (&'static str, uuid::Uuid), cut: bool| {
            let (settings, pilot) = (settings.clone(), pilot.clone());
            let endpoint = served.endpoint.clone();
            async move {
                let credentials = Credentials::new(endpoint, username.into(), || {
                    Ok(zeroize::Zeroizing::new(fresh::password().to_owned()))
                });
                let player = Player::sign_in(credentials, pilot.disclosure)
                    .await
                    .unwrap();
                let alive = player.keep_alive(Duration::from_secs(60));
                let deck = Frozen::load(&player, deck, Format::Standard).await.unwrap();
                let mut queue = Queue::new(
                    player.clone(),
                    Arc::new(deck),
                    Some(Duration::from_secs(120)),
                );
                let report = if cut {
                    cut_once(&player, &mut queue, &pilot, &settings.play).await
                } else {
                    let mut reports = Vec::new();
                    let played =
                        ranked::run(&player, &mut queue, &settings, &pilot, |r| reports.push(r))
                            .await
                            .unwrap();
                    assert_eq!(played, 1);
                    assert_eq!(reports.len(), 1);
                    reports
                        .remove(0)
                        .expect("the runner played its match to the end")
                };
                drop((alive, queue));
                player.sign_out().await;
                report
            }
        };
        let (one, two) = tokio::time::timeout(Duration::from_secs(540), async {
            tokio::join!(play(accounts[0], true), play(accounts[1], false))
        })
        .await
        .expect("both runners finish their ranked game");
        [one, two]
    });
    agreed(&one, &two);
    agent.abort();
}

/// Both runners saw a whole game to its end, the first through a socket it
/// lost and took again, and they and Central agree on the result.
fn agreed(one: &MatchReport, two: &MatchReport) {
    assert_eq!(one.match_id, two.match_id, "the two runners met each other");
    for report in [one, two] {
        let result = report
            .played
            .result
            .expect("the seat saw the game's end on its socket");
        let receipt = report.receipt.expect("Central settled the match");
        let won = match result.winner {
            Some(Victor::Player(seat)) => Some(seat == PlayerId::new(receipt.seat)),
            Some(Victor::Team(_)) => panic!("a duel has no teams"),
            None => None,
        };
        assert_eq!(
            won,
            receipt.won(),
            "the table and Central agree on the result"
        );
        let stats = &report.played.stats;
        // Two decks of sixty Forests end when a library runs out: a game of
        // fifty turns or more, and no question left to its clock.
        assert!(
            stats.turns >= 50,
            "a whole game, not a forfeit: {} turns, {result:?}",
            stats.turns
        );
        assert_eq!(stats.fallbacks.expired, 0, "no question ran out of time");
    }
    assert!(
        one.played.dials >= 2,
        "the cut socket was replaced: {} sockets",
        one.played.dials
    );
    let receipts = [one.receipt.unwrap(), two.receipt.unwrap()];
    assert_ne!(receipts[0].seat, receipts[1].seat);
    assert_eq!(receipts[0].outcome, receipts[1].outcome);
    assert!(
        one.renewals + two.renewals > 0,
        "the admission was renewed during the game"
    );
}
