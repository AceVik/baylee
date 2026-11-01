//! Offline contracts; no client, gateway, model, renderer or production service.
use super::{
    Args, check, common,
    devctl::{Control, Devctl, Http},
    keys, pixels, report, screens, settings,
    tree::Tree,
};
use anyhow::Result;
use clap::Parser;
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

fn tree(nodes: &Value) -> Tree {
    Tree::read(&json!({"shell_nodes":nodes}), None).unwrap()
}
fn n(depth: usize, kind: &str, x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({"d":depth,"k":kind,"x":x,"y":y,"w":w,"h":h})
}
fn text(depth: usize, words: &str, x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({"d":depth,"t":words,"x":x,"y":y,"w":w,"h":h})
}
#[test]
fn lane_login_refuses_unread_and_names_the_ways_in_that_are_left() {
    // An old invocation still parses, so it reaches the refusal instead of
    // a usage error, a password given in plain text among its arguments.
    for args in [
        &[
            "shell",
            "lane",
            "login",
            "lane",
            "--password-env",
            "LANE_SECRET",
        ][..],
        &["shell", "lane", "login", "lane", "plaintext-password"],
    ] {
        assert!(Args::try_parse_from(args).is_ok(), "{args:?}");
    }
    let refused = super::lane::login().unwrap_err().to_string();
    assert_eq!(refused, super::lane::REFUSED);
    for named in ["password", "Central", "guest", "issues/406"] {
        assert!(refused.contains(named), "{named}: {refused}");
    }
    assert!(!refused.contains("plaintext-password"));
}
#[test]
fn cli_covers_every_legacy_entry_point_without_reading_credentials_or_network() {
    let cases: &[&[&str]] = &[
        &["check", "gallery", "/tmp/out"],
        &["check", "centred"],
        &["devctl", "/state"],
        &["devctl", "/step", "--body-file", "-"],
        &["front", "/tmp/out", "Local"],
        &["header", "/tmp/out"],
        &["keys", "/tmp/out"],
        &["lane", "login", "lane", "--password-env", "LANE_SECRET"],
        &[
            "look",
            "/tmp/out",
            "play,decks",
            "960x700,844x390",
            "--touch",
            "--lang",
            "de",
        ],
        &["measure", "hover"],
        &["measure", "presses"],
        &["measure", "quit", "123", "--session-stdin"],
        &[
            "probe",
            "play",
            "844x390",
            "de",
            "touch",
            "xl",
            "/tmp/shot.png",
        ],
        &["report", "walk"],
        &["report", "table"],
        &["report", "dump", "/tmp/out"],
        &["screens", "/tmp/out", "room"],
        &["settings", "/tmp/out"],
        &["walks", "w2", "w6"],
    ];
    for args in cases {
        assert!(
            Args::try_parse_from(std::iter::once("shell").chain(args.iter().copied())).is_ok(),
            "{args:?}"
        );
    }
    // The session token is never positional, and no password is taken for
    // a measurement: the gateway signs nobody in with one.
    for args in [
        &["shell", "measure", "quit", "123", "plaintext-token"][..],
        &[
            "shell",
            "measure",
            "quit",
            "lane",
            "123",
            "--password-stdin",
        ],
        &[
            "shell",
            "measure",
            "quit",
            "123",
            "--session-env",
            "A",
            "--session-stdin",
        ],
    ] {
        assert!(Args::try_parse_from(args).is_err(), "{args:?}");
    }
    assert!(
        Args::try_parse_from([
            "shell", "probe", "play", "844x390", "de", "touch", "unknown"
        ])
        .is_err()
    );
}
#[test]
fn budgets_count_unicode_but_exclude_nested_count_badges_and_keycaps() {
    let label = text(2, &"ü".repeat(18), 0.0, 0.0, 20.0, 20.0);
    let mut t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 200.0, 100.0),
        n(1, "button", 0.0, 0.0, 100.0, 44.0),
        label,
        n(2, "keycap", 0.0, 0.0, 10.0, 10.0),
        text(3, "Ctrl+VeryLong", 0.0, 0.0, 10.0, 10.0),
        n(2, "count", 0.0, 0.0, 10.0, 10.0),
        text(3, "99999999", 0.0, 0.0, 10.0, 10.0)
    ]));
    assert!(check::budget(&t, false).is_empty());
    assert_eq!(t.label(1), "ü".repeat(18));
    t.0[2].t = Some("ü".repeat(19));
    assert_eq!(check::budget(&t, false).len(), 1);
    assert!(check::budget(&t, true).is_empty());
    t.0[1].k = Some("segment".into());
    t.0[2].t = Some("ü".repeat(15));
    assert_eq!(check::budget(&t, true).len(), 1);
}
#[test]
fn roots_ancestry_and_subtrees_reindex_without_crossing_roots() {
    let mut a = n(0, "panel", 0.0, 0.0, 50.0, 50.0);
    a["r"] = json!("a");
    let mut b = n(0, "header", 0.0, 0.0, 50.0, 50.0);
    b["r"] = json!("b");
    let raw = json!([
        a,
        n(1, "row", 0.0, 0.0, 10.0, 10.0),
        text(2, "first", 0.0, 0.0, 5.0, 5.0),
        b,
        n(1, "hit", 0.0, 0.0, 44.0, 44.0)
    ]);
    let t = tree(&raw);
    assert_eq!(t.0[3].parent, None);
    assert_eq!(t.0[4].parent, Some(3));
    let sub = t.subtree(3).unwrap();
    assert_eq!(sub.0.len(), 2);
    assert_eq!(sub.0[0].index, 0);
    assert_eq!(sub.0[1].parent, Some(0));
    let only = Tree::read(&json!({"shell_nodes":raw}), Some("a")).unwrap();
    assert_eq!(only.0.len(), 3);
    assert_eq!(only.label(0), "first");
    assert!(Tree::read(&json!({"shell_nodes":[]}), None).is_err());
}
#[test]
fn scrolling_excuses_only_its_direct_content_not_overflow_within_that_content() {
    let mut t = tree(&json!([
        n(0, "scroll", 0.0, 0.0, 100.0, 40.0),
        text(1, "scrolled", 0.0, 50.0, 100.0, 20.0),
        text(2, "escapes content", 0.0, 80.0, 100.0, 20.0)
    ]));
    assert_eq!(check::overflow(&t).len(), 1);
    t.0[2].hid = true;
    assert!(check::overflow(&t).is_empty());
}
#[test]
fn fit_slack_fields_and_control_line_contract_have_independent_failures() {
    let mut label = text(2, "words", 0.0, 0.0, 100.0, 20.0);
    label["tw"] = json!(104);
    label["th"] = json!(21.5);
    label["ln"] = json!(1);
    let mut t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 200.0, 100.0),
        n(1, "button", 0.0, 0.0, 100.0, 44.0),
        label
    ]));
    assert!(check::fit(&t).is_empty());
    t.0[2].tw = Some(104.01);
    assert_eq!(check::fit(&t).len(), 1);
    t.0[2].ln = Some(2);
    t.0[2].tw = Some(110.0);
    assert!(check::fit(&t).is_empty());
    assert_eq!(check::lines(&t).len(), 1);
    t.0[2].th = Some(21.51);
    assert_eq!(check::fit(&t).len(), 1);
    t.0[1].k = Some("field".into());
    assert!(check::fit(&t).is_empty());
    assert!(check::lines(&t).is_empty());
    t.0[1].hid = true;
    assert!(t.hidden(&t.0[2]));
}
#[test]
fn text_overlap_clips_scroll_content_and_separates_opaque_layers() {
    let mut t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 200.0, 100.0),
        n(1, "scroll", 0.0, 0.0, 100.0, 20.0),
        text(2, "offscreen", 0.0, 30.0, 100.0, 20.0),
        text(1, "page", 0.0, 30.0, 100.0, 20.0),
        n(1, "opaque", 0.0, 0.0, 200.0, 100.0),
        text(2, "sheet", 0.0, 30.0, 100.0, 20.0)
    ]));
    assert!(check::text_overlap(&t).is_empty());
    t.0[4].k = Some("panel".into());
    assert_eq!(check::text_overlap(&t).len(), 1);
    t.0[4].hid = true;
    assert!(check::text_overlap(&t).is_empty());
}
#[test]
fn sibling_window_touch_and_center_thresholds_are_not_vacuous() {
    let mut t = tree(&json!([
        n(0, "panel", 100.0, 100.0, 1400.0, 600.0),
        n(1, "row", 100.0, 100.0, 100.0, 44.0),
        n(1, "row", 199.0, 100.0, 100.0, 44.0),
        n(1, "hit", -1.0, 100.0, 43.5, 43.5)
    ]));
    assert_eq!(check::siblings(&t).len(), 1);
    assert!(check::window(&t, 1600.0).is_empty());
    assert!(check::hit(&t, true).is_empty());
    t.0[3].x = -1.01;
    t.0[3].h = 43.49;
    assert_eq!(check::window(&t, 1600.0).len(), 1);
    assert_eq!(check::hit(&t, true).len(), 1);
    assert!(check::hit(&t, false).is_empty());
    assert!(check::centred(&t, 1600.0).is_empty());
    assert!(!check::centred(&t, 2560.0).is_empty());
    t.0[0].x = 640.0;
    t.0[0].w = 1280.0;
    assert!(check::centred(&t, 2560.0).is_empty());
}
#[test]
fn contrast_excludes_glyphs_and_edges_but_detects_bright_ground_and_black_frames() {
    let t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 96.0, 96.0),
        text(1, "glyph", 30.0, 30.0, 30.0, 20.0)
    ]));
    let mut image = RgbImage::from_pixel(96, 96, Rgb([40, 40, 40]));
    for x in 30..60 {
        for y in 30..50 {
            image.put_pixel(x, y, Rgb([255, 255, 255]));
        }
    }
    let (faults, samples) = pixels::contrast(&t, &image, 96.0, 96.0).unwrap();
    assert!(faults.is_empty());
    assert_eq!(samples.len(), 1);
    assert!(samples[0].samples > 100);
    image.put_pixel(72, 72, Rgb([255, 255, 255]));
    assert_eq!(pixels::contrast(&t, &image, 96.0, 96.0).unwrap().0.len(), 2);
    let black = RgbImage::new(96, 96);
    assert!(!pixels::not_black(&black));
    assert_eq!(pixels::contrast(&t, &black, 96.0, 96.0).unwrap().0.len(), 1);
    let mut mist = t.clone();
    mist.0[0].k = Some("mist".into());
    let gray = RgbImage::from_pixel(96, 96, Rgb([64, 64, 64]));
    assert_eq!(pixels::contrast(&t, &gray, 96.0, 96.0).unwrap().0.len(), 1);
    assert!(
        pixels::contrast(&mist, &gray, 96.0, 96.0)
            .unwrap()
            .0
            .is_empty()
    );
}
#[test]
fn ring_pixels_are_measured_not_assumed_and_credit_ignores_the_glyph_box() {
    let t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 100.0, 100.0),
        n(1, "row", 10.0, 10.0, 80.0, 30.0),
        n(2, "credit", 30.0, 15.0, 40.0, 20.0)
    ]));
    let mut image = RgbImage::from_pixel(100, 100, Rgb([40, 40, 40]));
    assert_eq!(pixels::ring(&image, &t.0[1], 1.0), 0);
    for x in 10..90 {
        image.put_pixel(x, 7, Rgb([100, 200, 200]));
    }
    assert_eq!(pixels::ring(&image, &t.0[1], 1.0), 80);
    for x in 30..70 {
        for y in 15..35 {
            image.put_pixel(x, y, Rgb([255, 255, 255]));
        }
    }
    assert!(pixels::credit(&t, &image).unwrap() > 4.5);
    image.put_pixel(12, 15, Rgb([255, 255, 255]));
    assert!(pixels::credit(&t, &image).unwrap() < 4.5);
}
#[test]
fn phone_rows_tiles_and_report_controls_use_ancestor_geometry() {
    let t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 844.0, 390.0),
        n(1, "scroll", 0.0, 0.0, 400.0, 100.0),
        n(2, "row", 0.0, 90.0, 250.0, 20.0),
        text(3, "off", 0.0, 90.0, 20.0, 20.0),
        n(1, "tile", 0.0, 120.0, 200.0, 270.0),
        n(2, "hit", 0.0, 345.0, 44.0, 44.0),
        n(1, "tile", 220.0, 120.0, 200.0, 300.0),
        n(2, "hit", 220.0, 380.0, 44.0, 44.0)
    ]));
    assert_eq!(settings::rows_in_view(&t, 390.0), 0);
    assert_eq!(screens::tiles_per_row(&t), 2);
    assert_eq!(screens::phone_counts("decks", &t, 390.0)["tiles_whole"], 1);
    let mut sheet = tree(&json!([
        n(0, "root", 0.0, 0.0, 844.0, 390.0),
        n(1, "opaque", 20.0, 0.0, 800.0, 195.0),
        n(2, "field", 30.0, 90.0, 400.0, 40.0),
        n(2, "scroll", 30.0, 140.0, 400.0, 54.0),
        n(2, "hit", 780.0, 150.0, 40.0, 44.0)
    ]));
    assert!(report::sheet_faults(&sheet, true).unwrap().is_empty());
    sheet.0[3].h = 55.0;
    sheet.0[4].x = 782.0;
    assert_eq!(report::sheet_faults(&sheet, true).unwrap().len(), 2);
}
#[test]
fn report_candidates_never_collect_opponent_hand_or_face_down_names() {
    let view = json!({"hand":[{"name":"Own"}],"battlefield":[{"name":"Public","card":1},{"name":"Hidden","card":null}],"stack":[],"looking_at":[],"library_tops":[{"name":"Revealed top","card":2}],"graveyards":[[{"name":"Grave","card":3}]],"exile":[[]],"command":[[]],"seats":[{"hand":[{"name":"Secret"}]}],"library":[{"name":"Secret deck"}]});
    let (shown, count) = report::shown_cards(&view).unwrap();
    assert_eq!(count, 5);
    assert_eq!(
        shown.into_iter().collect::<Vec<_>>(),
        ["Grave", "Own", "Public", "Revealed top"]
    );
    assert!(report::shown_cards(&json!({})).is_err());
}
#[test]
fn section_title_must_be_outside_nav_and_gallery_order_comes_from_named_table() {
    let mut t = tree(&json!([
        n(0, "panel", 0.0, 0.0, 100.0, 100.0),
        text(1, "Audio", 0.0, 0.0, 50.0, 20.0),
        n(0, "panel", 100.0, 0.0, 100.0, 100.0),
        text(1, "Graphics", 100.0, 0.0, 50.0, 20.0)
    ]));
    assert!(!settings::shown(&t, "Audio"));
    t.0[3].t = Some("Audio".into());
    assert!(settings::shown(&t, "Audio"));
    let source = "stops: &[\"wrong\"], GALLERY_ORDER: TabOrder = TabOrder { stops: &[\"nav\",\"create\",\"last\"], };";
    assert_eq!(keys::order(source).unwrap(), ["nav", "create", "last"]);
    assert!(keys::order("stops: &[\"create\"],").is_err());
    assert!(check::colophon("Fan-Content Scryfall AGPL-3.0").is_empty());
    assert_eq!(check::colophon("Scryfall").len(), 2);
}
#[test]
fn url_secrets_size_parsing_and_geometry_have_strict_boundaries() {
    for valid in [
        "http://127.0.0.1:28770",
        "http://[::1]:1/",
        "http://localhost:123",
    ] {
        assert!(super::devctl::loopback_url(valid).is_ok());
    }
    for invalid in [
        "https://127.0.0.1:80",
        "http://example.com",
        "http://127.0.0.1.evil:80",
        "http://user:secret@127.0.0.1:80",
        "http://127.0.0.1/a",
        "http://127.0.0.1/?secret=1",
        "http://192.168.0.1",
    ] {
        assert!(super::devctl::loopback_url(invalid).is_err(), "{invalid}");
    }
    let a = Control {
        press: "button".into(),
        at_x: 10.0,
        at_y: 20.0,
    };
    let mut b = a.clone();
    b.at_x += 0.5;
    assert!(super::devctl::stable(&a, &b));
    b.at_y += 0.51;
    assert!(!super::devctl::stable(&a, &b));
    assert_eq!(
        common::sizes(Some("960x700,844x390")).unwrap(),
        [(960, 700), (844, 390)]
    );
    assert!(common::sizes(Some("0x2")).is_err());
    let mut secret =
        json!({"password":"one","nested":[{"seat_token":"two","api_key":"three","ok":1}]});
    super::redact(&mut secret);
    assert!(!secret.to_string().contains("one"));
    assert!(!secret.to_string().contains("two"));
    assert!(!secret.to_string().contains("three"));
    assert_eq!(secret["nested"][0]["ok"], 1);
}

struct Reply {
    status: u16,
    body: Value,
}
struct Mock {
    http: Http,
    port: u16,
    requests: Arc<Mutex<Vec<String>>>,
    worker: thread::JoinHandle<()>,
}
fn request(stream: &mut TcpStream) -> String {
    // macOS can inherit O_NONBLOCK from the listening socket. A read timeout
    // alone does not clear it: accept may win the race against the first byte.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buf = [0; 4096];
    loop {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buf[..n]);
        assert!(bytes.len() < 1024 * 1024);
        if let Some(pos) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..pos]);
            let length = headers
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    (k.eq_ignore_ascii_case("content-length"))
                        .then(|| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if bytes.len() >= pos + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(bytes).unwrap()
}
fn mock(replies: Vec<Reply>) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let capture = Arc::clone(&requests);
    let worker = thread::spawn(move || {
        for reply in replies {
            let end = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < end, "missing expected request");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("accept: {e}"),
                }
            };
            capture.lock().unwrap().push(request(&mut stream));
            let body = reply.body.to_string();
            write!(stream,"HTTP/1.1 {} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nLocation: http://example.invalid/secret\r\nConnection: close\r\n\r\n{body}",reply.status,body.len()).unwrap();
        }
    });
    Mock {
        http: Http::new(&format!("http://127.0.0.1:{port}")).unwrap(),
        port,
        requests,
        worker,
    }
}
#[test]
fn transport_rejects_error_bodies_http_errors_and_redirects_without_leaking_secrets() {
    for status in [200, 403, 302] {
        let m = mock(vec![Reply {
            status,
            body: json!({"error":"secret-password token=secret-token"}),
        }]);
        let err = m
            .http
            .call(
                "/auth/login",
                Some(&json!({"password":"secret-password"})),
                Some("secret-token"),
                Duration::from_secs(2),
            )
            .unwrap_err()
            .to_string();
        m.worker.join().unwrap();
        assert!(!err.contains("secret-password"));
        assert!(!err.contains("secret-token"));
        assert_eq!(m.requests.lock().unwrap().len(), 1);
    }
}
#[test]
fn chord_releases_modifier_even_when_the_main_key_is_refused() {
    let m = mock(vec![
        Reply {
            status: 200,
            body: json!({"ok":true}),
        },
        Reply {
            status: 200,
            body: json!({"error":"refused"}),
        },
        Reply {
            status: 200,
            body: json!({"ok":true}),
        },
    ]);
    let d = Devctl::new(m.port).unwrap();
    assert!(d.chord("SuperLeft", "KeyZ", None).is_err());
    m.worker.join().unwrap();
    let requests = m.requests.lock().unwrap();
    let bodies: Vec<Value> = requests
        .iter()
        .map(|r| serde_json::from_str(r.split_once("\r\n\r\n").unwrap().1).unwrap())
        .collect();
    assert_eq!(bodies[0], json!({"name":"SuperLeft","hold":true}));
    assert_eq!(bodies[1], json!({"name":"KeyZ"}));
    assert_eq!(bodies[2], json!({"name":"SuperLeft","release":true}));
}
#[test]
fn assertion_failures_are_persisted_and_return_failure() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("report.log");
    let mut r = common::Report::default();
    r.expect(false, "deliberate bad fixture");
    assert!(r.finish(Some(&path), false).is_err());
    assert!(std::fs::read_to_string(path)?.contains("failures: 1"));
    Ok(())
}

#[test]
fn default_front_settings_and_header_screenshot_budgets_stay_bounded() {
    let mut medium = 0;
    let mut header = 0;
    for &size in check::SIZES {
        for lang in ["en", "de"] {
            header += usize::from(common::language_shot(lang, size));
            for step in ["xs", "m", "xl"] {
                medium += usize::from(common::medium_shot(step, lang, size));
            }
        }
    }
    assert_eq!(medium * 5, 45, "five front-door faces");
    assert_eq!(
        medium * 10 + 7 * 2,
        104,
        "ten settings sections plus profile sheets"
    );
    assert_eq!(header * 3, 27, "three signed-in headers");
    assert!(!common::medium_shot("xl", "en", (1920, 1080)));
}

#[test]
fn semantic_click_waits_for_geometry_then_prehover_then_fresh_geometry_before_press() {
    let health = json!({"width":800,"height":600});
    let state = |x| json!({"lobby_controls":[{"press":"Button","at_x":x,"at_y":200}]});
    // First geometry moves, then holds. Hover changes it again: no press may
    // use that stale point. Only the final, reread position may be clicked.
    let replies = [
        health.clone(),
        state(100),
        health.clone(),
        state(120),
        health.clone(),
        state(120),
        json!({"ok":true}),
        health.clone(),
        state(140),
        health.clone(),
        state(140),
        json!({"ok":true}),
        health,
        state(140),
        json!({"ok":true}),
    ]
    .into_iter()
    .map(|body| Reply { status: 200, body })
    .collect();
    let m = mock(replies);
    let d = Devctl::new(m.port).unwrap();
    d.press("Button").unwrap();
    m.worker.join().unwrap();
    let requests = m.requests.lock().unwrap();
    let lines: Vec<_> = requests.iter().map(|r| r.lines().next().unwrap()).collect();
    assert_eq!(
        lines,
        [
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "POST /pointer HTTP/1.1",
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "POST /pointer HTTP/1.1",
            "GET /health HTTP/1.1",
            "GET /state HTTP/1.1",
            "POST /pointer HTTP/1.1",
        ]
    );
    let pointers: Vec<Value> = requests
        .iter()
        .filter(|r| r.starts_with("POST /pointer "))
        .map(|r| serde_json::from_str(r.split_once("\r\n\r\n").unwrap().1).unwrap())
        .collect();
    assert_eq!(
        pointers,
        [
            json!({"x":120.0,"y":200.0}),
            json!({"x":140.0,"y":200.0}),
            json!({"x":140.0,"y":200.0,"button":"left","press":true})
        ]
    );
}

#[test]
fn waits_are_bounded_and_missing_conditions_do_not_succeed() {
    let mut calls = 0;
    assert!(
        Devctl::until(0.0, || {
            calls += 1;
            Ok(false)
        })
        .is_err()
    );
    assert_eq!(calls, 1);
    let error = Devctl::until(1.0, || anyhow::bail!("fixture failure")).unwrap_err();
    assert_eq!(error.to_string(), "fixture failure");
}

#[test]
fn obsolete_secret_argument_is_rejected_without_echoing_it() {
    let args = ["measure", "quit", "123", "do-not-echo-this-token"].map(str::to_owned);
    let err = super::run(std::path::Path::new("."), &args)
        .unwrap_err()
        .to_string();
    assert!(err.contains("invalid shell arguments"));
    assert!(!err.contains("do-not-echo-this-token"));
    // `lane login` takes the old arguments and refuses them unread (#406).
    let args = ["lane", "login", "lane", "do-not-echo-this-password"].map(str::to_owned);
    let err = super::run(std::path::Path::new("."), &args)
        .unwrap_err()
        .to_string();
    assert_eq!(err, super::lane::REFUSED);
    assert!(!err.contains("do-not-echo-this-password"));
}

#[test]
fn native_layout_matches_frozen_python_results_for_every_check_and_label() {
    // Generated read-only from the original check.py; the source digest and
    // concrete inputs/results live in the fixture, not a second implementation.
    let fixtures: Value = serde_json::from_str(include_str!("parity-fixtures.json")).unwrap();
    for case in fixtures["fixtures"].as_array().unwrap() {
        let t = tree(&case["nodes"]);
        let width = case["width"].as_f64().unwrap();
        let checks = [
            ("overflow", check::overflow(&t)),
            ("window", check::window(&t, width)),
            ("siblings", check::siblings(&t)),
            ("budget_en", check::budget(&t, false)),
            ("budget_de", check::budget(&t, true)),
            ("hit", check::hit(&t, true)),
            ("fit", check::fit(&t)),
            ("lines", check::lines(&t)),
            ("overlap", check::text_overlap(&t)),
            ("centred", check::centred(&t, width)),
        ];
        for (name, faults) in checks {
            assert_eq!(
                json!(faults.len()),
                case["counts"][name],
                "{}: {name}: {faults:?}",
                case["name"]
            );
        }
        let labels: Vec<_> = (0..t.0.len()).map(|i| t.label(i)).collect();
        assert_eq!(json!(labels), case["labels"], "{} labels", case["name"]);
    }
}

#[test]
fn fixture_reads_delayed_fragmented_requests_from_an_initially_nonblocking_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    // Exercise the macOS accept behaviour on every platform, not just on Macs.
    server.set_nonblocking(true).unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let (ready, started) = std::sync::mpsc::sync_channel(0);
    let worker = thread::spawn(move || {
        ready.send(()).unwrap();
        request(&mut server)
    });
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    thread::sleep(Duration::from_millis(20));
    client
        .write_all(b"POST /key HTTP/1.1\r\nContent-Length: 2\r\n")
        .unwrap();
    thread::sleep(Duration::from_millis(20));
    client.write_all(b"\r\n{").unwrap();
    thread::sleep(Duration::from_millis(20));
    client.write_all(b"}").unwrap();
    assert_eq!(
        worker.join().unwrap(),
        "POST /key HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}"
    );
}

#[test]
fn social_occlusion_is_not_panel_ground_but_exposed_bright_pixels_still_fail() {
    let t = tree(&json!([n(0, "panel", 0.0, 0.0, 96.0, 96.0)]));
    let mut image = RgbImage::from_pixel(96, 96, Rgb([40, 40, 40]));
    image.put_pixel(72, 72, Rgb([255, 255, 255]));
    let controls = pixels::social_controls(&json!({"chat":{"controls":[
        {"x":72.0,"y":72.0,"w":12.0,"h":12.0},
        {"x":0.0,"y":0.0,"w":-1.0,"h":1.0}, {}
    ]}}));
    assert_eq!(controls, vec![[66.0, 66.0, 78.0, 78.0]]);
    assert_eq!(pixels::contrast(&t, &image, 96.0, 96.0).unwrap().0.len(), 2);
    let (faults, report) = pixels::contrast_behind(&t, &image, 96.0, 96.0, &controls).unwrap();
    assert!(faults.is_empty());
    assert!(report[0].samples > 100);
    image.put_pixel(48, 48, Rgb([255, 255, 255]));
    assert_eq!(
        pixels::contrast_behind(&t, &image, 96.0, 96.0, &controls)
            .unwrap()
            .0
            .len(),
        2
    );
}

#[test]
fn a_scaled_focus_ring_is_sampled_at_its_scaled_offset() {
    let t = tree(&json!([n(0, "hit", 10.0, 10.0, 80.0, 30.0)]));
    let mut image = RgbImage::from_pixel(100, 100, Rgb([40, 40, 40]));
    assert_eq!(pixels::ring(&image, &t.0[0], 0.5), 0);
    for x in 10..90 {
        image.put_pixel(x, 8, Rgb([100, 200, 200]));
    }
    assert_eq!(pixels::ring(&image, &t.0[0], 0.5), 80);
    assert_eq!(
        pixels::ring(&image, &t.0[0], 1.0),
        0,
        "the old unscaled sampling missed the visible ring"
    );
}
