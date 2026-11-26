//! Two AI accounts, each a ranked runner with a fair house mind
//! (`baylee_seat::ranked`), find each other in Central's queue and finish a
//! ranked game on a verified host, through Central's relay, as two people's
//! native clients would (`docs/llm-seat.md` §"The ranked runner"). Nothing
//! here drives the queue, the seat or the socket for them.
use super::{
    AccountStore, DatabaseConnection, Format, Runtime, Served, capacity, common, fresh, registered,
};
use baylee_core::ids::PlayerId;
use baylee_engine::win::Victor;
use baylee_seat::bridge::PlayOptions;
use baylee_seat::ranked::{
    self, Credentials, Frozen, LinkOptions, MatchOptions, MatchReport, Pilot, Player, Queue,
    Settings,
};
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
        ..Settings::default()
    };
    let pilot = Pilot::house("expert").unwrap();
    let reports = rt.block_on(async {
        let play = |(username, deck): (&'static str, uuid::Uuid)| {
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
                let mut reports = Vec::new();
                let played =
                    ranked::run(&player, &mut queue, &settings, &pilot, |r| reports.push(r))
                        .await
                        .unwrap();
                assert_eq!(played, 1);
                drop((alive, queue));
                player.sign_out().await;
                reports
            }
        };
        tokio::time::timeout(Duration::from_secs(420), async {
            tokio::join!(play(accounts[0]), play(accounts[1]))
        })
        .await
        .expect("both runners finish their ranked game")
    });
    let [one, two]: [MatchReport; 2] = [reports.0, reports.1].map(|mut r| {
        assert_eq!(r.len(), 1);
        r.remove(0).expect("the runner played its match to the end")
    });
    assert_eq!(one.match_id, two.match_id, "the two runners met each other");
    for report in [&one, &two] {
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
        assert!(report.played.dials >= 1);
        assert!(report.played.stats.turns > 0);
    }
    let receipts = [one.receipt.unwrap(), two.receipt.unwrap()];
    assert_ne!(receipts[0].seat, receipts[1].seat);
    assert_eq!(receipts[0].outcome, receipts[1].outcome);
    assert!(
        one.renewals + two.renewals > 0,
        "the admission was renewed during the game"
    );
    agent.abort();
}
