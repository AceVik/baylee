use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

use baylee_client_core::portrait::{File, Worn};
use bevy::time::TimeUpdateStrategy;

use super::helper::{FRAME_BYTES, Frames, Refused};
use super::*;
use crate::shellkit::avatar::{AvatarFace, wear_portraits};

/// A moving picture as Central stores one: an AVIF image sequence, 256 px
/// square, six frames shown 100, 200, 300, 400, 500 and 100 ms (made with
/// ffmpeg's `libsvtav1` from its `testsrc2` pattern; nobody's picture).
const MOVING: &[u8] = include_bytes!("../../tests/data/picture-moving.avif");

/// The fixture's frames' durations by the parent's own reading.
const MOVING_MS: [u32; 6] = [100, 200, 300, 400, 500, 100];

/// A 256 px PNG of one colour, as Central's master is made.
fn png(side: u32, rgba: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(side, side, image::Rgba(rgba));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("a PNG");
    bytes.into_inner()
}

// --- The child's decode -------------------------------------------------

/// The decoder's own work, in memory: exactly the frames the parent's
/// reading counted, each 256 × 256 RGBA, and then nothing.
#[test]
fn the_decoder_writes_exactly_the_frames_and_nothing_else() {
    assert_eq!(helper::durations(MOVING), Some(MOVING_MS.to_vec()));
    let mut input = Vec::new();
    baylee_worker::frame::write(&mut input, MOVING, baylee_avif::Bounds::PICTURE.max_bytes)
        .unwrap();
    let mut output = Vec::new();
    helper::serve(&mut input.as_slice(), &mut output).expect("decoded");
    assert_eq!(output.len(), MOVING_MS.len() * FRAME_BYTES);
    let frames = helper::read_frames(&mut output.as_slice(), MOVING_MS.len()).unwrap();
    assert_eq!(frames.len(), 6);
    assert!(
        frames
            .iter()
            .all(|frame| frame.len() == (KEPT_SIDE * KEPT_SIDE * 4) as usize)
    );
    assert_ne!(frames[0], frames[5], "the pattern moves");
    assert!(
        frames[0].chunks(4).all(|px| px[3] == 255),
        "an opaque picture"
    );
    // One byte too many, or one too few, refuses the whole picture.
    let mut longer = output.clone();
    longer.push(0);
    assert!(helper::read_frames(&mut longer.as_slice(), 6).is_err());
    assert!(helper::read_frames(&mut &output[..output.len() - 1], 6).is_err());
}

/// A sequence as Central's encoder makes one with SVT-AV1 (Central's own
/// fixture, `central-picture-codec/tests/fixtures/opaque-sequence.avif`):
/// hidden frames inside its samples, and a last sample of three bytes that
/// only shows a frame decoded before. One picture comes out per sample, by
/// the durations the parent reads.
const SVT: &[u8] = include_bytes!("../../tests/data/picture-svt-hidden.avif");

#[test]
fn hidden_frames_and_a_shown_existing_frame_give_one_picture_a_sample() {
    assert_eq!(helper::durations(SVT), Some(vec![100, 300, 1500]));
    let mut input = Vec::new();
    baylee_worker::frame::write(&mut input, SVT, baylee_avif::Bounds::PICTURE.max_bytes).unwrap();
    let mut output = Vec::new();
    helper::serve(&mut input.as_slice(), &mut output).expect("decoded");
    let frames = helper::read_frames(&mut output.as_slice(), 3).expect("three frames");
    assert_eq!(frames.len(), 3);
    assert!(Timing::new(&[100, 300, 1500]).is_some());
}

/// What the decoder is handed is held to the picture's bounds before the
/// AV1 decoder sees a byte: garbage, a file too large, a still.
#[test]
fn the_decoder_refuses_what_is_not_a_picture_sequence() {
    let framed = |bytes: &[u8]| {
        let mut input = Vec::new();
        baylee_worker::frame::write(&mut input, bytes, usize::MAX).unwrap();
        input
    };
    for bad in [
        framed(b"not an avif at all"),
        framed(&png(256, [1, 2, 3, 255])),
        framed(&vec![0u8; baylee_avif::Bounds::PICTURE.max_bytes + 1]),
        framed(&MOVING[..MOVING.len() / 2]),
        Vec::new(),
    ] {
        let mut output = Vec::new();
        assert!(helper::serve(&mut bad.as_slice(), &mut output).is_err());
        assert!(output.is_empty(), "no pixels for a refused file");
    }
    assert_eq!(helper::durations(b"garbage"), None);
    assert_eq!(helper::durations(&png(256, [0; 4])), None);
}

/// A shell command as the decoder's stand-in.
#[cfg(unix)]
fn stand_in(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(script);
    command
}

/// The parent's side against stand-ins: one that does its part is taken,
/// and a crash, a hang, too many bytes, garbage and a failed exit are each
/// refused, the child killed and reaped within the deadline.
#[cfg(unix)]
#[test]
fn a_child_that_does_anything_but_its_part_is_refused() {
    let all = MOVING_MS.len() * FRAME_BYTES;
    let quick = Duration::from_secs(5);
    // Its part: read the file, write the frames, end well.
    let good = helper::decode(
        stand_in(&format!("cat >/dev/null; head -c {all} /dev/zero")),
        MOVING,
        quick,
    )
    .expect("a stand-in that does its part");
    assert_eq!(good.rgba.len(), 6);
    assert_eq!(good.durations, MOVING_MS.to_vec());

    let refused = |script: &str, deadline: Duration| {
        let started = Instant::now();
        let result = helper::decode(stand_in(script), MOVING, deadline);
        (result, started.elapsed())
    };
    // A crash.
    let (result, _) = refused("exit 3", quick);
    assert_eq!(result, Err(Refused::Child));
    // A hang: killed at the deadline, not waited for.
    let (result, took) = refused("exec sleep 60", Duration::from_millis(300));
    assert_eq!(result, Err(Refused::Child));
    assert!(took < Duration::from_secs(5), "the hang took {took:?}");
    // A hang after its frames.
    let (result, took) = refused(
        &format!("cat >/dev/null; head -c {all} /dev/zero; exec sleep 60"),
        Duration::from_millis(500),
    );
    assert_eq!(result, Err(Refused::Child));
    assert!(took < Duration::from_secs(5), "{took:?}");
    // Too many bytes: an endless stream.
    let (result, took) = refused("exec cat /dev/zero", quick);
    assert_eq!(result, Err(Refused::Child));
    assert!(took < Duration::from_secs(5), "{took:?}");
    // One byte too many.
    let (result, _) = refused(
        &format!("cat >/dev/null; head -c {} /dev/zero", all + 1),
        quick,
    );
    assert_eq!(result, Err(Refused::Child));
    // Garbage: too short to be the frames.
    let (result, _) = refused("cat >/dev/null; printf garbage", quick);
    assert_eq!(result, Err(Refused::Child));
    // The frames, and then a failed exit.
    let (result, _) = refused(
        &format!("cat >/dev/null; head -c {all} /dev/zero; exit 1"),
        quick,
    );
    assert_eq!(result, Err(Refused::Child));
    // A file the parent's own reading refuses never reaches a child.
    assert_eq!(
        helper::decode(Command::new("/nonexistent/decoder"), b"garbage", quick),
        Err(Refused::File)
    );
    // A child that cannot start.
    assert_eq!(
        helper::decode(Command::new("/nonexistent/decoder"), MOVING, quick),
        Err(Refused::Child)
    );
}

// --- The still, the origin, the download --------------------------------

#[test]
fn a_still_is_a_png_of_exactly_256_pixels() {
    let rgba = fetch::still(&png(256, [10, 20, 30, 255])).expect("a still");
    assert_eq!(rgba.len(), (KEPT_SIDE * KEPT_SIDE * 4) as usize);
    assert_eq!(&rgba[..4], &[10, 20, 30, 255]);
    assert!(fetch::still(&png(255, [0; 4])).is_none(), "another size");
    assert!(fetch::still(&png(512, [0; 4])).is_none(), "too large");
    assert!(fetch::still(b"\x89PNG\r\n\x1a\nbroken").is_none());
    assert!(fetch::still(b"GIF89a").is_none());
    // A header that claims a huge picture is refused before its pixels.
    let mut huge = png(256, [0; 4]);
    huge[16..20].copy_from_slice(&65_535u32.to_be_bytes());
    huge[20..24].copy_from_slice(&65_535u32.to_be_bytes());
    assert!(fetch::still(&huge).is_none());
    let mut big = png(256, [0; 4]);
    big.resize(MAX_STILL_BYTES_FOR_TEST + 1, 0);
    assert!(fetch::still(&big).is_none());
}

const MAX_STILL_BYTES_FOR_TEST: usize = baylee_client_core::portrait::MAX_STILL_BYTES;

#[test]
fn pictures_come_only_from_a_plain_https_or_loopback_origin() {
    for good in [
        "https://baylee.app",
        "https://pictures.example:8443",
        "http://127.0.0.1:28794",
        "http://[::1]:28794",
    ] {
        assert!(fetch::origin_ok(good), "{good}");
    }
    for bad in [
        "http://baylee.app",
        "http://10.0.0.1:80",
        "http://localhost:28794",
        "http://127.0.0.1",
        "https://baylee.app/",
        "https://baylee.app/path",
        "https://user@baylee.app",
        "https://baylee.app?x=1",
        "https://203.0.113.9",
        "file:///etc/passwd",
        "ftp://baylee.app",
        "",
    ] {
        assert!(!fetch::origin_ok(bad), "{bad}");
    }
    let key = [0xab; 16];
    assert_eq!(
        fetch::url("https://baylee.app", &key, File::Still),
        format!("https://baylee.app/api/v1/pictures/{}.png", "ab".repeat(16))
    );
    assert_eq!(
        fetch::url("https://baylee.app", &key, File::Moving),
        format!(
            "https://baylee.app/api/v1/pictures/{}.sequence.avif",
            "ab".repeat(16)
        )
    );
}

/// A one-shot HTTP server on loopback: answers one request with `answer`
/// and hands back what was asked.
fn server(answer: Vec<u8>) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut asked = Vec::new();
        let mut byte = [0u8; 1];
        while !asked.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
            asked.push(byte[0]);
        }
        let _ = stream.write_all(&answer);
        String::from_utf8_lossy(&asked).into_owned()
    });
    (origin, thread)
}

fn answer(status: &str, headers: &[(&str, String)], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for (name, value) in headers {
        let _ = writeln!(out, "{name}: {value}\r");
    }
    out.push_str("\r\n");
    let mut out = out.into_bytes();
    out.extend_from_slice(body);
    out
}

/// The download asks Central's path with no credential, takes only the
/// file it asked for within its bound, and follows no redirect.
#[test]
fn a_download_is_bounded_typed_and_never_redirected() {
    let key = [0x5a; 16];
    let body = png(256, [1, 2, 3, 255]);
    let typed = |kind: &str, length: usize| {
        vec![
            ("Content-Type", kind.to_owned()),
            ("Content-Length", length.to_string()),
        ]
    };
    let (origin, asked) = server(answer("200 OK", &typed("image/png", body.len()), &body));
    assert_eq!(
        fetch::download(&origin, &key, File::Still),
        Ok(body.clone())
    );
    let asked = asked.join().unwrap();
    assert!(
        asked.starts_with(&format!(
            "GET /api/v1/pictures/{}.png HTTP/1.1",
            "5a".repeat(16)
        )),
        "{asked}"
    );
    let lower = asked.to_ascii_lowercase();
    assert!(!lower.contains("authorization"), "{asked}");
    assert!(!lower.contains("cookie"), "{asked}");
    assert!(!lower.contains("sec-fetch"), "{asked}");

    let cases: Vec<(Vec<u8>, Result<(), fetch::Miss>)> = vec![
        (
            answer(
                "302 Found",
                &[("Location", "https://elsewhere.example/x.png".into())],
                b"",
            ),
            Err(fetch::Miss::Never),
        ),
        (answer("404 Not Found", &[], b""), Err(fetch::Miss::Never)),
        (
            answer("401 Unauthorized", &[], b""),
            Err(fetch::Miss::Never),
        ),
        (
            answer("429 Too Many Requests", &[], b""),
            Err(fetch::Miss::Later),
        ),
        (answer("503 Unavailable", &[], b""), Err(fetch::Miss::Later)),
        (
            answer("200 OK", &typed("text/html", body.len()), &body),
            Err(fetch::Miss::Never),
        ),
        (
            answer(
                "200 OK",
                &typed("image/png", MAX_STILL_BYTES_FOR_TEST + 1),
                b"",
            ),
            Err(fetch::Miss::Never),
        ),
        (
            answer(
                "200 OK",
                &[("Content-Type", "image/png".into())],
                &vec![7u8; MAX_STILL_BYTES_FOR_TEST + 10],
            ),
            Err(fetch::Miss::Never),
        ),
        (
            answer("200 OK", &typed("image/png; charset=x", body.len()), &body),
            Ok(()),
        ),
    ];
    for (reply, expected) in cases {
        let (origin, asked) = server(reply);
        let got = fetch::download(&origin, &key, File::Still).map(|_| ());
        assert_eq!(got, expected);
        let _ = asked.join();
    }
    // A moving picture's answer must say AVIF.
    let (origin, asked) = server(answer("200 OK", &typed("image/png", 4), b"abcd"));
    assert_eq!(
        fetch::download(&origin, &key, File::Moving),
        Err(fetch::Miss::Never)
    );
    assert!(asked.join().unwrap().contains(".sequence.avif"));
    // An origin that is not plain is never asked.
    assert_eq!(
        fetch::download("http://10.0.0.1:9", &key, File::Still),
        Err(fetch::Miss::Never)
    );
}

/// The cache keeps the raw bytes by key, reads back only its own size of
/// file, and forgets a key's files on demand.
#[test]
fn the_disk_keeps_bytes_by_key_within_bounds() {
    let dir = std::env::temp_dir().join(format!("baylee-portraits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shelf = fetch::Shelf::open(dir.clone());
    let key = [0x11; 16];
    assert!(shelf.read(&key, File::Still).is_none());
    shelf.write(&key, File::Still, b"still bytes");
    shelf.write(&key, File::Moving, b"moving bytes");
    assert_eq!(
        shelf.read(&key, File::Still).as_deref(),
        Some(&b"still bytes"[..])
    );
    assert!(dir.join(format!("{}.png", "11".repeat(16))).is_file());
    // A file larger than its bound is not read.
    std::fs::write(
        dir.join(format!("{}.png", "22".repeat(16))),
        vec![0u8; MAX_STILL_BYTES_FOR_TEST + 1],
    )
    .unwrap();
    assert!(shelf.read(&[0x22; 16], File::Still).is_none());
    // Files that are not the cache's are left alone.
    std::fs::write(dir.join("notes.txt"), b"mine").unwrap();
    shelf.trim();
    assert!(dir.join("notes.txt").is_file());
    shelf.remove(&key);
    assert!(shelf.read(&key, File::Still).is_none());
    assert!(shelf.read(&key, File::Moving).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

// --- Keys, the account, and the discs -----------------------------------

/// The near accounts are the account, its contacts and requests, and its
/// groups' members, each under its handle; nobody blocked, nobody else.
#[test]
fn only_the_account_and_its_contacts_are_asked() {
    assert_eq!(
        near_handles(Some("Me#0007"), None),
        BTreeMap::from([(7, "Me#0007".to_owned())])
    );
    assert!(near_handles(None, None).is_empty());
    assert!(near_handles(Some("no tag"), None).is_empty());
}

/// An app with the pictures' systems and a disc's dressing, at a clock the
/// test moves.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .init_resource::<Portraits>()
        .init_resource::<crate::hud::UiFonts>()
        .init_resource::<PictureWork>()
        .insert_resource(Assets::<Image>::default())
        .add_systems(Update, (lay_in, play).chain())
        .add_systems(PostUpdate, wear_portraits);
    app
}

/// A disc for `handle`, drawn and visible.
fn drawn(app: &mut App, handle: &str) -> Entity {
    let fonts = app.world().resource::<crate::hud::UiFonts>().clone();
    let mut commands = app.world_mut().commands();
    let disc = crate::shellkit::avatar::disc(&mut commands, &fonts, handle, 28.0);
    commands.entity(disc).insert(InheritedVisibility::VISIBLE);
    app.world_mut().flush();
    disc
}

/// The book says the account `handle` names wears `key`, as an answer
/// would, and the account is near this one under that handle.
fn wears(work: &mut PictureWork, handle: &str, key: PictureKey, moving: bool) {
    let tag = tag_of(handle).expect("a handle with a tag");
    work.handles.insert(tag, handle.to_owned());
    let near = BTreeSet::from([tag]);
    let ask = work.book.next(1, &near, &BTreeSet::new()).expect("an ask");
    work.book.answer(
        1,
        &ask,
        Answer::Pictures(vec![baylee_client_core::portrait::TaggedPicture {
            tag,
            key,
            moving,
        }]),
    );
}

fn face(app: &App, disc: Entity) -> Option<AvatarFace> {
    let children = app.world().get::<Children>(disc)?;
    children
        .iter()
        .find_map(|child| app.world().get::<AvatarFace>(child).cloned())
}

fn pixels(app: &App, image: &Handle<Image>) -> Vec<u8> {
    app.world()
        .resource::<Assets<Image>>()
        .get(image)
        .and_then(|image| image.data.clone())
        .expect("the image")
}

/// The still arrives and every disc of its account wears it; the moving
/// picture's frames arrive and play by their own durations; nothing is
/// written while the frame shown stays the same; under reduced motion, or
/// with its disc unseen, the still stands.
#[test]
#[allow(clippy::too_many_lines)] // one picture's life, still to moving to still
fn a_moving_picture_plays_in_its_disc_and_writes_only_on_a_new_frame() {
    let mut app = app();
    let key = [0x42; 16];
    let disc = drawn(&mut app, "Mia Wallace#00ff");
    let other = drawn(&mut app, "Alice#0001");
    // A gateway's own account that holds the same tag under another name.
    let stranger = drawn(&mut app, "Mallory#00ff");
    {
        let mut work = app.world_mut().resource_mut::<PictureWork>();
        wears(&mut work, "Mia Wallace#00ff", key, true);
        work.account = Some(1);
    }
    app.update();
    assert_eq!(face(&app, disc), Some(AvatarFace::Letters), "no still yet");

    let still = vec![9u8; (KEPT_SIDE * KEPT_SIDE * 4) as usize];
    let frames: Vec<Vec<u8>> = (0..6u8)
        .map(|n| vec![n * 10; (KEPT_SIDE * KEPT_SIDE * 4) as usize])
        .collect();
    deliver(
        &mut app,
        fetch::Done::Still {
            key,
            rgba: still.clone(),
        },
    );
    app.update();
    let image = match face(&app, disc) {
        Some(AvatarFace::Picture(image)) => image,
        other => panic!("the picture, not {other:?}"),
    };
    assert_eq!(face(&app, other), Some(AvatarFace::Letters));
    assert_eq!(
        face(&app, stranger),
        Some(AvatarFace::Letters),
        "the same tag under another name keeps its monogram"
    );
    assert_eq!(pixels(&app, &image), still);
    let portrait = app
        .world()
        .resource::<Portraits>()
        .portrait("Mia Wallace#00ff")
        .cloned()
        .unwrap();
    assert!(portrait.moving);

    deliver(
        &mut app,
        fetch::Done::Moving {
            key,
            frames: Frames {
                rgba: frames.clone(),
                durations: MOVING_MS.to_vec(),
            },
        },
    );
    // 50 ms a frame of the app: the first frame for two of them, then the
    // second for four (100 ms, 200 ms).
    let mut shown = Vec::new();
    let mut writes = 0;
    for _ in 0..12 {
        let tick = app.world().read_change_tick();
        app.update();
        if app
            .world()
            .resource_ref::<Assets<Image>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
        {
            writes += 1;
        }
        shown.push(pixels(&app, &image)[0]);
    }
    let distinct: Vec<u8> = {
        let mut seen = shown.clone();
        seen.dedup();
        seen
    };
    assert_eq!(
        distinct,
        vec![0, 10, 20],
        "frames by their own times: {shown:?}"
    );
    assert_eq!(
        writes,
        distinct.len(),
        "one write per frame change: {shown:?}"
    );
    assert_eq!(
        app.world()
            .resource::<Portraits>()
            .portrait("Mia Wallace#00ff"),
        Some(&portrait),
        "the image is the same: no disc is touched"
    );
    let faces_before = app.world().get::<Children>(disc).unwrap()[0];

    // Reduced motion: the still, at once, and then nothing more.
    let mut prefs = crate::prefs::Prefs::default();
    prefs.edit().reduce_motion = true;
    app.insert_resource(prefs);
    app.update();
    assert_eq!(pixels(&app, &image), still);
    let tick = app.world().read_change_tick();
    app.update();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Image>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick()),
        "a still picture writes nothing"
    );
    app.world_mut().remove_resource::<crate::prefs::Prefs>();

    // Its disc unseen: the still again.
    app.update();
    assert_ne!(pixels(&app, &image), still, "playing again");
    app.world_mut()
        .entity_mut(disc)
        .insert(InheritedVisibility::HIDDEN);
    app.update();
    assert_eq!(pixels(&app, &image), still);
    assert_eq!(
        app.world().get::<Children>(disc).unwrap()[0],
        faces_before,
        "the face was never rebuilt"
    );
}

/// Hands the worker's answer in, as `lay_in` takes it.
fn deliver(app: &mut App, done: fetch::Done) {
    app.world_mut()
        .resource_scope(|world, mut work: Mut<PictureWork>| {
            world.resource_scope(|world, mut portraits: Mut<Portraits>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                arrived(&mut work, done, 1, &mut portraits, &mut images);
            });
        });
}

/// An answer that drops a tag's picture, or gives it a new key, takes the
/// old picture away; a sign-out forgets every picture.
#[test]
fn a_lost_or_replaced_picture_leaves_and_a_sign_out_forgets_all() {
    let mut app = app();
    let disc = drawn(&mut app, "Mia#00ff");
    let key = [1; 16];
    {
        let mut work = app.world_mut().resource_mut::<PictureWork>();
        wears(&mut work, "Mia#00ff", key, false);
        work.account = Some(1);
    }
    deliver(
        &mut app,
        fetch::Done::Still {
            key,
            rgba: vec![1; (KEPT_SIDE * KEPT_SIDE * 4) as usize],
        },
    );
    app.update();
    assert!(matches!(face(&app, disc), Some(AvatarFace::Picture(_))));
    // Central's next answer: no picture for the tag.
    app.world_mut()
        .resource_scope(|world, mut work: Mut<PictureWork>| {
            world.resource_scope(|world, mut portraits: Mut<Portraits>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                let change = Change {
                    tag: 0xff,
                    was: Some(Worn { key, moving: false }),
                    now: None,
                };
                work.book.forget();
                apply(&mut work, &[change], &mut portraits, &mut images);
            });
        });
    app.update();
    assert_eq!(face(&app, disc), Some(AvatarFace::Letters));
    assert!(app.world().resource::<PictureWork>().pictures.is_empty());
}

/// The lookup is one request at a time, and after a refusal the account is
/// not asked again however many looks pass: no storm, and the chat's own
/// worker jobs are the only ones besides it.
#[test]
fn a_refused_lookup_is_not_asked_again() {
    let (handle, jobs) = crate::lobby::central::Handle::queued_for_test();
    let mut app = app();
    let near = BTreeSet::from([1, 2, 3]);
    let now = crate::social::now_ms();
    app.world_mut()
        .resource_scope(|world, mut work: Mut<PictureWork>| {
            let mut portraits = world.resource_mut::<Portraits>();
            look(
                &mut work,
                Some(handle.clone()),
                Some(1),
                &near,
                now,
                &mut portraits,
            );
            assert!(work.lookup.is_some());
            look(
                &mut work,
                Some(handle.clone()),
                Some(1),
                &near,
                now + 1000,
                &mut portraits,
            );
        });
    assert_eq!(jobs.try_iter().count(), 1, "one lookup out");
    // The worker answers: refused, as a Central without the native route does.
    app.world_mut()
        .resource_scope(|world, mut work: Mut<PictureWork>| {
            let lookup = work.lookup.as_ref().unwrap();
            let ask = lookup.ask.clone();
            let (send, receive) = mpsc::sync_channel(1);
            send.send((Answer::Refused, None)).unwrap();
            work.lookup = Some(Lookup {
                ask,
                receive: Mutex::new(receive),
            });
            let mut portraits = world.resource_mut::<Portraits>();
            assert!(take_answer(&mut work, now + 2000).is_empty());
            assert!(work.lookup.is_none());
            for second in 3..600 {
                look(
                    &mut work,
                    Some(handle.clone()),
                    Some(1),
                    &near,
                    now + second * 1000,
                    &mut portraits,
                );
            }
            assert!(work.lookup.is_none());
            assert_eq!(work.asked, 1);
        });
    assert_eq!(jobs.try_iter().count(), 0, "ten minutes, no second ask");
    assert!(app.world().resource::<Portraits>().is_empty());
}
