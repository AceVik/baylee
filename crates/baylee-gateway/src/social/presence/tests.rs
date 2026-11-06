//! One walk of the lobby against the walk per account it replaced, which
//! stays here as the reading it has to equal.

use super::*;
use crate::lobby::{LobbySeat, SeatKind};

type Watching = BTreeMap<String, BTreeMap<String, usize>>;

/// Where `account` is, by asking every game whether it sits there.
fn place_walking_every_game(
    lobby: &Lobby,
    watching: &Watching,
    account: &str,
    activity: Option<Activity>,
    share_table: bool,
    now: u64,
) -> Place {
    let mut room = None;
    for game in lobby.games.values() {
        if !seated(game, None).contains(account) {
            continue;
        }
        let locked = game.password_hash.is_some();
        let shared = (share_table && !locked).then(|| game.id.clone());
        let seats = u8::try_from(game.seats.len()).unwrap_or(u8::MAX);
        match game.state {
            LobbyState::Playing => {
                return Place::Playing {
                    game: shared,
                    name: game.name.clone(),
                    seats,
                    running_secs: game.started_at.map_or(0, |t| {
                        u32::try_from(now.saturating_sub(t)).unwrap_or(u32::MAX)
                    }),
                };
            }
            LobbyState::Waiting => {
                room = Some(Place::Room {
                    game: shared,
                    name: game.name.clone(),
                    seats,
                    taken: u8::try_from(game.seats.iter().filter(|s| s.occupied()).count())
                        .unwrap_or(u8::MAX),
                    locked,
                });
            }
            LobbyState::Over => {}
        }
    }
    room.unwrap_or_else(|| {
        place(
            Seating::default(),
            lobby,
            watching,
            account,
            activity,
            share_table,
            now,
        )
    })
}

fn table(id: &str, state: LobbyState, accounts: &[&str]) -> LobbyGame {
    let mut game = LobbyGame::blank(id.to_owned(), 10);
    game.state = state;
    game.name = format!("table {id}");
    game.started_at = Some(20);
    game.seats = accounts
        .iter()
        .enumerate()
        .map(|(seat, account)| {
            let mut chair = LobbySeat::open(seat);
            chair.account_id = Some((*account).to_owned());
            chair
        })
        .collect();
    game
}

fn lobby_of(games: Vec<LobbyGame>) -> Lobby {
    let mut lobby = Lobby::default();
    for game in games {
        lobby.games.insert(game.id.clone(), game);
    }
    lobby
}

/// Every shape an account's seats can take at once, with the chairs that must
/// not count among them.
fn tables() -> Vec<LobbyGame> {
    let mut locked = table("locked", LobbyState::Waiting, &["locked"]);
    locked.password_hash = Some("hash".to_owned());
    let mut house = table("house", LobbyState::Playing, &["house"]);
    house.seats[0].kind = SeatKind::Ai;
    vec![
        table("playing", LobbyState::Playing, &["one", "both"]),
        table("second", LobbyState::Playing, &["two-games"]),
        table("third", LobbyState::Playing, &["two-games"]),
        table("room", LobbyState::Waiting, &["waiting", "both"]),
        table(
            "room-a",
            LobbyState::Waiting,
            &["two-rooms", "twice", "twice"],
        ),
        table("room-b", LobbyState::Waiting, &["two-rooms"]),
        table("over", LobbyState::Over, &["over"]),
        locked,
        house,
    ]
}

#[test]
fn one_walk_answers_as_a_walk_per_account_did() {
    let lobby = lobby_of(tables());
    let watching: Watching = [(
        "watcher".to_owned(),
        [("playing".to_owned(), 1)].into_iter().collect(),
    )]
    .into_iter()
    .collect();
    let seatings = Seating::of_everyone(&lobby);
    let accounts = [
        "one",
        "both",
        "two-games",
        "waiting",
        "two-rooms",
        "twice",
        "over",
        "locked",
        "house",
        "watcher",
        "nobody",
    ];
    for account in accounts {
        for share_table in [true, false] {
            for activity in [None, Some(Activity::Building), Some(Activity::House)] {
                let then =
                    place_walking_every_game(&lobby, &watching, account, activity, share_table, 30);
                let everyone = seatings.get(account).copied().unwrap_or_default();
                let alone = Seating::of(&lobby, account);
                for seating in [everyone, alone] {
                    let now = place(
                        seating,
                        &lobby,
                        &watching,
                        account,
                        activity,
                        share_table,
                        30,
                    );
                    assert_eq!(now, then, "{account} sharing {share_table}");
                }
            }
        }
    }
}

/// What a pass holds the lobby lock for, by hand:
/// `cargo test --release -p baylee-gateway presence_pass -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run by hand in --release"]
fn presence_pass_under_the_lobby_lock() {
    const GAMES: usize = 1000;
    let accounts: Vec<String> = (0..2 * GAMES).map(|i| format!("account-{i}")).collect();
    let games = (0..GAMES)
        .map(|i| {
            let (a, b) = (accounts[2 * i].as_str(), accounts[2 * i + 1].as_str());
            table(&format!("game-{i}"), LobbyState::Playing, &[a, b])
        })
        .collect();
    let lobby = lobby_of(games);
    let watching = Watching::new();
    let walking = std::time::Instant::now();
    for account in &accounts {
        std::hint::black_box(place_walking_every_game(
            &lobby, &watching, account, None, true, 30,
        ));
    }
    let walking = walking.elapsed();
    let once = std::time::Instant::now();
    let seatings = Seating::of_everyone(&lobby);
    for account in &accounts {
        let seating = seatings.get(account.as_str()).copied().unwrap_or_default();
        std::hint::black_box(place(seating, &lobby, &watching, account, None, true, 30));
    }
    let once = once.elapsed();
    println!(
        "{} online, {GAMES} games: a walk per account {walking:?}, one walk {once:?}",
        accounts.len()
    );
}
