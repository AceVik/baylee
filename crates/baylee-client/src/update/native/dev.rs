//! A `dev-control` build's way to show every state of the update card
//! without a release: `POST /shell {"update":{…}}` feeds the face the
//! [`Report`]s the updater's thread would send, and `receive` takes them by
//! the thread's own path, so what the harness photographs is what a player
//! sees (`docs/client.md` §"Updating").
//!
//! - `{"stage":"found","version":"0.1.0-beta.9","size":150000000}`
//! - `{"stage":"downloading","received":45000000,"total":150000000,
//!   "rate":3200000,"left":33}` (`rate` and `left` optional)
//! - `{"stage":"verifying"}`, `{"stage":"unpacking"}`
//! - `{"stage":"staged","version":…}`: ready to install
//! - `{"stage":"failed","version":…,"reason":…}`: the download stopped
//! - `{"stage":"available","version":…,"why":"dev|off|unsigned|folder|other"}`
//! - `{"stage":"updated","version":…}`, `{"stage":"moved"}`,
//!   `{"stage":"kept"}`: the cards the start shows, set directly
//! - `{"stage":"simulate","version":…,"size":150000000,"ms":8000}`: a whole
//!   download in real time on a line whose speed wanders, its readings from
//!   the updater's own [`Meter`], then verifying, unpacking and ready.
//!
//! The release page it names is a loopback address, which the face opens
//! only as a stub's; nothing here asks the network.

use crate::update::{Shown, UpdateNotice, UpdatePlace};
use baylee_update::check::{Manual, Outcome};
use baylee_update::progress::{Meter, Stage, Transfer};
use baylee_update::service::Report;
use bevy::prelude::*;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The release page a fed report names.
const PAGE: &str = "http://127.0.0.1/";

/// The version a fed report names when the harness gives none.
const VERSION: &str = "0.1.0-beta.9";

/// A download's size when the harness gives none: about a release's.
const SIZE: u64 = 150_000_000;

type Feed = (Mutex<Sender<Report>>, Mutex<Receiver<Report>>);

fn feed_channel() -> &'static Feed {
    static FEED: OnceLock<Feed> = OnceLock::new();
    FEED.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        (Mutex::new(sender), Mutex::new(receiver))
    })
}

fn send(report: Report) {
    if let Ok(sender) = feed_channel().0.lock() {
        let _ = sender.send(report);
    }
}

/// What the harness fed since the last frame.
pub(super) fn fed() -> Vec<Report> {
    feed_channel()
        .1
        .lock()
        .map(|receiver| receiver.try_iter().collect())
        .unwrap_or_default()
}

/// `/shell`'s `"update"` knob: feeds the report it describes and answers a
/// JSON member for the reply; `None` when the body has no such knob.
pub(crate) fn feed(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let knob = value.get("update")?;
    let text = |key: &str| knob.get(key).and_then(Value::as_str);
    let number = |key: &str| knob.get(key).and_then(Value::as_u64);
    let version = text("version").unwrap_or(VERSION).to_owned();
    let page = PAGE.to_owned();
    let stage = text("stage").unwrap_or_default();
    let report = match stage {
        "found" => Report::Stage(Stage::Found {
            version,
            page,
            size: number("size").unwrap_or(SIZE),
        }),
        "downloading" => {
            let total = number("total").unwrap_or(SIZE);
            Report::Stage(Stage::Downloading(Transfer {
                received: number("received").unwrap_or(total / 3).min(total),
                total,
                rate: number("rate"),
                left: number("left"),
            }))
        }
        "verifying" => Report::Stage(Stage::Verifying),
        "unpacking" => Report::Stage(Stage::Unpacking),
        "staged" => Report::Done(Outcome::Staged { version, page }),
        "failed" => Report::Done(Outcome::Available {
            version,
            page,
            why: Manual::Download(
                text("reason")
                    .unwrap_or("connection reset by peer")
                    .to_owned(),
            ),
        }),
        "available" => Report::Done(Outcome::Available {
            version,
            page,
            why: match text("why") {
                Some("off") => Manual::Off,
                Some("unsigned") => Manual::Unsigned,
                Some("folder") => Manual::NotWritable("permission denied".into()),
                Some("other") => Manual::NoArchive,
                _ => Manual::DevBuild,
            },
        }),
        // The two cards no report leads to: the first start after an
        // update, and after a move (and its answer, which puts it away).
        "updated" => return Some(direct(Direct::Updated(version), stage)),
        "moved" => {
            return Some(direct(
                Direct::Moved(
                    "/Users/you/Applications/Baylee.app".into(),
                    "/Users/you/Downloads/Baylee.app".into(),
                ),
                stage,
            ));
        }
        "kept" => return Some(direct(Direct::Kept, stage)),
        "simulate" => {
            let length = Duration::from_millis(number("ms").unwrap_or(8_000).clamp(500, 120_000));
            return Some(
                match simulate(version, number("size").unwrap_or(SIZE), length) {
                    Ok(()) => "\"update\":\"simulate\"".to_owned(),
                    Err(err) => format!("\"update\":\"no thread: {err}\""),
                },
            );
        }
        _ => {
            return Some(
                "\"update\":\"unknown stage: found downloading verifying unpacking staged \
                 failed available updated moved kept simulate\""
                    .to_owned(),
            );
        }
    };
    send(report);
    Some(format!("\"update\":\"{stage}\""))
}

/// What the harness sets on the face directly: the states the updater's
/// thread never reports, because they come from the start.
pub(super) enum Direct {
    /// "Updated to X", as the first start after an update says it.
    Updated(String),
    /// The first start after a move: where it runs now, the old copy.
    Moved(String, String),
    /// The old copy kept: the move's question answered.
    Kept,
}

static DIRECT: Mutex<Vec<Direct>> = Mutex::new(Vec::new());

fn direct(change: Direct, stage: &str) -> String {
    if let Ok(mut queue) = DIRECT.lock() {
        queue.push(change);
    }
    format!("\"update\":\"{stage}\"")
}

/// Applies what [`direct`] queued, on the app's thread.
pub(super) fn apply_direct(mut notice: ResMut<UpdateNotice>, mut place: ResMut<UpdatePlace>) {
    let changes: Vec<Direct> = match DIRECT.lock() {
        Ok(mut queue) if !queue.is_empty() => queue.drain(..).collect(),
        _ => return,
    };
    for change in changes {
        match change {
            Direct::Updated(version) => notice.show(Shown::Updated {
                version,
                page: Some(PAGE.to_owned()),
            }),
            Direct::Moved(now, old) => place.moved = Some((now, old)),
            Direct::Kept => place.moved = None,
        }
    }
}

/// A whole download of `size` bytes over about `length`, on a thread of its
/// own as the updater's is: begun, found, the readings of the updater's
/// [`Meter`] on a line whose speed wanders by a quarter, then verifying,
/// unpacking and ready.
fn simulate(version: String, size: u64, length: Duration) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("update-simulation".into())
        .spawn(move || {
            send(Report::Checking { at: Instant::now() });
            send(Report::Stage(Stage::Found {
                version: version.clone(),
                page: PAGE.to_owned(),
                size,
            }));
            let began = Instant::now();
            let mut meter = Meter::new(size);
            if let Some(reading) = meter.feed(Duration::ZERO, 0) {
                send(Report::Stage(Stage::Downloading(reading)));
            }
            #[allow(clippy::cast_precision_loss)] // a simulated line's speed
            let mean = size as f64 / length.as_secs_f64();
            let (mut received, mut last) = (0_u64, Duration::ZERO);
            while received < size {
                std::thread::sleep(Duration::from_millis(16));
                let at = began.elapsed();
                let wander = 1.0 + 0.25 * (at.as_secs_f64() * 1.3).sin();
                let step = mean * wander * at.saturating_sub(last).as_secs_f64();
                last = at;
                // A positive step far below `u64::MAX`.
                received = (received + step as u64).min(size);
                if let Some(reading) = meter.feed(at, received) {
                    send(Report::Stage(Stage::Downloading(reading)));
                }
            }
            send(Report::Stage(Stage::Verifying));
            std::thread::sleep(Duration::from_millis(1_400));
            send(Report::Stage(Stage::Unpacking));
            std::thread::sleep(Duration::from_millis(1_000));
            send(Report::Done(Outcome::Staged {
                version,
                page: PAGE.to_owned(),
            }));
        })
        .map(|_| ())
}
