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
        let award = json!({"kind":"peak","format":"standard","track":"player","season":current["id"],"rank":player["standing"]["rank"]});
        json!({"profile":{"season":current,"account":player["account"],"name":player["name"],
            "tag":player["tag"],"formats":[player["standing"],deck["standing"]],"awards":[award]}})
    } else if let Some(query) = request.pointer("/request/board") {
        let first = query["after"].is_null();
        let range = if first { 1u8..=30 } else { 31..=40 };
        let entries: Vec<_> = range.map(|n| standing(query, n)).collect();
        let next = first.then(|| {
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
    let names = ["Linden", "Mistral", "Ember", "Willow", "Sable"];
    let name = format!("{}-{n:02}", names[usize::from(n) % names.len()]);
    let place = (query["league"] == "champion").then_some(n);
    json!({
        "account":vec![n;16],"name":name,"tag":u32::from(n),
        "standing":{
            "format":query["format"],"track":query["track"],"deck":deck,
            "elo":elo,"games":games,"wins":wins,"losses":losses,"draws":draws,
            "placement_games":if placed{10}else{7},"career":career,
            "rank":{"league":query["league"],"division":query["division"],"champion_place":place}
        }
    })
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
        if dark >= 32 && light >= 32 {
            return Ok(());
        }
        std::fs::remove_file(path)?;
    }
    anyhow::bail!("no rendered pixels in four captures of {name}")
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
        board.expect_shows("Sign in with Baylee Central")?;
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
fn ranked_wait_and_both_promotions_render_in_the_private_screen_fixture() {
    let fixture = PublicLadder::new();
    let launch = Launch::new()
        .viewport(Viewport::LAPTOP)
        .lang("de")
        .env("BAYLEE_CENTRAL_DEV_ENDPOINT", &fixture.origin)
        .env("BAYLEE_E2E_RANKED_FIXTURE", "1");
    e2e::run_with(launch, |client| {
        client.click(press("Ranked(Open)"))?;
        client.wait_until("public ladder and synthetic waiting queue", |s| {
            s.shows("Mistral-01")
                && s.shows("Gegner für Standard wird gesucht")
                && s.shows("7/10 Platzierungsspiele")
        })?;
        capture(client, "ranked-queue-fixture")?;
        client.click(press("Ranked(Format(Commander))"))?;
        client.wait_until("player division promotion", |s| {
            presented(s, "Commander · Spieler · Gold I")
        })?;
        capture(client, "ranked-division-fixture")?;
        client.wait_until("deck league promotion", |s| {
            presented(s, "Commander · Deck · Platin IV")
        })?;
        capture(client, "ranked-league-fixture")?;
        Ok(())
    });
}
