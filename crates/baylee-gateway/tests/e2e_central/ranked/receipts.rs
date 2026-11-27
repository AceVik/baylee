//! Wait for the gateway's own outbox before exercising replay against Central.
use super::{
    Assignment, CentralHttp, Format, Outcome, Player, QueueState, Registered, Request, Response,
    Served, Uuid, sdk,
};
use baylee_account::ranked::{HostRequest, HostResponse, League, ProfileTarget, Record, Track};
use std::time::{Duration, Instant};

pub(super) fn receipt(player: &Player) -> QueueState {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        match player.account.ranked(Request::Status).unwrap() {
            Response::Queue {
                state: state @ QueueState::Finished { .. },
            } => return state,
            Response::Queue {
                state: QueueState::Assigned { .. },
            } if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(250));
            }
            _ => panic!("the host outbox must settle the assigned match"),
        }
    }
}

fn profile(served: &Served, assignment: &Assignment) -> Response {
    baylee_account::ranked_read(
        &served.endpoint,
        Request::Profile {
            target: ProfileTarget::Account(assignment.account),
            season: None,
        },
    )
    .unwrap()
}

pub(super) fn before(served: &Served, players: &[Player; 2]) {
    assert!(
        baylee_account::ranked_read(&served.endpoint, Request::OwnProfile { season: None })
            .is_err()
    );
    let first = players[0].account.public_id();
    assert!(
        baylee_account::ranked_read(
            &served.endpoint,
            Request::Profile {
                target: ProfileTarget::Account(first),
                season: None,
            }
        )
        .is_err(),
        "a new unnamed account has no public profile before playing"
    );
    for player in players {
        let reply = player
            .account
            .ranked(Request::OwnProfile { season: None })
            .unwrap();
        let Response::Profile {
            account,
            formats,
            name,
            awards,
            ..
        } = reply
        else {
            panic!("private initial standing")
        };
        assert_eq!(account, player.account.public_id());
        assert_eq!(name.is_none(), account == first);
        assert!(awards.is_empty());
        for format in Format::ALL {
            let row = formats
                .iter()
                .find(|s| s.format == format && s.track == Track::Player)
                .expect("every format has a placement baseline");
            assert_eq!(
                (row.rank.league, row.placement_games, row.career),
                (League::Unranked, 0, Record::default())
            );
        }
    }
}

fn verify_boards(served: &Served, profiles: &[Response; 2]) {
    for track in [Track::Player, Track::Deck] {
        let reply = baylee_account::ranked_read(
            &served.endpoint,
            Request::Board {
                season: None,
                format: Format::Standard,
                track,
                league: League::Unranked,
                division: None,
                after: None,
                players: baylee_account::ranked::ai::BoardPlayers::Everyone,
            },
        )
        .unwrap();
        let Response::Board { entries, next, .. } = reply else {
            panic!("public placement ladder")
        };
        assert_eq!(entries.len(), 2);
        assert!(next.is_none());
        for profile in profiles {
            let Response::Profile {
                account, formats, ..
            } = profile
            else {
                unreachable!()
            };
            let row = entries
                .iter()
                .find(|row| row.account == *account)
                .expect("both players appear publicly");
            assert!(formats.contains(&row.standing));
        }
    }
}

fn verify_player(
    player: &Player,
    assignment: &Assignment,
    receipt: &QueueState,
    profile: &Response,
    outcome: Outcome,
    won: bool,
) {
    let QueueState::Finished {
        key,
        match_id,
        seat,
        format,
        elo,
        delta,
        before,
        after,
        outcome: confirmed,
        deck,
        opponent_ai,
    } = receipt
    else {
        unreachable!()
    };
    assert_eq!(*opponent_ai, None, "two people met: no AI badge");
    assert_eq!(
        (*key, *match_id, *seat, *format, *confirmed),
        (
            player.key,
            assignment.match_id,
            assignment.seat,
            Format::Standard,
            outcome
        )
    );
    assert_eq!(
        (
            before.league,
            after.league,
            deck.before.league,
            deck.after.league
        ),
        (
            League::Unranked,
            League::Unranked,
            League::Unranked,
            League::Unranked
        )
    );
    assert_eq!(delta.signum(), if won { 1 } else { -1 });
    assert_eq!(deck.delta.signum(), delta.signum());
    assert_eq!(deck.placement_games, 1);
    let Response::Profile {
        account,
        formats,
        awards,
        ..
    } = profile
    else {
        panic!("published profile")
    };
    assert_eq!(*account, assignment.account);
    assert!(awards.is_empty(), "no badge before placements finish");
    let expected = Record {
        games: 1,
        wins: u32::from(won),
        losses: u32::from(!won),
        draws: 0,
    };
    for (track, rating) in [(Track::Player, *elo), (Track::Deck, deck.elo)] {
        let row = formats
            .iter()
            .find(|s| s.format == Format::Standard && s.track == track)
            .expect("settled ladder standing");
        assert_eq!(
            (row.placement_games, row.rank.league, row.elo),
            (1, League::Unranked, rating)
        );
        assert_eq!(row.career, expected);
        assert_eq!(
            (row.games, row.wins, row.losses, row.draws),
            (
                expected.games,
                expected.wins,
                expected.losses,
                expected.draws
            )
        );
        assert_eq!(row.deck.is_some(), track == Track::Deck);
        assert_ne!(
            row.deck,
            Some(player.commitment),
            "a public deck identifier is opaque"
        );
    }
    assert!(
        formats
            .iter()
            .filter(|s| s.format != Format::Standard)
            .all(|s| s.career.games == 0)
    );
    let public = serde_json::to_string(profile).unwrap();
    assert!(!public.contains("Forest") && !public.contains("Private forest"));
}

pub(super) fn settled(
    served: &Served,
    players: &[Player; 2],
    assignments: &[Assignment; 2],
    outcome: Outcome,
) {
    let receipts = players.each_ref().map(receipt);
    let profiles = assignments.each_ref().map(|a| profile(served, a));
    for i in 0..2 {
        verify_player(
            &players[i],
            &assignments[i],
            &receipts[i],
            &profiles[i],
            outcome,
            i == 1,
        );
    }
    assert!(baylee_account::ranked_read(&served.endpoint, Request::Status).is_err());
    verify_boards(served, &profiles);
    for i in 0..2 {
        assert_eq!(receipt(&players[i]), receipts[i]);
        assert_eq!(profile(served, &assignments[i]), profiles[i]);
        assert_eq!(
            players[i]
                .account
                .ranked(Request::OwnProfile { season: None })
                .unwrap(),
            profiles[i]
        );
    }
}

pub(super) fn verify(
    served: &Served,
    home: &Registered,
    host: &Registered,
    players: &[Player; 2],
    assignments: &[Assignment; 2],
    game: Uuid,
    outcome: Outcome,
) {
    // First wait for the actual host delivery, then replay once and check
    // both player and deck game counts, private receipts, and public boards.
    let confirmed = players.each_ref().map(receipt);
    let http = CentralHttp::new(served.endpoint.clone());
    let finish = HostRequest::Finish {
        match_id: assignments[0].match_id,
        game: *game.as_bytes(),
        outcome,
    };
    assert!(sdk::ranked::host(&http, &home.token, finish.clone()).is_err());
    assert_eq!(
        sdk::ranked::host(&http, &host.token, finish).unwrap(),
        HostResponse::Done
    );
    assert_eq!(players.each_ref().map(receipt), confirmed);
    settled(served, players, assignments, outcome);
}
