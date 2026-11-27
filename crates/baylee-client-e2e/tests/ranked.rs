//! The signed-out ladder uses the public Central API. The loopback fixture
//! publishes synthetic standings only; its address lives for this test.
use baylee_client_e2e::prelude::*;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

struct PublicLadder {
    origin: String,
    stop: Arc<AtomicBool>,
    task: Option<std::thread::JoinHandle<()>>,
}

impl PublicLadder {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback fixture");
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let task = std::thread::spawn(move || {
            while let Ok((stream, _)) = listener.accept() {
                if stopping.load(Ordering::Acquire) {
                    break;
                }
                let _ = respond(stream);
            }
        });
        Self {
            origin: format!("http://{addr}"),
            stop,
            task: Some(task),
        }
    }
}
impl Drop for PublicLadder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.origin.trim_start_matches("http://"));
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

fn respond(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut body = Vec::new();
    {
        let mut reader = BufReader::new(&stream).take(65536);
        let mut len = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" {
                break;
            }
            if let Some((name, v)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                len = v.trim().parse::<usize>().unwrap_or_default().min(16_384);
            }
        }
        body.resize(len, 0);
        reader.read_exact(&mut body)?;
    }
    let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let current = json!({"id":vec![1u8;16],"name":"Solstice","starts":1_782_864_000,"ends":null});
    let past =
        json!({"id":vec![2u8;16],"name":"Equinox","starts":1_767_225_600,"ends":1_782_864_000});
    let reply = if request.pointer("/request/seasons").is_some() {
        json!({"seasons":{"items":[current,past],"next":null}})
    } else if request.pointer("/request/profile/target/handle") == Some(&json!("Mistral-01#0001")) {
        let player = standing(
            &json!({"format":"standard","track":"player","league":"champion","division":null}),
            1,
        );
        let deck = standing(
            &json!({"format":"commander","track":"deck","league":"gold","division":2}),
            1,
        );
        let commander = standing(
            &json!({"format":"commander","track":"player","league":"platinum","division":4}),
            1,
        );
        let award = json!({"kind":"peak","format":"standard","track":"player","season":current["id"],"rank":player["standing"]["rank"]});
        json!({"profile":{"season":current,"account":player["account"],"name":player["name"],
            "tag":player["tag"],"formats":[player["standing"],commander["standing"],deck["standing"]],"awards":[award]}})
    } else if let Some(query) = request.pointer("/request/board") {
        let first = query["after"].is_null();
        let limit = if query["format"] == "highlander" && query["league"] == "champion" {
            20
        } else {
            40
        };
        let range = if first {
            1u8..=limit.min(30)
        } else {
            31..=limit
        };
        // Whom the board lists (protocol 3): an absent `players` is everyone.
        let players = query["players"].as_str().unwrap_or("everyone");
        let entries: Vec<_> = range
            .map(|n| standing(query, n))
            .filter(|row| match players {
                "ai" => row.get("ai").is_some(),
                "people" => row.get("ai").is_none(),
                _ => true,
            })
            .collect();
        let next = (first && limit > 30).then(|| {
            let last = entries.last().expect("first page has 30 entries");
            json!({"elo":last["standing"]["elo"],"account":last["account"],"deck":last["standing"]["deck"]})
        });
        let season = if query["season"] == past["id"] {
            past
        } else {
            current
        };
        json!({"board":{"season":season,"entries":entries,"next":next}})
    } else {
        stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        )?;
        return Ok(());
    };
    let response = json!({"result":"ranked","response":reply});
    let bytes = serde_json::to_vec(&response)?;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)
}

fn standing(query: &Value, n: u8) -> Value {
    let placed = query["league"] != "unranked";
    let deck = (query["track"] == "deck").then(|| vec![n; 32]);
    let (elo, games, wins, losses, draws, career) = if query["league"] == "champion" {
        (
            3030 - i32::from(n),
            1340,
            901,
            426,
            13,
            json!({"games":3540,"wins":2394,"losses":1114,"draws":32}),
        )
    } else if query["league"] == "platinum" {
        (
            1852,
            500,
            312,
            182,
            6,
            json!({"games":950,"wins":602,"losses":339,"draws":9}),
        )
    } else if placed {
        (
            1775,
            310,
            188,
            118,
            4,
            json!({"games":650,"wins":388,"losses":255,"draws":7}),
        )
    } else {
        (
            1516 - i32::from(n),
            7,
            4,
            3,
            0,
            json!({"games":7,"wins":4,"losses":3,"draws":0}),
        )
    };
    let offset = match query["format"].as_str() {
        Some("commander") => 40,
        Some("highlander") => 80,
        _ => 0,
    };
    let names = if !placed {
        ["Brook", "Aster", "Hazel", "Finch", "Mica"]
    } else if offset == 0 {
        ["Linden", "Mistral", "Ember", "Willow", "Sable"]
    } else {
        ["Laurel", "Reed", "Iris", "Birch", "Rowan"]
    };
    // Two rows in seven are AI players, named with Central's reserved
    // prefix and labelled by their badge.
    let ai = match n % 7 {
        4 => Some(json!({"kind":"house","label":"Haus-KI · Expert"})),
        6 => Some(json!({"kind":"model","label":"Claude Haiku · Hoch"})),
        _ => None,
    };
    let name = if ai.is_some() {
        format!("KI-Spieler-{n:02}")
    } else {
        format!("{}-{n:02}", names[usize::from(n) % names.len()])
    };
    let place = (query["league"] == "champion").then_some(n + offset);
    let mut account = [n; 16];
    account[0] = offset;
    account[1] = u8::from(placed);
    let mut row = json!({
        "account":account,"name":name,"tag":u32::from(n),
        "standing":{
            "format":query["format"],"track":query["track"],"deck":deck,
            "elo":elo,"games":games,"wins":wins,"losses":losses,"draws":draws,
            "placement_games":if placed{10}else{7},"career":career,
            "rank":{"league":query["league"],"division":query["division"],"champion_place":place}
        }
    });
    if let Some(ai) = ai {
        row["ai"] = ai;
    }
    row
}

fn presented(screen: &Screen, text: &str) -> bool {
    screen.nodes().iter().enumerate().any(|(index, node)| {
        !node.hidden
            && node.text.as_deref() == Some(text)
            && screen.window().contains(&node.rect, 1.0)
            && screen.clipped_by(index).is_none()
    })
}

/// An isolated Metal window can hand the renderer a cleared drawable during
/// screenshot attachment. Accept a frame only after the UI has reached the
/// pixel buffer. This also fails if a regression permanently renders black.
fn capture(client: &mut Client, name: &str) -> Result<()> {
    for _ in 0..4 {
        let path = client.screenshot(name)?;
        let image = image::open(&path)?.into_rgb8();
        let mut dark = 0;
        let mut light = 0;
        for pixel in image.pixels().step_by(283) {
            dark += usize::from(pixel.0.iter().all(|&channel| channel < 48));
            light += usize::from(pixel.0.iter().any(|&channel| channel > 142));
        }
        // Play's retained tree may already exist during the rendered entrance
        // passage. The bright portal still lacks the dark Play header.
        let has_header = !name.starts_with("ranked-play-") || {
            let width = image.width() as usize;
            image
                .rows()
                .nth((image.height() / 60) as usize)
                .is_some_and(|row| {
                    row.skip(width / 3)
                        .take(width / 4)
                        .step_by(7)
                        .filter(|pixel| pixel.0.iter().all(|&channel| channel < 100))
                        .count()
                        >= width / 50
                })
        };
        if dark >= 32 && light >= 32 && has_header {
            return Ok(());
        }
        std::fs::remove_file(path)?;
    }
    anyhow::bail!("no rendered pixels in four captures of {name}")
}

/// Whom the ladder lists: the AI players alone, with their badges, and back;
/// the ladder's head (`ladder`, its Refresh) stays where it was.
fn ai_players_on_the_ladder(client: &mut Client, ladder: Rect) -> Result<()> {
    client.click(press("Ranked(Players(Ai))"))?;
    let ai = client.wait_until("only AI players on the ladder", |s| {
        s.shows("Haus-KI · Expert") && !s.shows("Mistral-01")
    })?;
    anyhow::ensure!(
        ai.locate(&press("Ranked(Refresh)"))? == ladder,
        "the ladder's head moved with its filter"
    );
    capture(client, "ranked-board-ai")?;
    client.click(press("Ranked(Players(Everyone))"))?;
    client.wait_until("everyone again", |s| s.shows("Mistral-01"))?;
    Ok(())
}

/// The first search asks whom it accepts, with nothing picked, and
/// searches only once a choice is confirmed; the fill-in countdown comes
/// into its own room, and one click to people only takes it away, with
/// Leave standing still throughout.
fn first_search_asks_counts_down_and_goes_to_people(
    client: &mut Client,
    ladder: Rect,
) -> Result<()> {
    client.click(press("Ranked(Join)"))?;
    client.wait_until("the sheet asking whom the search accepts", |s| {
        s.shows("Wer darf gegen dich spielen?")
            && s.shows("Der Anbieter eines Sprachmodells erhält die Züge dieser Partie.")
    })?;
    capture(client, "ranked-opponents-sheet")?;
    anyhow::ensure!(
        client
            .screen()?
            .control("Shared(ConfirmDestructive)")
            .is_none(),
        "Search answers nothing before a choice is picked"
    );
    client.click(press("Ranked(PickOpponents(Local))"))?;
    client.wait_until("Search answers once a choice is picked", |s| {
        s.control("Shared(ConfirmDestructive)").is_some()
    })?;
    client.click(press("Shared(ConfirmDestructive)"))?;
    let waiting = client.wait_until("the waiting queue beside the ladder", |s| {
        s.shows("Gegner für Highlander wird gesucht") && s.shows("Mistral-01")
    })?;
    anyhow::ensure!(
        waiting.locate(&press("Ranked(Refresh)"))? == ladder,
        "the ladder moved while the desk searched"
    );
    let leave = waiting.locate(&press("Ranked(Cancel)"))?;
    capture(client, "ranked-page-waiting")?;
    let counting = client.wait_until("the fill-in countdown", |s| {
        s.shows("KI-Gegner in") && s.shows("Lokale KIs")
    })?;
    anyhow::ensure!(
        counting.locate(&press("Ranked(Cancel)"))? == leave,
        "Leave moved when the countdown came"
    );
    capture(client, "ranked-page-countdown")?;
    client.click(press("Ranked(PeopleOnly)"))?;
    let people = client.wait_until("a search for people only", |s| {
        !s.shows("KI-Gegner in") && s.shows("Du wartest auf einen Menschen")
    })?;
    anyhow::ensure!(
        people.locate(&press("Ranked(Cancel)"))? == leave,
        "Leave moved when the countdown went"
    );
    Ok(())
}

fn narrow_queue_remains_playable(client: &mut Client) -> Result<()> {
    client.resize(Viewport::NARROW)?;
    let opened = client.wait_until("the desk stacked over the ladder", |s| {
        s.is_at(Viewport::NARROW) && s.control("Ranked(Join)").is_some()
    })?;
    capture(client, "ranked-page-narrow")?;
    // A narrow window scrolls the whole page; the desk stands first, so its
    // search is reached by scrolling the page as a player would, if at all.
    let (x, y) = opened.locate(&press("Ranked(Back)"))?.centre();
    for _ in 0..15 {
        if presented(&client.screen()?, "Gegner suchen") {
            break;
        }
        client.scroll(at(x, y + 120.0), 1.0)?;
    }
    client.wait_until("search reachable on the narrow page", |s| {
        presented(s, "Gegner suchen")
    })?;
    client.click(press("Ranked(Join)"))?;
    client.wait_until("compact queue confirmation", |s| {
        s.shows("Gegner für Highlander wird gesucht")
    })?;
    client.resize(Viewport::LAPTOP)?;
    client.click(press("Ranked(Cancel)"))?;
    client.wait_until("compact search cancelled", |s| s.shows("Sternenmeer"))?;
    Ok(())
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn ranked_public_board_filters_and_scrolls_without_an_account() {
    let fixture = PublicLadder::new();
    let launch = Launch::new()
        .viewport(Viewport::LAPTOP)
        .lang("en")
        .env("BAYLEE_CENTRAL_DEV_ENDPOINT", &fixture.origin);
    e2e::run_with(launch, |client| {
        client.click(press("Ranked(Open)"))?;
        let board = client.wait_until("the public Champion ladder", |s| s.shows("Mistral-01"))?;
        board.expect_not_shown("Sign in with Baylee Central")?;
        board.expect_not_shown("Find opponent")?;
        capture(client, "ranked-champion")?;
        client.click(text("Mistral-01#0001"))?;
        client.wait_until("the named player's public profile", |s| {
            s.shows("Player profile")
                && s.shows("Career · 3540 games")
                && s.shows("1775 Elo")
                && s.shows("Ranked honours")
        })?;
        capture(client, "ranked-profile")?;
        client.press(Key::named("Escape"))?;
        client.wait_until("return from the profile to its ladder", |s| {
            !s.shows("Player profile") && s.shows("Mistral-01")
        })?;
        client.scroll(text("Mistral-01#0001"), 12.0)?;
        client.wait_until("first player scrolls above the standings viewport", |s| {
            s.nodes().iter().enumerate().any(|(index, node)| {
                node.text.as_deref() == Some("Mistral-01#0001") && s.scrolled_away(index)
            })
        })?;
        client.click(press("Ranked(Page(true))"))?;
        client.wait_until("second page", |s| {
            s.shows("Page 2") && s.shows("Mistral-31")
        })?;
        client.click(press("Ranked(Page(false))"))?;
        client.wait_until("first page restored", |s| {
            s.shows("Page 1") && s.shows("Mistral-01")
        })?;
        client.click(press("Ranked(ToggleLeagues)"))?;
        client.click(press("Ranked(League(Unranked))"))?;
        client.wait_until("seven completed placements", |s| s.shows("7/10 placements"))?;
        capture(client, "ranked-placement")?;
        client.click(press("Ranked(Track(Deck))"))?;
        client.wait_until("deck-version identifier", |s| s.shows("01010101"))?;
        client.click(press("Ranked(Seasons)"))?;
        client.wait_until("previous season", |s| s.shows("Equinox"))?;
        client.click(label("Equinox · 2026-01-01"))?;
        client.wait_until("archive board", |s| {
            s.shows("Equinox") && s.shows("01010101")
        })?;
        let narrow = client.resize(Viewport::NARROW)?;
        narrow.expect_shows("Equinox")?;
        narrow.expect_shows("01010101")?;
        client.click(press("Ranked(Back)"))?;
        client.wait_until("returned to the original sign-in screen", |s| {
            s.control("Front(WithoutAccount)").is_some()
        })?;
        Ok(())
    });
}

#[test]
#[ignore = "opens a passive client window: cargo dev e2e"]
fn ranked_page_searches_and_cancels_beside_the_public_ladder() {
    let fixture = PublicLadder::new();
    let launch = Launch::new()
        .viewport(Viewport::LAPTOP)
        .lang("de")
        .env("BAYLEE_CENTRAL_DEV_ENDPOINT", &fixture.origin)
        .env("BAYLEE_E2E_RANKED_FIXTURE", "1");
    e2e::run_with(launch, |client| {
        let seeded = client
            .devctl()
            .post("shell", &json!({ "ranked_lobby": true }))?;
        anyhow::ensure!(seeded["ranked_lobby"] == true, "fixture scene was refused");
        let lobby = client.wait_until("the four game modes and the gateway's tables", |s| {
            [
                "Ranglistenspiel",
                "Tower-Run",
                "Tagesaufgabe",
                "Wochenaufgabe",
                "Abendrunde",
            ]
            .iter()
            .all(|name| s.shows(name))
                && s.at_rest()
                && !s.flag("/account/entrance_active")
                && !s.flag("/account/journey_active")
        })?;
        let opener = "Ranked(Open)";
        let card = lobby.locate(&press(opener))?;
        let tables = lobby.locate(&press("Play(CreateTable)"))?;
        anyhow::ensure!(lobby.window().contains(&card, 1.0) && card.right() < tables.x);
        anyhow::ensure!(
            lobby.control("Ranked(Join)").is_none(),
            "the queue stands on the Ranked page, not in the lobby"
        );
        capture(client, "ranked-play-lobby")?;
        client.click(press(opener))?;
        let page = client.wait_until("the desk beside the public ladder", |s| {
            s.control("Ranked(Format(Standard))").is_some()
                && s.shows("Morgenrot")
                && s.shows("Mistral-01")
        })?;
        let ladder = page.locate(&press("Ranked(Refresh)"))?;
        client.click(press("Ranked(ToggleDecks)"))?;
        client.click(text("Waldwache"))?;
        client.wait_until("chosen deck", |s| {
            s.shows("Waldwache") && !s.shows("Morgenrot")
        })?;
        client.click(press("Ranked(Format(Highlander))"))?;
        client.wait_until("only the Highlander deck is offered", |s| {
            s.shows("Sternenmeer")
        })?;
        capture(client, "ranked-page")?;
        ai_players_on_the_ladder(client, ladder)?;
        first_search_asks_counts_down_and_goes_to_people(client, ladder)?;
        client.click(press("Ranked(Cancel)"))?;
        client.wait_until("back in the same format after cancelling", |s| {
            s.shows("Sternenmeer") && s.control("Ranked(Join)").is_some()
        })?;
        narrow_queue_remains_playable(client)?;
        // The page's one format choice moves the ladder and the desk together.
        client.click(press("Ranked(Format(Commander))"))?;
        client.wait_until("Commander deck and ladder", |s| s.shows("Solstice"))?;
        client.click(press("Ranked(Join)"))?;
        client.wait_until("player division promotion", |s| {
            presented(s, "Commander · Spieler · Gold I")
        })?;
        client.wait_until("the result names its AI opponent with its badge", |s| {
            s.shows("Gegen") && s.shows("Haus-KI · Expert")
        })?;
        capture(client, "ranked-division-fixture")?;
        client.wait_until("deck league promotion", |s| {
            presented(s, "Commander · Deck · Platin IV")
        })?;
        capture(client, "ranked-league-fixture")?;
        client.click(press("Ranked(Back)"))?;
        client.wait_until("Play again, the tile saying the result", |s| {
            s.shows("Abendrunde") && s.control("Ranked(Open)").is_some()
        })?;
        Ok(())
    });
}
