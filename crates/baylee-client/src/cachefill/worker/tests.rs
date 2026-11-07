//! The fill against servers on loopback, on the real clock and the real
//! reader: what a fake clock cannot prove. Nothing here leaves the machine.

use super::*;
use crate::artreader::{ArtReader, allowed, fill_one, key};
use baylee_client_core::cachefill::{
    Answer, Budget, FREE_FLOOR, FillChoices, MAX_FAILURES, Pace, Reach, Status, Steering, Stop,
    Trouble,
};
use bevy::asset::io::{AssetReader, AssetReaderError, Reader};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

/// One pixel of PNG: what a reader asks of a picture is its header.
const ONE_PIXEL: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89\0\0\0\rIDATx\x9cc\0\x01\0\0\x05\0\x01\r\n-\xb4\0\0\0\0IEND\xaeB`\x82";

/// The picture a test server hands out, as a server's bytes: a real one, and
/// after its end the path that asked, so that no two are the same.
fn picture_of(path: &str) -> Vec<u8> {
    [ONE_PIXEL, path.as_bytes()].concat()
}

/// How a test server answers one path.
#[derive(Clone)]
struct Reply {
    /// The status line and any headers after it.
    head: String,
    /// How long it thinks first.
    delay: Duration,
    /// How long it pauses between two pieces of its body.
    trickle: Duration,
}

impl Reply {
    fn ok() -> Self {
        Self::of("200 OK")
    }

    fn of(head: &str) -> Self {
        Self {
            head: head.to_string(),
            delay: Duration::ZERO,
            trickle: Duration::ZERO,
        }
    }

    fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// One request a test server was sent.
#[derive(Clone, Debug)]
struct Seen {
    path: String,
    head: String,
    at: Instant,
}

/// A web server on loopback, one thread a connection, so requests overlap as
/// they would at a real one.
struct Server {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Server {
    fn start(answer: impl Fn(&str, usize) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let base = format!("http://{}/art", listener.local_addr().expect("bound"));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let answer = Arc::new(answer);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let (log, answer) = (log.clone(), answer.clone());
                thread::spawn(move || serve(stream, &log, answer.as_ref()));
            }
        });
        Self { base, seen }
    }

    fn requests(&self) -> usize {
        self.seen.lock().expect("the log").len()
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("the log").clone()
    }
}

fn serve(
    mut stream: TcpStream,
    log: &Mutex<Vec<Seen>>,
    answer: &(impl Fn(&str, usize) -> Reply + ?Sized),
) {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 2048];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&chunk[..n]),
        }
    }
    let head = String::from_utf8_lossy(&request).to_string();
    let path = head
        .split_ascii_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let count = {
        let mut log = log.lock().expect("the log");
        log.push(Seen {
            path: path.clone(),
            head: head.to_ascii_lowercase(),
            at: Instant::now(),
        });
        log.len()
    };
    let reply = answer(&path, count);
    thread::sleep(reply.delay);
    let body = picture_of(&path);
    let _ = write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reply.head,
        body.len()
    );
    if reply.trickle.is_zero() {
        let _ = stream.write_all(&body);
        return;
    }
    for piece in body.chunks(4) {
        if stream.write_all(piece).is_err() || stream.flush().is_err() {
            return;
        }
        thread::sleep(reply.trickle);
    }
}

/// A directory of this test's own, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-fill-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("made");
    dir
}

/// Three cards, one of them double-faced.
fn little_pool() -> Vec<Source<'static>> {
    let card = |name, scryfall_id, back| Source {
        name,
        scryfall_id,
        back,
        basic: false,
    };
    vec![
        card("Alpha", "aaaaaaaa-0000-4000-8000-000000000001", false),
        card("Beta", "bbbbbbbb-0000-4000-8000-000000000002", true),
        card("Gamma", "cccccccc-0000-4000-8000-000000000003", false),
    ]
}

/// The pictures [`little_pool`] plans: the printed back, then two rounds of
/// three cards and one back.
const LITTLE_PLAN: usize = 1 + 4 + 4;

/// A worker filling from `server` into `dir`.
fn worker(server: &Server, dir: &Path, pool: fn() -> Vec<Source<'static>>) -> Worker {
    Worker {
        courier: Arc::new(Courier::new(0.0)),
        cache: dir.to_path_buf(),
        base: server.base.as_str().into(),
        pool,
        room: |_| Some(50_000_000_000),
    }
}

/// What a draw reads through, on the fill's own cache and courier, with `base`
/// in force and `session` shown to it.
fn reader_at(worker: &Worker, base: &str, session: Option<&str>) -> ArtReader {
    let base: Arc<str> = base.into();
    let session: Option<Arc<str>> = session.map(Into::into);
    ArtReader {
        scheme: "http",
        cache: Some(worker.cache.clone()),
        base: Arc::new(move || base.clone()),
        session: Arc::new(move || session.clone()),
        courier: worker.courier.clone(),
    }
}

/// A draw signed in nowhere: the base is the one the fill uses.
fn reader_of(worker: &Worker) -> ArtReader {
    let base = worker.base.clone();
    reader_at(worker, &base, None)
}

/// `url` read the way the asset server reads it.
fn read(reader: &ArtReader, url: &str) -> Result<Vec<u8>, AssetReaderError> {
    let path = Path::new(url.strip_prefix("http://").expect("an http url"));
    bevy::tasks::block_on(async {
        let mut bytes = Vec::new();
        reader.read(path).await?.read_to_end(&mut bytes).await?;
        Ok(bytes)
    })
}

fn steering(pace: Pace, reach: Reach) -> Steering {
    Steering {
        choices: FillChoices {
            on: true,
            pace,
            reach,
            budget: Budget::Two,
        },
        playing: false,
        held: false,
    }
}

/// Workers at work until their guard is stopped.
struct Crew {
    worker: Worker,
    done: Arc<AtomicBool>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl Crew {
    fn start(worker: &Worker, count: usize) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let threads = (0..count)
            .map(|_| {
                let (worker, done) = (worker.clone(), done.clone());
                thread::spawn(move || {
                    while !done.load(Ordering::SeqCst) {
                        worker.step();
                    }
                })
            })
            .collect();
        Self {
            worker: worker.clone(),
            done,
            threads,
        }
    }

    fn stop(self) {
        self.done.store(true, Ordering::SeqCst);
        self.worker.courier.with(|_, _| ());
        for thread in self.threads {
            thread.join().expect("a worker");
        }
    }
}

/// `count` workers at work until `until` says stop.
fn run(worker: &Worker, count: usize, until: impl Fn(Status) -> bool) {
    let crew = Crew::start(worker, count);
    let deadline = Instant::now() + Duration::from_secs(60);
    while !until(status(worker)) {
        assert!(Instant::now() < deadline, "the fill never got there");
        thread::sleep(Duration::from_millis(5));
    }
    crew.stop();
}

fn status(worker: &Worker) -> Status {
    worker.courier.with(|fill, now| fill.status(now))
}

/// Starts a fill and skips the settling.
fn begin(worker: &Worker, pace: Pace, reach: Reach) {
    let steering = steering(pace, reach);
    worker.courier.with(|fill, now| fill.steer(steering, now));
    worker.courier.skip(Duration::from_secs(6));
}

/// A prefetched card is drawn with no request at all, through the reader a
/// draw goes through, off the file the fill wrote.
#[test]
fn a_prefetched_card_is_drawn_with_no_network_request() {
    let dir = scratch("drawn");
    let server = Server::start(|_, _| Reply::ok());
    let fill = worker(&server, &dir, little_pool);
    begin(&fill, Pace::Brisk, Reach::Everything);
    run(&fill, 3, |status| status == Status::Finished);

    let progress = fill.courier.with(Fill::progress);
    let asked = server.requests();
    assert_eq!(asked, LITTLE_PLAN, "one request a picture");
    assert_eq!(progress.pictures_done as usize, LITTLE_PLAN);
    let draw = reader_of(&fill);
    for picture in plan(&server.base, &little_pool(), Reach::Everything) {
        let path = picture.url.strip_prefix("http://").expect("http");
        let path = path.split_once('/').expect("a path").1;
        assert_eq!(
            read(&draw, &picture.url).expect("a picture"),
            picture_of(&format!("/{path}")),
            "the bytes the server sent, for {}",
            picture.url
        );
    }
    assert_eq!(server.requests(), asked, "a draw asked the network");
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// The owner's reason for the key: a picture filled from the CDN is drawn
/// after sign-in, under the mirror's address and with its session, with no
/// request to either. The fill never asked the mirror, and showed nobody a
/// session.
#[test]
fn a_picture_filled_from_the_cdn_is_drawn_after_sign_in_with_no_request() {
    let dir = scratch("sign-in");
    let cdn = Server::start(|_, _| Reply::ok());
    let mirror = Server::start(|_, _| Reply::ok());
    let fill = worker(&cdn, &dir, little_pool);
    begin(&fill, Pace::Brisk, Reach::Everything);
    run(&fill, 3, |status| status == Status::Finished);
    let filled = cdn.requests();
    assert_eq!(filled, LITTLE_PLAN);
    assert!(
        cdn.seen()
            .iter()
            .all(|seen| !seen.head.contains("authorization")),
        "the fill signs in to nothing"
    );

    let signed_in = reader_at(&fill, &mirror.base, Some("tok"));
    for picture in plan(&mirror.base, &little_pool(), Reach::Everything) {
        let bytes = read(&signed_in, &picture.url).expect("a picture");
        let path = picture.url.split_once("/art").expect("a path").1;
        assert_eq!(bytes, picture_of(&format!("/art{path}")), "{}", picture.url);
    }
    assert_eq!(mirror.requests(), 0, "the draw asked the mirror");
    assert_eq!(cdn.requests(), filled, "the draw asked the CDN");

    // And not the other way: what a mirror answered is for its own table. A
    // gateway may answer any picture for a printing, so signed out of it the
    // picture is asked for again, and the mirror's stays on the mirror's shelf.
    let url = format!("{}/small/front/d/d/dd.jpg", mirror.base);
    read(&signed_in, &url).expect("a picture");
    assert_eq!(mirror.requests(), 1);
    read(&signed_in, &url).expect("a picture");
    assert_eq!(mirror.requests(), 1, "at its own table it was on disk");
    let signed_out = reader_of(&fill);
    let url = format!("{}/small/front/d/d/dd.jpg", cdn.base);
    read(&signed_out, &url).expect("a picture");
    assert_eq!(cdn.requests(), filled + 1, "signed out, the CDN was asked");
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// Pictures from when every gateway shared one shelf say nothing of where they
/// came from, so they go, and what lies beside them stays.
#[test]
fn the_shelf_every_gateway_shared_is_cleared_and_nothing_else() {
    let dir = scratch("retire");
    let shelf = dir.join("scryfall");
    let kept_there = shelf.join("a.jpg");
    std::fs::create_dir_all(&shelf).expect("the shelf");
    std::fs::write(&kept_there, b"x").expect("written");
    let old = format!("{}.jpg", "a".repeat(64));
    std::fs::write(dir.join(&old), b"x").expect("written");
    std::fs::write(dir.join("notes.txt"), b"x").expect("written");
    retire_the_shared_shelf(&shelf);
    assert!(!dir.join(&old).exists(), "the old picture went");
    assert!(dir.join("notes.txt").exists(), "what is no picture stays");
    assert!(kept_there.exists(), "Scryfall's shelf is not touched");
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// In the app every picture of the fill's plan lies on Scryfall's two image
/// hosts, inside the allowlist, and none on a gateway or on the API.
#[test]
fn the_fill_goes_straight_to_scryfall() {
    let planned = plan(images::SCRYFALL_CDN, &pool(), Reach::Everything);
    for picture in &planned {
        assert!(
            picture.url.starts_with("https://cards.scryfall.io/")
                || picture.url.starts_with("https://backs.scryfall.io/"),
            "{}",
            picture.url
        );
        assert_eq!(allowed(&picture.url, images::SCRYFALL_CDN), Ok(()));
    }
    // Lobby or no lobby, session or none: `start` names the CDN and nothing
    // else, and a worker has no other base to go to.
    assert!(planned.len() > 1_000);
}

/// The bytes stay in the cache. A file is all that is left, its name is the
/// key a draw looks up, and no `.part` remains.
#[test]
fn the_fill_writes_the_key_a_draw_reads_and_nothing_else() {
    let dir = scratch("keys");
    let server = Server::start(|_, _| Reply::ok());
    let fill = worker(&server, &dir, little_pool);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    run(&fill, 2, |status| status == Status::Finished);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("a name")
        })
        .collect();
    names.sort();
    let mut wanted: Vec<String> = plan(&server.base, &little_pool(), Reach::Thumbnails)
        .iter()
        .map(|picture| key(&picture.url, &server.base))
        .collect();
    wanted.sort();
    assert_eq!(names, wanted);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A restart resumes from the disk. A fresh courier and worker with nothing
/// but the cache directory ask for what is missing and no more.
#[test]
fn a_restart_asks_only_for_what_is_missing() {
    let dir = scratch("resume");
    let server = Server::start(|_, _| Reply::ok());
    let first = worker(&server, &dir, little_pool);
    begin(&first, Pace::Brisk, Reach::Thumbnails);
    run(&first, 2, |status| status == Status::Finished);
    let asked = server.requests();
    assert_eq!(asked, 5);

    // An empty file is a picture the reader would not take.
    let thumbnails = plan(&server.base, &little_pool(), Reach::Thumbnails);
    std::fs::write(dir.join(key(&thumbnails[1].url, &server.base)), b"").expect("emptied");

    let second = worker(&server, &dir, little_pool);
    begin(&second, Pace::Brisk, Reach::Everything);
    run(&second, 2, |status| status == Status::Finished);
    let more: Vec<String> = server.seen()[asked..]
        .iter()
        .map(|seen| seen.path.clone())
        .collect();
    assert_eq!(more.len(), 1 + 4, "the emptied one and the large round");
    assert_eq!(
        more.iter().filter(|path| path.contains("/small/")).count(),
        1
    );
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A disk at its floor and a cache at its budget are found by the listing, and
/// not one picture is asked for.
#[test]
fn a_full_disk_or_a_full_cache_asks_for_nothing() {
    let server = Server::start(|_, _| Reply::ok());

    let dir = scratch("floor");
    let mut fill = worker(&server, &dir, little_pool);
    fill.room = |_| Some(FREE_FLOOR);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    run(&fill, 2, |status| matches!(status, Status::Stopped(_)));
    assert_eq!(status(&fill), Status::Stopped(Stop::DiskFull));
    std::fs::remove_dir_all(&dir).expect("cleaned up");

    // Sparse: it reads as two gigabytes and takes none.
    let dir = scratch("budget");
    std::fs::File::create(dir.join("large"))
        .expect("made")
        .set_len(Budget::Two.bytes())
        .expect("sized");
    let fill = worker(&server, &dir, little_pool);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    run(&fill, 2, |status| matches!(status, Status::Stopped(_)));
    assert_eq!(status(&fill), Status::Stopped(Stop::Budget));
    assert_eq!(server.requests(), 0);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A pool large enough to outlast a measurement.
fn long_pool() -> Vec<Source<'static>> {
    (0_u32..2_000)
        .map(|n| Source {
            name: Box::leak(format!("Card {n:05}").into_boxed_str()),
            scryfall_id: Box::leak(format!("{n:08x}-0000-4000-8000-000000000000").into_boxed_str()),
            back: false,
            basic: false,
        })
        .collect()
}

/// A game stops the requests, and they go on by themselves when it ends.
#[test]
fn a_game_stops_the_requests_and_they_go_on_by_themselves() {
    let dir = scratch("rest");
    let server = Server::start(|_, _| Reply::ok().after(Duration::from_millis(5)));
    let fill = worker(&server, &dir, long_pool);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    let crew = Crew::start(&fill, 3);
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.requests() < 3 {
        assert!(Instant::now() < deadline, "the fill never started");
        thread::sleep(Duration::from_millis(10));
    }
    let mut rest = steering(Pace::Brisk, Reach::Thumbnails);
    rest.playing = true;
    fill.courier.with(|fill, now| fill.steer(rest, now));
    // What was in flight lands; nothing starts after it.
    thread::sleep(Duration::from_millis(100));
    let at_rest = server.requests();
    thread::sleep(Duration::from_millis(700));
    assert_eq!(server.requests(), at_rest, "a request during a game");

    let go_on = steering(Pace::Brisk, Reach::Thumbnails);
    fill.courier.with(|fill, now| fill.steer(go_on, now));
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.requests() == at_rest {
        assert!(Instant::now() < deadline, "the fill never went on");
        thread::sleep(Duration::from_millis(10));
    }
    crew.stop();
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// The median of `spans`.
fn median(mut spans: Vec<Duration>) -> Duration {
    spans.sort();
    spans[spans.len() / 2]
}

/// How long `url(i)` takes through the reader, `count` times, `apart` apart.
fn timed(
    reader: &ArtReader,
    count: usize,
    apart: Duration,
    url: impl Fn(usize) -> String,
) -> Vec<Duration> {
    (0..count)
        .map(|i| {
            thread::sleep(apart);
            let started = Instant::now();
            read(reader, &url(i)).expect("a picture");
            started.elapsed()
        })
        .collect()
}

/// Measured: what a running batch adds to a wanted picture. The batch's
/// requests take 150 ms here and a wanted picture's 30, so a wanted picture
/// queued behind the batch would show at once. Run with `--nocapture` for the
/// numbers `docs/client.md` quotes.
#[test]
fn a_running_batch_adds_almost_nothing_to_a_wanted_picture() {
    let slow = Duration::from_millis(150);
    let quick = Duration::from_millis(30);
    let dir = scratch("latency");
    let server = Server::start(move |path, _| {
        Reply::ok().after(if path.contains("/wanted/") {
            quick
        } else {
            slow
        })
    });
    let fill = worker(&server, &dir, long_pool);
    let draw = reader_of(&fill);
    let wanted = |tag: &'static str| {
        let base = server.base.clone();
        move |i: usize| format!("{base}/wanted/{tag}/{i}.jpg")
    };

    let idle = median(timed(&draw, 7, Duration::from_millis(20), wanted("idle")));

    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    let crew = Crew::start(&fill, 3);
    thread::sleep(Duration::from_millis(1_200));
    // Far enough apart that the batch is back at work each time.
    let busy = timed(&draw, 7, Duration::from_millis(1_000), wanted("busy"));
    let filled = server
        .seen()
        .iter()
        .filter(|seen| !seen.path.contains("/wanted/"))
        .count();
    crew.stop();
    let during = median(busy.clone());
    let worst = busy.iter().max().copied().unwrap_or_default();
    println!(
        "wanted picture: idle {idle:?}, beside a running batch {during:?} (worst {worst:?}), \
         added {:?}, batch sent {filled} requests",
        during.saturating_sub(idle)
    );
    assert!(filled > 10, "the batch was running: {filled}");
    assert!(
        during < idle + Duration::from_millis(100),
        "a running batch cost a wanted picture {:?} (idle {idle:?})",
        during.saturating_sub(idle)
    );
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A wanted picture has nothing in front of it: thirty at once, beside a
/// running fill, all reach the server together, however slowly it answers.
#[test]
fn wanted_pictures_have_nothing_in_front_of_them() {
    let dir = scratch("no-gate");
    let server = Server::start(|path, _| {
        Reply::ok().after(Duration::from_millis(if path.contains("/wanted/") {
            300
        } else {
            5
        }))
    });
    let fill = worker(&server, &dir, long_pool);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    let crew = Crew::start(&fill, 3);
    thread::sleep(Duration::from_millis(300));
    let draw = reader_of(&fill);
    let started = Instant::now();
    let loads: Vec<_> = (0..30)
        .map(|i| {
            let (draw, url) = (draw.clone(), format!("{}/wanted/{i}.jpg", server.base));
            thread::spawn(move || read(&draw, &url).expect("a picture"))
        })
        .collect();
    for load in loads {
        load.join().expect("a load");
    }
    let took = started.elapsed();
    crew.stop();
    let times: Vec<Instant> = server
        .seen()
        .iter()
        .filter(|seen| seen.path.contains("/wanted/"))
        .map(|seen| seen.at)
        .collect();
    let first = times.iter().min().expect("requests");
    let last = times.iter().max().expect("requests");
    println!(
        "30 wanted pictures: {:?} between the first and the last, {took:?} in all",
        last.duration_since(*first)
    );
    assert_eq!(times.len(), 30);
    assert!(
        last.duration_since(*first) < Duration::from_secs(1),
        "something paced them: {:?}",
        last.duration_since(*first)
    );
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A wanted picture behaves as it did: a 429 is its own and holds nothing up.
#[test]
fn a_429_to_a_wanted_picture_holds_nothing_up() {
    let dir = scratch("wanted-429");
    let server = Server::start(|_, count| {
        if count == 1 {
            Reply::of("429 Too Many Requests\r\nRetry-After: 3600")
        } else {
            Reply::ok()
        }
    });
    let fill = worker(&server, &dir, little_pool);
    let draw = reader_of(&fill);
    let url = |name: &str| format!("{}/small/front/a/b/{name}.jpg", server.base);
    assert!(matches!(
        read(&draw, &url("one")),
        Err(AssetReaderError::HttpError(429))
    ));
    assert!(
        read(&draw, &url("two")).is_ok(),
        "the next one goes at once"
    );
    assert_eq!(server.requests(), 2);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A fill request stops between two reads of its body for a wanted picture,
/// and leaves nothing behind.
#[test]
fn a_running_request_gives_way_mid_body() {
    let dir = scratch("gives-way");
    let server = Server::start(|_, _| Reply {
        trickle: Duration::from_millis(20),
        ..Reply::ok()
    });
    let url = format!("{}/normal/front/a/b/x.jpg", server.base);
    let asked = AtomicUsize::new(0);
    let answer = fill_one(&url, &server.base, &dir, &|| {
        asked.fetch_add(1, Ordering::SeqCst) >= 2
    });
    assert_eq!(answer, Answer::GaveWay);
    assert_eq!(std::fs::read_dir(&dir).expect("listed").count(), 0);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// Each answer a server can give, as the fill counts it, and only a success
/// is ever a file.
#[test]
fn every_answer_is_counted_and_only_a_picture_is_stored() {
    let second = Duration::from_secs(1);
    for (head, answer) in [
        ("404 Not Found", Answer::Missing),
        ("429 Too Many Requests", Answer::Busy(None)),
        (
            "429 Too Many Requests\r\nRetry-After: 7",
            Answer::Busy(Some(7 * second)),
        ),
        (
            "503 Service Unavailable\r\nRetry-After: 2",
            Answer::Busy(Some(2 * second)),
        ),
        ("500 Oops", Answer::Server(500, None)),
        (
            "502 Bad Gateway\r\nRetry-After: 3",
            Answer::Server(502, Some(3 * second)),
        ),
        ("401 Unauthorized", Answer::Refused(401)),
        ("403 Forbidden", Answer::Refused(403)),
        (
            "302 Found\r\nLocation: http://elsewhere/",
            Answer::Refused(302),
        ),
    ] {
        let dir = scratch("answers");
        let reply = Reply::of(head);
        let server = Server::start(move |_, _| reply.clone());
        let url = format!("{}/small/front/a/b/x.jpg", server.base);
        assert_eq!(
            fill_one(&url, &server.base, &dir, &|| false),
            answer,
            "{head}"
        );
        assert_eq!(
            std::fs::read_dir(&dir).expect("listed").count(),
            0,
            "{head}"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }
}

/// No answer at all is a failure to back off from.
#[test]
fn a_closed_port_is_unreachable() {
    let dir = scratch("closed");
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let base = format!("http://{}/art", listener.local_addr().expect("bound"));
    drop(listener);
    let url = format!("{base}/small/front/a/b/x.jpg");
    assert_eq!(fill_one(&url, &base, &dir, &|| false), Answer::Unreachable);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// The fill goes through the reader's own doors: the allowlist before a
/// request, the client's User-Agent, no session, and a file already there is
/// not asked for.
#[test]
fn the_fill_keeps_the_readers_rules() {
    let dir = scratch("rules");
    let server = Server::start(|_, _| Reply::ok());
    let elsewhere = Server::start(|_, _| Reply::ok());

    let refused = fill_one(
        &format!("{}/small/front/a/b/x.jpg", elsewhere.base),
        &server.base,
        &dir,
        &|| false,
    );
    assert_eq!(refused, Answer::Refused(0), "off the allowlist");
    assert_eq!(elsewhere.requests(), 0, "no request left");

    let url = format!("{}/small/front/a/b/x.jpg", server.base);
    let answer = fill_one(&url, &server.base, &dir, &|| false);
    assert!(matches!(answer, Answer::Stored(_)), "{answer:?}");
    let head = server.seen()[0].head.clone();
    assert!(!head.contains("authorization"), "{head}");
    assert!(
        head.contains(&format!(
            "\r\nuser-agent: {}\r\n",
            crate::cardtext::scryfall::AGENT
        )),
        "{head}"
    );
    assert_eq!(
        fill_one(&url, &server.base, &dir, &|| false),
        Answer::Already
    );
    assert_eq!(server.requests(), 1);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A cache that cannot be written stops the fill, and a draw still gets its
/// picture: the existing "treat the cache as optional".
#[cfg(unix)]
#[test]
fn a_locked_cache_stops_the_fill_and_costs_a_draw_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("locked");
    let cache = dir.join("art");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("locked");
    assert!(
        std::fs::create_dir(&cache).is_err(),
        "{} was writable, so this proves nothing (running as root?)",
        dir.display()
    );
    let server = Server::start(|_, _| Reply::ok());
    let fill = worker(&server, &cache, little_pool);
    begin(&fill, Pace::Brisk, Reach::Thumbnails);
    run(&fill, 2, |status| matches!(status, Status::Stopped(_)));
    assert_eq!(status(&fill), Status::Stopped(Stop::CannotWrite));
    let draw = reader_of(&fill);
    let url = format!("{}/small/front/f/f/ff.jpg", server.base);
    assert!(read(&draw, &url).is_ok());
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("unlocked");
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// End to end: a 429's `Retry-After` keeps the fill away for as long as it
/// says, and it then goes on where it stopped.
#[test]
fn a_429_keeps_the_fill_away_for_as_long_as_it_says() {
    let dir = scratch("fill-429");
    let server = Server::start(|_, count| {
        if count == 1 {
            Reply::of("429 Too Many Requests\r\nRetry-After: 30")
        } else {
            Reply::ok()
        }
    });
    let fill = worker(&server, &dir, little_pool);
    begin(&fill, Pace::Gentle, Reach::Thumbnails);
    run(&fill, 1, |status| matches!(status, Status::Cooling(_)));
    let Status::Cooling(wait) = status(&fill) else {
        panic!("it should be cooling: {:?}", status(&fill));
    };
    assert!(wait > Duration::from_secs(25), "{wait:?}");
    let crew = Crew::start(&fill, 1);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(server.requests(), 1, "nothing during the wait");
    fill.courier.skip(Duration::from_secs(31));
    let deadline = Instant::now() + Duration::from_secs(10);
    while status(&fill) != Status::Finished {
        assert!(Instant::now() < deadline, "the fill never went on");
        thread::sleep(Duration::from_millis(5));
    }
    crew.stop();
    assert_eq!(
        server.requests(),
        1 + 5,
        "the refused one again, and the rest"
    );
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// End to end: failure after failure stops the fill, a server that keeps
/// failing is asked [`MAX_FAILURES`] times, and then not again.
#[test]
fn a_failing_server_is_left_alone() {
    let dir = scratch("failing");
    let server = Server::start(|_, _| Reply::of("500 Oops"));
    let fill = worker(&server, &dir, little_pool);
    begin(&fill, Pace::Gentle, Reach::Thumbnails);
    let courier = fill.courier.clone();
    let hurrying = Arc::new(AtomicBool::new(true));
    let hurry = {
        let hurrying = hurrying.clone();
        thread::spawn(move || {
            while hurrying.load(Ordering::SeqCst) {
                courier.skip(Duration::from_secs(10));
                thread::sleep(Duration::from_millis(10));
            }
        })
    };
    run(&fill, 1, |status| matches!(status, Status::Stopped(_)));
    hurrying.store(false, Ordering::SeqCst);
    hurry.join().expect("the clock");
    assert_eq!(
        status(&fill),
        Status::Stopped(Stop::Failing(Trouble::Server(500)))
    );
    let asked = server.requests();
    assert_eq!(asked, MAX_FAILURES as usize);
    fill.courier.skip(Duration::from_secs(3_600));
    fill.step();
    assert_eq!(server.requests(), asked, "and then nothing");
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// A listing counts what is there, takes an empty file for missing as the
/// reader does, clears away a stale `.part` and not a fresh one, and clears
/// what the old key filed, which nothing reads and which would eat the budget.
#[test]
fn a_listing_sees_what_a_draw_would_read() {
    let dir = scratch("listing");
    std::fs::write(dir.join("whole"), b"12345").expect("written");
    std::fs::write(dir.join("empty"), b"").expect("written");
    let stale = dir.join("x.1-0.part");
    let fresh = dir.join("x.1-1.part");
    let old_key = dir.join("a".repeat(64));
    std::fs::write(&stale, b"123").expect("written");
    std::fs::write(&fresh, b"12").expect("written");
    std::fs::write(&old_key, b"123456789").expect("written");
    let old = SystemTime::now() - Duration::from_secs(7_200);
    std::fs::File::options()
        .write(true)
        .open(&stale)
        .expect("opened")
        .set_modified(old)
        .expect("aged");
    let listing = list(&dir);
    assert_eq!(listing.bytes, 7, "the whole file and the fresh part");
    assert!(listing.names.contains("whole"));
    assert!(!listing.names.contains("empty"));
    assert!(!stale.exists());
    assert!(fresh.exists());
    assert!(!old_key.exists(), "a picture under the old key");
    assert_eq!(list(&dir.join("not-yet")).bytes, 0);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// Emptying takes the pictures and leaves anything else in the directory.
#[test]
fn emptying_takes_only_pictures() {
    let dir = scratch("wipe");
    let picture = dir.join(key("http://mirror/one.jpg", "http://mirror"));
    std::fs::write(&picture, b"1").expect("written");
    std::fs::write(dir.join("b".repeat(64)), b"1").expect("written");
    std::fs::write(dir.join("a.1-1.part"), b"1").expect("written");
    std::fs::write(dir.join("notes.txt"), b"1").expect("written");
    wipe(&dir);
    assert!(!picture.exists());
    assert!(dir.join("notes.txt").exists());
    assert_eq!(std::fs::read_dir(&dir).expect("listed").count(), 1);
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// The disk's free room is read on this machine, and a cache directory that
/// is not there yet reads the disk it will be on.
#[test]
fn the_room_on_the_disk_can_be_read() {
    let dir = scratch("room");
    assert!(room(&dir).is_some_and(|free| free > 0));
    assert!(
        room(&dir.join("art/deep")).is_some_and(|free| free > 0),
        "not the same number twice: anything else writing to the disk moves it"
    );
    std::fs::remove_dir_all(&dir).expect("cleaned up");
}

/// The pool is what the owner asked for: fully implemented, tested cards and
/// nothing else. The counts are what `docs/client.md` quotes.
#[test]
fn the_pool_is_the_implemented_cards() {
    let implemented = baylee_cards::all().filter(|c| c.is_implemented()).count();
    let sources = pool();
    assert_eq!(sources.len(), implemented);
    assert!(implemented > 1_000);
    let backs = sources.iter().filter(|s| s.back).count();
    let planned = plan(images::SCRYFALL_CDN, &sources, Reach::Everything);
    println!(
        "{implemented} cards, {backs} with a back, {} pictures",
        planned.len()
    );
    assert!(planned.len() >= implemented * 2, "{}", planned.len());
    assert!(sources.iter().filter(|s| s.basic).count() >= 5);
    assert_eq!(
        planned.iter().filter(|p| p.card).count(),
        implemented * 2,
        "every card has a printing, so every card has its two pictures"
    );
}
