//! `/seed {"friends":500,"lines":400,"stack":60,"portraits":{"1000":"moving"}}`:
//! the large, repeatable data a fixed scenario stands on, without a gateway.
//! Every field is optional. `portraits` lays a profile picture into the discs
//! of each tag (`"still"` or `"moving"`) through the shipped path, from the
//! cache directory instead of Central (`portraits::seed`).
//!
//! The rows are made here and go in by the doors a host uses, so what draws
//! them is the shipped code: frames into the social machine, a view into the
//! duel. It is what `cargo tools pgo client` trains on and what a measurement
//! of a long list reads. The machine's answers go nowhere: a seed talks to
//! nobody.

use crate::Duel;
use crate::social::SocialHost;
use baylee_client_core::social::{Body, Social};
use baylee_core::ids::ObjectId;
use baylee_protocol::social::{Down, Friend, Person, Presence, Sender, Shown, Snapshot};

/// Where `portraits` go: the pictures' work (`crate::portraits`).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(super) type Pictures = crate::portraits::PictureWork;

/// Phones draw no pictures yet: a resource no one inserts.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[derive(bevy::prelude::Resource)]
pub(super) struct Pictures;

/// A fixed time, so two runs of a scenario draw the same rows.
const SEEDED_AT_MS: u64 = 1_790_000_000_000;

/// What was asked for.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Seed {
    friends: usize,
    lines: usize,
    stack: usize,
    /// Pictures by tag: `"still"` or `"moving"`.
    portraits: std::collections::BTreeMap<String, String>,
}

/// Answers one `/seed`.
pub(super) fn answer(
    body: &str,
    host: Option<&mut SocialHost>,
    duel: Option<&mut Duel>,
    pictures: Option<&mut Pictures>,
) -> String {
    let Ok(seed) = serde_json::from_str::<Seed>(body) else {
        return "{\"error\":\"not a seed\"}".to_string();
    };
    let mut social = 0;
    if seed.friends + seed.lines > 0 {
        let Some(machine) = host.and_then(|host| host.machine.as_mut()) else {
            return "{\"error\":\"no social session\"}".to_string();
        };
        social = seed_the_machine(machine, &seed);
    }
    let stack = if seed.stack > 0 {
        let Some(duel) = duel else {
            return "{\"error\":\"no duel\"}".to_string();
        };
        pile_the_stack(duel, seed.stack)
    } else {
        0
    };
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let portraits = if seed.portraits.is_empty() {
        0
    } else {
        let Some(work) = pictures else {
            return "{\"error\":\"no pictures\"}".to_string();
        };
        let wanted: Vec<(u32, crate::portraits::Seeded)> = seed
            .portraits
            .iter()
            .filter_map(|(tag, kind)| {
                Some((baylee_client_core::portrait::tag_of(tag)?, seeded(kind)?))
            })
            .collect();
        match crate::portraits::seed(work, &wanted, &still(), MOVING) {
            Ok(laid) => laid,
            Err(why) => return format!("{{\"error\":\"{why}\"}}"),
        }
    };
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let portraits = {
        let _ = pictures;
        0
    };
    format!("{{\"ok\":true,\"frames\":{social},\"stack\":{stack},\"portraits\":{portraits}}}")
}

/// What a seed's word for a picture asks: `"still"` or `"moving"` (a
/// fixture), or a real key's 32 hex digits, `":moving"` after them for one
/// that moves (fetched from the development Central's website).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn seeded(kind: &str) -> Option<crate::portraits::Seeded> {
    use crate::portraits::Seeded;
    match kind {
        "still" => Some(Seeded::Fixture { moving: false }),
        "moving" => Some(Seeded::Fixture { moving: true }),
        _ => {
            let (hex, moving) = kind
                .strip_suffix(":moving")
                .map_or((kind, false), |hex| (hex, true));
            let key = baylee_client_core::portrait::parse_key(hex)?;
            Some(Seeded::Key { key, moving })
        }
    }
}

/// The moving picture a seed lays in: the client's test fixture (256 px,
/// six frames of `testsrc2`, nobody's picture).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const MOVING: &[u8] = include_bytes!("../../tests/data/picture-moving.avif");

/// The still a seed lays in: 256 px of rings in two of the felt's colours,
/// made here, as a PNG like Central's master.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn still() -> Vec<u8> {
    let image = image::RgbaImage::from_fn(256, 256, |x, y| {
        let (dx, dy) = (i64::from(x) - 128, i64::from(y) - 96);
        let ring = ((dx * dx + dy * dy).isqrt() / 18) % 2 == 0;
        if ring {
            image::Rgba([214, 168, 74, 255])
        } else {
            image::Rgba([28, 92, 104, 255])
        }
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    let _ = image.write_to(&mut bytes, image::ImageFormat::Png);
    bytes.into_inner()
}

fn seed_the_machine(machine: &mut Social, seed: &Seed) -> usize {
    let mut frames = Vec::with_capacity(seed.lines + 1);
    if seed.friends > 0 {
        // A guest has no search over its friends, and offline everyone is one.
        frames.push(Down::Welcome {
            tag: machine.me().tag.clone(),
            handle: "Seeded#0001".to_owned(),
            device: machine.device_id().to_owned(),
            guest: false,
        });
        frames.push(Down::Snapshot(Snapshot {
            friends: (0..seed.friends).map(friend).collect(),
            ..Snapshot::default()
        }));
    }
    frames.extend((0..seed.lines).map(line));
    let taken = frames.len();
    for frame in frames {
        let _ = machine.on_frame(frame, SEEDED_AT_MS, &mut crate::social::OsEntropy);
    }
    taken
}

/// One friend in four is online, so both sections are long.
fn friend(n: usize) -> Friend {
    let tag = format!("{:04x}", 0x1000 + n);
    Friend {
        person: Person {
            handle: format!("Friend {n}#{tag}"),
            tag,
            guest: false,
        },
        since: SEEDED_AT_MS,
        presence: Presence {
            status: if n.is_multiple_of(4) {
                Shown::Online
            } else {
                Shown::Offline
            },
            place: None,
        },
    }
}

/// A seeded line's id: a v7 UUID of the seed's own time, the same on every
/// run. The machine drops a lobby line whose id is no v7 UUID
/// (`sealing::uuid_ms`), which is what `seed-00000000` was: every seeded
/// line was dropped unseen.
fn line_id(n: usize) -> String {
    let mut rest = [0u8; 10];
    rest[2..].copy_from_slice(&(n as u64).to_be_bytes());
    uuid::Builder::from_unix_timestamp_millis(SEEDED_AT_MS + n as u64, &rest)
        .into_uuid()
        .to_string()
}

/// A line of the lobby's chat, the one conversation not sealed end to end.
fn line(n: usize) -> Down {
    let tag = format!("{:04x}", 0x1000 + n % 7);
    let text = format!("Line {n}: a sentence long enough to wrap once in the chat's window.");
    Down::Global {
        id: line_id(n),
        at: SEEDED_AT_MS + n as u64,
        from: Sender {
            handle: format!("Speaker#{tag}"),
            tag,
            device: "seed".to_string(),
            guest: false,
        },
        body: serde_json::to_string(&Body {
            text,
            links: Vec::new(),
            mentions: Vec::new(),
            emote: false,
        })
        .unwrap_or_default(),
    }
}

/// Stands `count` copies of a permanent on the stack, in the view the table
/// already holds. The engine knows nothing of them; its next view takes them
/// away. Answers how many stood.
fn pile_the_stack(duel: &mut Duel, count: usize) -> usize {
    let Some(mut view) = duel.view.clone() else {
        return 0;
    };
    let Some(object) = view.battlefield.first().cloned() else {
        return 0;
    };
    view.stack = (0..count)
        .map(|n| {
            let mut copy = object.clone();
            copy.id = ObjectId::queued_trigger(u32::try_from(n).unwrap_or(0));
            copy
        })
        .collect();
    duel.receive_view(view);
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::social::{ChatPrefs, Me};
    use baylee_protocol::social::Conv;

    /// What a seed asks for is what the chat then holds: the machine keeps
    /// every seeded line (it drops one whose id is no v7 UUID), and the
    /// seeded friends stand in the list.
    #[test]
    fn a_seeded_line_and_friend_are_kept_by_the_machine() {
        let (mut machine, _) = Social::start(
            Me {
                tag: "0001".into(),
                handle: "Seeded#0001".into(),
                guest: true,
                gateway: "http://127.0.0.1:1".into(),
            },
            None,
            None,
            ChatPrefs::default(),
            SEEDED_AT_MS,
            &mut crate::social::OsEntropy,
        );
        let seed = Seed {
            friends: 5,
            lines: 9,
            ..Seed::default()
        };
        assert_eq!(seed_the_machine(&mut machine, &seed), 11);
        let lobby = machine
            .conversation(&Conv::Global)
            .expect("the lobby's conversation");
        assert_eq!(lobby.lines.len(), 9);
        for n in 0..5 {
            let tag = format!("{:04x}", 0x1000 + n);
            assert!(machine.contacts().friend(&tag).is_some(), "{tag}");
        }
        assert_ne!(line_id(1), line_id(2));
        assert_eq!(line_id(3), line_id(3));
    }
}
