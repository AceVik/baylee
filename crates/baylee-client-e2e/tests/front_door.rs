//! The front door: what a player sees before signing in or playing, at
//! every size and in both languages.
use baylee_client_e2e::prelude::*;

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_front_door_fits_every_viewport() {
    e2e::run(|client| {
        client.across(Viewport::ALL, |client, _| {
            let screen = client.settled()?;
            screen.expect_shows("Play offline")?;
            screen.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_largest_text_step_still_fits_the_small_windows() {
    e2e::run_with(Launch::new().text_step(TextStep::Xl), |client| {
        client.across(Viewport::SMALL, |client, _| {
            client.settled()?.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn a_german_client_opens_in_german() {
    e2e::run_with(Launch::new().lang("de"), |client| {
        let screen = client.settled()?;
        screen.expect_shows("Offline spielen")?;
        screen.expect_shows("Ohne Konto weiter")?;
        screen.expect_not_shown("Play offline")
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn picking_a_language_rewrites_the_interface_in_place() {
    e2e::run(|client| {
        client.click(press("Front(FrontMenu)"))?;
        client.click(press("Shared(PickLang(De))"))?;
        client.wait_until("German", |s| s.shows("Offline spielen"))?;
        client.click(press("Front(FrontMenu)"))?;
        client.click(press("Shared(PickLang(En))"))?;
        client
            .wait_until("English again", |s| s.shows("Play offline"))?
            .expect_not_shown("Offline spielen")
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn going_on_without_an_account_reaches_the_gateways_and_comes_back() {
    e2e::run(|client| {
        client.across(Viewport::ALL, |client, _| {
            client
                .settled()?
                .expect_shows("Continue without an account")?;
            client.click(press("Front(WithoutAccount)"))?;
            client.wait_until("the device's gateways", |s| {
                s.control("Front(BackToCentral)").is_some()
            })?;
            let list = client.settled()?;
            list.expect_shows("Choose your gateway")?;
            list.expect_shows("Sign in with an account")?;
            list.expect_layout_ok()?;
            client.click(press("Front(BackToCentral)"))?;
            client.wait_until("the sign-in", |s| {
                s.control("Front(WithoutAccount)").is_some()
            })?;
            client.settled()?.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn playing_offline_offers_the_house_and_a_deck() {
    e2e::run(|client| {
        let lobby = client.play_offline()?;
        lobby.locate(&label("Play the house"))?;
        lobby.expect_shows("Offline · local decks")?;
        lobby.expect_layout_ok()
    });
}

/// A stand-in for Baylee Central on a loopback port: it takes a pairing's
/// claim and then says "not approved yet" for as long as it is asked, so
/// the pairing's panel stands with its code. Anything else is a `503`.
/// Nothing reaches the real Central.
fn a_central_that_never_approves() -> Result<String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let origin = format!("http://{}", listener.local_addr()?);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = answer_a_pairing(stream);
        }
    });
    Ok(origin)
}

fn answer_a_pairing(mut stream: std::net::TcpStream) -> std::io::Result<()> {
    use std::io::{Read as _, Write as _};
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut head = Vec::new();
    let mut byte = [0];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16_384 {
        stream.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(65_536);
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    let body = String::from_utf8_lossy(&body);
    let (status, reply) = if body.contains(r#""operation":"pair_claim""#) {
        (
            "200 OK",
            r#"{"result":"pair_accepted","expires_at":4102444800}"#,
        )
    } else if body.contains(r#""operation":"pair_poll""#) {
        ("200 OK", r#"{"result":"pending"}"#)
    } else {
        ("503 Service Unavailable", "")
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    )
}

/// Whether `words` is a comparison code: `XXX-XXX` in Crockford's base32.
fn is_a_code(words: &str) -> bool {
    let bytes = words.as_bytes();
    bytes.len() == 7
        && bytes[3] == b'-'
        && bytes.iter().enumerate().all(|(i, b)| {
            i == 3 || ((b.is_ascii_digit() || b.is_ascii_uppercase()) && !b"ILOU".contains(b))
        })
}

/// A `baylee://pair` link at a cold start opens Central's sign-in and the
/// sign-in from baylee.app over it: the panel shows the code the page shows
/// beside its Approve, fits every window, and `Esc` cancels it and says so.
#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn a_pairing_link_shows_its_code_and_esc_cancels_it() {
    let central = a_central_that_never_approves().expect("a loopback port");
    // 32 zero bytes, as 43 characters of URL-safe base64.
    let zeros = "A".repeat(43);
    let link = format!("baylee://pair?h={zeros}&s={zeros}");
    let launch = Launch::new()
        .arg(&link)
        .env("BAYLEE_CENTRAL_DEV_ENDPOINT", &central);
    e2e::run_with(launch, |client| {
        let waiting =
            client.wait_until("the pairing's code", |s| s.shows("Continue on baylee.app"))?;
        ensure!(
            waiting.texts().filter(|words| is_a_code(words)).count() == 1,
            "the panel shows one code"
        );
        ensure!(
            waiting.control("Shared(PairSignIn)").is_none(),
            "nothing to say yes to before the page approved"
        );
        client.across(Viewport::ALL, |client, _| {
            client.settled()?.expect_layout_ok()
        })?;
        client.press(Key::named("Escape"))?;
        client
            .wait_until("the pairing cancelled", |s| {
                s.shows("The sign-in from baylee.app was cancelled.")
            })?
            .expect_not_shown("Continue on baylee.app")
    });
}
