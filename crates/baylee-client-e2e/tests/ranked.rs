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
    let Some(query) = request.pointer("/request/board") else {
        stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        )?;
        return Ok(());
    };
    let entries:Vec<_>=(1u8..=30).map(|n|{
        let placed=query["league"]!="unranked";
        let deck=(query["track"]=="deck").then(||vec![n;32]);
        let record=json!({"games":if placed{2100}else{7},"wins":if placed{1490}else{4},"losses":if placed{590}else{3},"draws":if placed{20}else{0}});
        json!({
            "account":vec![n;16],"name":format!("{} {n:02}",["Linden","Mistral","Ember","Willow","Sable"][usize::from(n)%5]),"tag":u32::from(n),
            "standing":{"format":query["format"],"track":query["track"],"deck":deck,
            "elo":3030-i32::from(n),"games":record["games"],"wins":record["wins"],"losses":record["losses"],"draws":record["draws"],
            "placement_games":if placed{10}else{7},"career":record,
            "rank":{"league":query["league"],"division":query["division"],"champion_place":(query["league"]=="champion").then_some(n)}}})
    }).collect();
    let response = json!({"result":"ranked","response":{"board":{"season":{"id":vec![1u8;16],"name":"Solstice","starts":1,"ends":null},"entries":entries,"next":null}}});
    let bytes = serde_json::to_vec(&response)?;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)
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
        let board = client.wait_until("the public Champion ladder", |s| s.shows("Mistral 01"))?;
        board.expect_shows("Sign in with Baylee Central")?;
        board.expect_not_shown("Find opponent")?;
        client.screenshot("ranked-champion")?;
        client.scroll(text("Mistral 01#0001"), 12.0)?;
        client.click(press("Ranked(ToggleLeagues)"))?;
        client.click(press("Ranked(League(Unranked))"))?;
        client.wait_until("seven completed placements", |s| s.shows("7/10 placements"))?;
        client.screenshot("ranked-placement")?;
        client.click(press("Ranked(Track(Deck))"))?;
        client.wait_until("deck-version identifier", |s| s.shows("01010101"))?;
        client.click(press("Ranked(Back)"))?;
        client.wait_until("returned to the original sign-in screen", |s| {
            s.control("Front(WithoutAccount)").is_some()
        })?;
        Ok(())
    });
}
