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

fn place_of(lobby: &Lobby, watching: &Watching, account: &str, share: bool) -> Place {
    place(
        Seating::of(lobby, account),
        lobby,
        watching,
        account,
        None,
        share,
        50,
    )
}

fn game_of(place: &Place) -> Option<&str> {
    match place {
        Place::Playing { game, .. } | Place::Room { game, .. } | Place::Watching { game, .. } => {
            game.as_deref()
        }
        _ => panic!("not at a table"),
    }
}

/// A table's id is what a friend joins or watches by: it is told only by a
/// player who shares it, and never of a table behind a password.
#[test]
fn a_table_is_named_only_by_who_shares_it_and_never_behind_a_password() {
    let mut locked_game = table("locked-game", LobbyState::Playing, &["p"]);
    locked_game.password_hash = Some("hash".into());
    let mut games = tables();
    games.push(locked_game);
    let lobby = lobby_of(games);
    let watching = Watching::from([
        ("w".into(), BTreeMap::from([("playing".into(), 1)])),
        ("lw".into(), BTreeMap::from([("locked-game".into(), 1)])),
    ]);
    for (account, game) in [("one", "playing"), ("waiting", "room"), ("w", "playing")] {
        let shared = place_of(&lobby, &watching, account, true);
        assert_eq!(game_of(&shared), Some(game), "{account}");
        let kept = place_of(&lobby, &watching, account, false);
        assert_eq!(game_of(&kept), None, "{account}");
    }
    for account in ["locked", "p", "lw"] {
        for share in [true, false] {
            let place = place_of(&lobby, &watching, account, share);
            assert_eq!(game_of(&place), None, "{account}");
        }
    }
}

#[test]
fn an_invisible_player_is_offline_wherever_they_are() {
    let lobby = lobby_of(tables());
    let place = place_of(&lobby, &Watching::new(), "one", true);
    assert_eq!(
        shown(Status::Invisible, place.clone()),
        Presence::offline(),
        "not even the table"
    );
    for (status, want) in [
        (Status::Online, Shown::Online),
        (Status::Away, Shown::Away),
        (Status::Dnd, Shown::Dnd),
    ] {
        let presence = shown(status, place.clone());
        assert_eq!(presence.status, want);
        assert_eq!(presence.place.as_ref(), Some(&place));
    }
}

fn online(account: &str, friends: &[&str]) -> OnlineView {
    OnlineView {
        account: account.into(),
        me: Person {
            tag: format!("t-{account}"),
            handle: account.into(),
            guest: false,
        },
        status: Status::Online,
        activity: None,
        share_table: true,
        friends: friends.iter().map(|f| (*f).to_owned()).collect(),
    }
}

#[test]
fn a_running_clock_alone_is_no_new_presence() {
    let social = Social::new(true, 30);
    let lobby = lobby_of(tables());
    let published = |social: &Social| social.published.lock().shown["one"].clone();
    let at = |now: u64| presences(&lobby, vec![online("one", &[])], &Watching::new(), now);
    push_presences(&social, at(50));
    let (first, when) = published(&social);
    push_presences(&social, at(5000));
    assert_eq!(published(&social), (first, when));

    // A real change inside the debounce waits for the next round.
    let moved = || vec![(online("one", &[]), shown(Status::Away, Place::Lobby))];
    push_presences(&social, moved());
    assert_eq!(published(&social).1, when);
    social.published.lock().shown.get_mut("one").unwrap().1 = when.checked_sub(DEBOUNCE).unwrap();
    push_presences(&social, moved());
    assert_eq!(published(&social).0.status, Shown::Away);
    social.published.lock().forget("one");
    assert!(social.published.lock().shown.is_empty());
}

#[test]
fn a_tables_conversation_holds_its_players_and_a_teams_only_its_own() {
    let mut game = table("g", LobbyState::Playing, &["a", "b", "c", "house"]);
    for (seat, team) in [(0, Some(1)), (1, Some(1)), (2, Some(2)), (3, Some(2))] {
        game.seats[seat].team = team;
    }
    game.seats[3].kind = SeatKind::Ai;
    let empty = table("empty", LobbyState::Waiting, &[]);
    let convs = conversations(&lobby_of(vec![game, empty]));
    let members: Vec<_> = convs
        .iter()
        .map(|(conv, (_, members))| {
            let members: Vec<&str> = members.iter().map(String::as_str).collect();
            (conv.as_str(), members)
        })
        .collect();
    assert_eq!(
        members,
        [
            ("tbl:g", vec!["a", "b", "c"]),
            ("team:g:1", vec!["a", "b"]),
            ("team:g:2", vec!["c"]),
        ]
    );
}

#[test]
fn table_mates_are_those_at_a_table_together_now() {
    use crate::social::members::{share_a_table, table_mates};
    let lobby = lobby_of(tables());
    assert!(share_a_table(&lobby, "one", "both"));
    assert!(share_a_table(&lobby, "both", "waiting"));
    assert!(!share_a_table(&lobby, "one", "waiting"));
    assert!(!share_a_table(&lobby, "one", "nobody"));
    assert!(
        !share_a_table(&lobby, "house", "house"),
        "the house's chair is nobody's"
    );
    let mates = table_mates(&lobby, "both");
    assert_eq!(
        mates.iter().map(String::as_str).collect::<Vec<_>>(),
        ["one", "waiting"]
    );
}
