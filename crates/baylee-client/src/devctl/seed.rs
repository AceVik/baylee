//! `/seed {"friends":500,"lines":400,"stack":60}`: the large, repeatable data a
//! fixed scenario stands on, without a gateway. Every field is optional.
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

/// A fixed time, so two runs of a scenario draw the same rows.
const SEEDED_AT_MS: u64 = 1_790_000_000_000;

/// What was asked for.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Seed {
    friends: usize,
    lines: usize,
    stack: usize,
}

/// Answers one `/seed`.
pub(super) fn answer(body: &str, host: Option<&mut SocialHost>, duel: Option<&mut Duel>) -> String {
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
    format!("{{\"ok\":true,\"frames\":{social},\"stack\":{stack}}}")
}

fn seed_the_machine(machine: &mut Social, seed: &Seed) -> usize {
    let mut frames = Vec::with_capacity(seed.lines + 1);
    if seed.friends > 0 {
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

/// A line of the lobby's chat, the one conversation not sealed end to end.
fn line(n: usize) -> Down {
    let tag = format!("{:04x}", 0x1000 + n % 7);
    let text = format!("Line {n}: a sentence long enough to wrap once in the chat's window.");
    Down::Global {
        id: format!("seed-{n:08}"),
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
