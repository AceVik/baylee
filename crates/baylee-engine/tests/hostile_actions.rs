//! What a guest's frame can do to the reader of an action, in the real decoder.
//!
//! The gateway forwards a seat's frames unread, so these bytes reach the engine
//! as they were sent, up to the seat socket's limit of 64 KiB. `postcard` keeps
//! no count of depth, so an action must not be able to nest, and a list may not
//! allocate on what it claims (`baylee_core::wire`).

#![allow(clippy::cast_possible_truncation)] // bytes cut from words on purpose

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_core::wire::{decode, encode};
use baylee_engine::choice::{PaymentStep, PlayerAction};

/// The seat socket's `MAX_SEAT_FRAME`.
const FRAME_LIMIT: usize = 64 * 1024;

/// A stack far smaller than a thread's, on which any recursion a frame could
/// drive to the frame's own depth would overflow.
const SMALL_STACK: usize = 64 * 1024;

/// Tracks the bytes held and the most held since the last reset.
struct Counting;

static HELD: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call goes straight to `System` with the arguments it was given,
// so the allocator's contract is `System`'s. The counters allocate nothing.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let held = HELD.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(held, Ordering::Relaxed);
        // SAFETY: the caller's layout is passed on unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HELD.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: `ptr` came from `System.alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn peak_of(run: impl FnOnce()) -> usize {
    let before = HELD.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    run();
    PEAK.load(Ordering::Relaxed).saturating_sub(before)
}

fn on_a_small_stack(run: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(run)
        .expect("a thread")
        .join()
        .expect("nothing overflowed or panicked");
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    while value >= 0x80 {
        out.push((value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
    out
}

/// The bytes that open a `PayWindow`, before its steps' length.
fn pay_window_tag() -> Vec<u8> {
    let empty = encode(&PlayerAction::PayWindow { steps: Vec::new() });
    empty[..empty.len() - 1].to_vec()
}

/// What `PayWindow { steps: [PayWindow { steps: [ …` would be if it could be
/// written, to the frame limit: thirty thousand levels and more.
fn nested_payment() -> Vec<u8> {
    let tag = pay_window_tag();
    let mut frame = Vec::with_capacity(FRAME_LIMIT);
    while frame.len() + tag.len() < FRAME_LIMIT {
        frame.extend(&tag);
        frame.push(1);
    }
    frame
}

/// The `PayWindow` of the frame above is no step, so the reader stops at the
/// first one, on a stack that deep recursion would overflow.
#[test]
fn a_payment_nested_to_the_frame_limit_cannot_be_read() {
    let frame = nested_payment();
    assert!(frame.len() > FRAME_LIMIT - 8, "a frame at the limit");
    assert!(frame.len() / (pay_window_tag().len() + 1) > 20_000);
    on_a_small_stack(move || {
        assert!(decode::<PlayerAction>(&frame).is_err());
    });
}

/// The same with an honest step first, so the reader is inside the list when it
/// meets the payment that may not be there.
#[test]
fn a_payment_behind_a_real_step_cannot_be_read() {
    let mut frame = pay_window_tag();
    frame.push(2);
    frame.extend(encode(&PaymentStep::YesNo(true)));
    frame.extend(nested_payment());
    frame.truncate(FRAME_LIMIT);
    on_a_small_stack(move || {
        assert!(decode::<PlayerAction>(&frame).is_err());
    });
}

/// `PlayerAction` holds lists of lists and nothing that holds an action, so no
/// other variant can be driven deeper than its type: the deepest is `Arrange`,
/// two lists down. Lists of empty lists to the frame limit read flat.
#[test]
fn the_deepest_lists_an_action_has_stay_flat() {
    let arrange = encode(&PlayerAction::Arrange {
        piles: vec![Vec::new()],
    });
    let tag = &arrange[..arrange.len() - 2];
    let mut frame = tag.to_vec();
    let piles = FRAME_LIMIT - tag.len() - 3;
    frame.extend(varint(piles as u64));
    frame.resize(frame.len() + piles, 0);
    assert!(frame.len() <= FRAME_LIMIT);
    on_a_small_stack(move || {
        let read = decode::<PlayerAction>(&frame).expect("empty piles are an answer's shape");
        let PlayerAction::Arrange { piles: read } = read else {
            panic!("an arrangement");
        };
        assert_eq!(read.len(), piles);
    });
}

/// A list that claims more than the frame holds allocates for what arrives and
/// not for what it says, on every list an action has.
#[test]
fn a_claimed_list_allocates_nothing_for_the_claim() {
    let lists = [
        encode(&PlayerAction::ChooseObjects {
            objects: Vec::new(),
        }),
        encode(&PlayerAction::PayWindow { steps: Vec::new() }),
        encode(&PlayerAction::Arrange { piles: Vec::new() }),
        encode(&PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        }),
    ];
    for empty in lists {
        let tag = &empty[..empty.len() - 1];
        for claim in [1 << 20, 1 << 40, u64::MAX] {
            let mut frame = tag.to_vec();
            frame.extend(varint(claim));
            frame.resize(FRAME_LIMIT, 0x01);
            let held = peak_of(|| {
                let _ = decode::<PlayerAction>(&frame);
            });
            assert!(held < 4 << 20, "a claim of {claim} held {held} bytes");
        }
    }
}

fn every_step() -> Vec<PaymentStep> {
    vec![
        PaymentStep::ActivateManaAbility {
            source: ObjectId::new(3, 1),
        },
        PaymentStep::ActivateAbility {
            source: ObjectId::new(4, 0),
            ability_index: 2,
        },
        PaymentStep::TakeGrantedAction {
            id: baylee_core::ids::GrantedActionId::new(9),
        },
        PaymentStep::ChooseColor(ManaColor::Blue),
        PaymentStep::ChooseMode(3),
        PaymentStep::ChooseNumber(70_000),
        PaymentStep::YesNo(true),
        PaymentStep::ChooseObjects {
            objects: vec![ObjectId::new(7, 2)],
        },
        PaymentStep::ChoosePlayer(PlayerId::new(1)),
    ]
}

#[test]
fn a_payment_survives_the_wire() {
    let action = PlayerAction::PayWindow {
        steps: every_step(),
    };
    assert_eq!(decode::<PlayerAction>(&encode(&action)), Ok(action));
}

/// Each step is the action of its name: applied as that action, and spelled as
/// that action in a record's JSON, so a record from before this type reads and
/// a replay of it asks the same questions.
#[test]
fn a_step_is_spelled_as_the_action_of_its_name() {
    for step in every_step() {
        let action = PlayerAction::from(step.clone());
        assert_eq!(
            serde_json::to_value(&step).expect("a step writes"),
            serde_json::to_value(&action).expect("an action writes"),
        );
    }
    let written = r#"{"PayWindow":{"steps":[{"ActivateManaAbility":{"source":12}},{"ChooseColor":"Blue"},{"YesNo":true}]}}"#;
    let read: PlayerAction = serde_json::from_str(written).expect("a record's payment reads");
    assert_eq!(serde_json::to_string(&read).expect("it writes"), written);
}

/// A step that never was one stays refused, now at the reader's door: a record
/// cannot hold one, since the engine refused it when it came.
#[test]
fn a_record_cannot_name_a_step_that_is_no_step() {
    for written in [
        r#"{"PayWindow":{"steps":["PassPriority"]}}"#,
        r#"{"PayWindow":{"steps":[{"PayWindow":{"steps":[]}}]}}"#,
    ] {
        assert!(serde_json::from_str::<PlayerAction>(written).is_err());
    }
}

/// Bytes from a seeded stream, a million of them, as a frame: an error or an
/// action, never a panic.
#[test]
fn a_million_made_up_actions_are_survived() {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut frame = Vec::with_capacity(64);
    let mut read = 0_u32;
    for _ in 0..1_000_000 {
        frame.clear();
        let length = (next() % 48) as usize;
        frame.extend((0..length).map(|_| next() as u8));
        read += u32::from(decode::<PlayerAction>(&frame).is_ok());
    }
    assert!(read > 0, "some of them were actions");
}

/// An offer as the engine sends it (a priority that names what restricted
/// mana may pay for, protocol 43), and the same offer cut and damaged: a
/// client reads it whole or refuses it, never panics, and never holds memory
/// for a list it only claims.
mod offers {
    use super::{FRAME_LIMIT, peak_of, varint};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::wire::{decode, encode};
    use baylee_engine::choice::{CastModeKind, LegalActions, Pending, RestrictedTaps, Spend};

    fn offer() -> Pending {
        let card = ObjectId::new(50, 0);
        let mut green = [0; 6];
        green[4] = 1;
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                payable: vec![card],
                restricted_taps: vec![
                    RestrictedTaps {
                        taps: vec![(ObjectId::new(1, 0), 1), (ObjectId::new(4, 0), 1)],
                        spends: vec![Spend::Cast(card, CastModeKind::Normal), Spend::Owed],
                    },
                    RestrictedTaps {
                        taps: vec![(ObjectId::new(2, 0), 0)],
                        spends: vec![Spend::Cast(card, CastModeKind::Face(1))],
                    },
                    RestrictedTaps {
                        taps: vec![(ObjectId::new(3, 0), 1)],
                        spends: vec![Spend::Activate(ObjectId::new(9, 0))],
                    },
                ],
                restricted_floating: vec![(Spend::Cast(card, CastModeKind::Normal), green)],
                unpaid_suspend: vec![(
                    ObjectId::new(61, 0),
                    baylee_core::mana::ManaCost::parse("{1}{U}"),
                )],
                ..LegalActions::default()
            }),
        }
    }

    #[test]
    fn an_offer_naming_restricted_mana_survives_the_wire() {
        let frame = encode(&offer());
        let read = decode::<Pending>(&frame).expect("the offer reads");
        assert_eq!(encode(&read), frame, "and reads as it was written");
        let Pending::Priority { legal, .. } = read else {
            panic!("a priority");
        };
        assert_eq!(legal.restricted_taps.len(), 3);
        assert_eq!(legal.restricted_floating.len(), 1);
        assert_eq!(
            legal.unpaid_suspend,
            vec![(
                ObjectId::new(61, 0),
                baylee_core::mana::ManaCost::parse("{1}{U}")
            )]
        );
        assert_eq!(
            legal.restricted_taps_for(Spend::Owed),
            vec![(ObjectId::new(1, 0), 1), (ObjectId::new(4, 0), 1)]
        );
    }

    #[test]
    fn every_cut_and_every_flipped_byte_of_an_offer_is_survived() {
        let frame = encode(&offer());
        for cut in 0..frame.len() {
            assert!(
                decode::<Pending>(&frame[..cut]).is_err(),
                "a cut at {cut} read"
            );
        }
        for at in 0..frame.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut damaged = frame.clone();
                damaged[at] ^= flip;
                let _ = decode::<Pending>(&damaged);
            }
        }
    }

    /// The two lists are the offer's last fields: each claims more than the
    /// frame holds, and so does a class's list of taps, and the reader holds
    /// what arrives and not what is claimed.
    #[test]
    fn a_claimed_restricted_list_allocates_nothing_for_the_claim() {
        let quiet = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions::default()),
        };
        let empty = encode(&quiet);
        assert_eq!(
            &empty[empty.len() - 2..],
            &[0, 0],
            "both lists close the offer"
        );
        let lists = &empty[..empty.len() - 2];
        let floating = &empty[..empty.len() - 1];
        let mut one_class = lists.to_vec();
        one_class.extend(varint(1));
        for head in [lists, floating, &one_class[..]] {
            for claim in [1 << 20, 1 << 40, u64::MAX] {
                let mut frame = head.to_vec();
                frame.extend(varint(claim));
                frame.resize(FRAME_LIMIT, 0x01);
                let held = peak_of(|| {
                    assert!(decode::<Pending>(&frame).is_err());
                });
                assert!(held < 4 << 20, "a claim of {claim} held {held} bytes");
            }
        }
    }
}

/// `LegalActions::unpaid_suspend` (protocol 44): a list of cards with their
/// suspend costs, as a client reads it from a hostile host.
mod unpaid_suspend {
    use super::{FRAME_LIMIT, peak_of, varint};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::mana::ManaCost;
    use baylee_core::wire::{decode, encode};
    use baylee_engine::choice::{LegalActions, Pending};

    fn offer(unpaid_suspend: Vec<(ObjectId, ManaCost)>) -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                mana_abilities: vec![ObjectId::new(1, 0)],
                unpaid_suspend,
                ..LegalActions::default()
            }),
        }
    }

    /// Where a list of the offer stands: the first byte at which the offer
    /// written with `a` and with `b` differ.
    fn first_difference(a: &[u8], b: &[u8]) -> usize {
        a.iter()
            .zip(b)
            .position(|(x, y)| x != y)
            .expect("the two frames differ")
    }

    #[test]
    fn an_unpaid_suspend_survives_the_wire_and_every_damage() {
        let frame = encode(&offer(vec![
            (ObjectId::new(61, 0), ManaCost::parse("{U}")),
            (ObjectId::new(62, 0), ManaCost::parse("{1}{B}")),
        ]));
        let read = decode::<Pending>(&frame).expect("the offer reads");
        assert_eq!(encode(&read), frame, "and reads as it was written");
        for cut in 0..frame.len() {
            assert!(
                decode::<Pending>(&frame[..cut]).is_err(),
                "a cut at {cut} read"
            );
        }
        for at in 0..frame.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut damaged = frame.clone();
                damaged[at] ^= flip;
                let _ = decode::<Pending>(&damaged);
            }
        }
    }

    /// The list claims more cards than the frame holds, and then one card's
    /// cost claims a longer notation than is left: the reader holds what
    /// arrives and not what is claimed.
    #[test]
    fn a_claimed_list_or_cost_allocates_nothing_for_the_claim() {
        let none = encode(&offer(Vec::new()));
        let blank = encode(&offer(vec![(ObjectId::new(61, 0), ManaCost::parse(""))]));
        let list = first_difference(&none, &blank);
        assert_eq!((none[list], blank[list]), (0, 1), "the list's length");
        let priced = encode(&offer(vec![(ObjectId::new(61, 0), ManaCost::parse("{U}"))]));
        let cost = first_difference(&blank, &priced);
        assert_eq!(blank[cost], 0, "the cost's length");
        for head in [&none[..list], &blank[..cost]] {
            for claim in [1 << 20, 1 << 40, u64::MAX] {
                let mut frame = head.to_vec();
                frame.extend(varint(claim));
                frame.resize(FRAME_LIMIT, 0x01);
                let held = peak_of(|| {
                    assert!(decode::<Pending>(&frame).is_err());
                });
                assert!(held < 4 << 20, "a claim of {claim} held {held} bytes");
            }
        }
    }
}
