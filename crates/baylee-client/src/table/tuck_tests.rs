//! What lies under a card (#305), from a view through the board model to the
//! placements: an aura, equipment or fortification lies under its host,
//! peeking out towards the middle of the table by what the rows leave it,
//! turned with a tapped host, and the host stands apart from its bare twins
//! with no count to badge. What is ahead of the host — the band its seat's
//! bar is written on, or the next row — is read from the layout, never from
//! the arithmetic `tuck` does.

use super::*;
use baylee_client_core::layout::LaneKind;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::Defender;
use baylee_core::types::TypeSet;
use baylee_view::{AttackerView, ObjectStatus, PublicObject};

/// The first id of seat `seat`'s objects; each row's host, and what hangs
/// off it, are counted from there.
fn base(seat: u8) -> u32 {
    1000 * (u32::from(seat) + 1)
}

/// Seat `seat`'s host in `lane`.
fn host(seat: u8, lane: LaneKind) -> ObjectId {
    let row = match lane {
        LaneKind::Creatures => 1,
        LaneKind::Support => 10,
        LaneKind::Lands => 20,
    };
    ObjectId::new(base(seat) + row, 0)
}

/// The `k`th card hanging off seat `seat`'s host in `lane`, from 0.
fn hanging(seat: u8, lane: LaneKind, k: u32) -> ObjectId {
    ObjectId::new(host(seat, lane).slot() * 10 + k, 0)
}

/// A table of `seats` chairs at `aspect`. Every seat has a host in each of
/// its three rows with `under` auras on it, and two bare twins of the
/// creature beside it, tapped or untapped with it; the creature host
/// attacks the next seat when `staged`.
fn table(seats: u8, aspect: f32, under: u32, tapped: bool, staged: bool) -> Duel {
    let status = if tapped {
        ObjectStatus::TAPPED
    } else {
        ObjectStatus::NONE
    };
    let mut builder = ViewBuilder::new(seats);
    let mut attackers = Vec::new();
    for seat in 0..seats {
        let mut objs: Vec<PublicObject> = Vec::new();
        for (lane, name, types) in [
            (LaneKind::Creatures, "Bear", TypeSet::CREATURE),
            (LaneKind::Support, "Relic", TypeSet::ARTIFACT),
            (LaneKind::Lands, "Field", TypeSet::LAND),
        ] {
            let mut card = token(host(seat, lane).slot(), seat, name, 2, 2);
            card.types = types;
            if lane != LaneKind::Creatures {
                (card.power, card.toughness) = (None, None);
            }
            card.status = status;
            objs.push(card);
            for k in 0..under {
                let mut aura = token(hanging(seat, lane, k).slot(), seat, "Aura", 0, 0);
                aura.types = TypeSet::ENCHANTMENT;
                (aura.power, aura.toughness) = (None, None);
                aura.attached_to = Some(host(seat, lane));
                objs.push(aura);
            }
        }
        for twin in [2, 3] {
            let mut bear = token(base(seat) + twin, seat, "Bear", 2, 2);
            bear.status = status;
            objs.push(bear);
        }
        builder = builder.with_battlefield(seat, objs);
        if staged {
            attackers.push(AttackerView {
                creature: host(seat, LaneKind::Creatures),
                defending: Defender::Player(PlayerId::new((seat + 1) % seats)),
                blocked: false,
            });
        }
    }
    let mut duel = Duel {
        view: Some(builder.with_combat(attackers, Vec::new()).build()),
        canvas_aspect: Some(aspect),
        ..Duel::default()
    };
    crate::rebuild_board(&mut duel);
    duel
}

fn placed(all: &[Placement], object: ObjectId) -> &Placement {
    all.iter()
        .find(|p| p.object == object)
        .unwrap_or_else(|| panic!("{object:?} is on the table"))
}

/// An aura lies under the creature it enchants and peeks out past it by
/// `ATTACH_PEEK`, straight towards the middle of the table where the print
/// has its name, a hair under it; the host stays out of its twins' pile and
/// carries no count, so no badge stands over what peeks out.
#[test]
fn an_aura_peeks_out_from_under_its_host_towards_the_middle() {
    let duel = table(2, 16.0 / 9.0, 1, false, false);
    let all = placements(&duel);
    for seat in 0..2 {
        let host = placed(&all, host(seat, LaneKind::Creatures));
        let aura = placed(&all, hanging(seat, LaneKind::Creatures, 0));
        let step = aura.position - host.position;
        assert!(
            (step - host.slot.forward() * ATTACH_PEEK).length() < 1e-4,
            "seat {seat}: the aura lies {step} from its host"
        );
        assert!(
            aura.lift < host.lift,
            "seat {seat}: the aura is over its host"
        );
        assert!(!aura.tapped && aura.shown);
        assert_eq!((host.count, host.badge), (1, 0), "seat {seat}: the host");
        assert_eq!((aura.count, aura.badge), (1, 0), "seat {seat}: the aura");
        let twins = placed(&all, ObjectId::new(base(seat) + 2, 0));
        assert_eq!(twins.count, 2, "seat {seat}: the bare twins pile");
    }
    // Each aura is drawn once, under its host and in no row of its own:
    // three hosts, three auras and a pile of twins a seat.
    let objects: Vec<ObjectId> = all.iter().map(|p| p.object).collect();
    assert_eq!(objects.len(), 2 * 7, "{objects:?}");
    for (i, object) in objects.iter().enumerate() {
        assert!(!objects[..i].contains(object), "{object:?} is drawn twice");
    }
}

/// A tapped host turns what is under it with it (client-41, on #305):
/// upright behind a tapped card an aura would stand out on both sides of it,
/// into the air where the next row's plate stands.
#[test]
fn what_lies_under_a_tapped_host_turns_with_it() {
    let duel = table(4, 16.0 / 9.0, 2, true, false);
    let all = placements(&duel);
    let mut seen = 0;
    for seat in 0..4 {
        for lane in LaneKind::ALL {
            for k in 0..2 {
                let aura = placed(&all, hanging(seat, lane, k));
                assert!(
                    aura.tapped,
                    "seat {seat}, {lane:?}: aura {k} stands upright"
                );
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 24);
}

/// Nothing tucked under a card reaches what stands ahead of its host: the
/// band the seat's bar is written on, ahead of the creature row, and the
/// next row ahead of either other (client-41, on #305; the card-sized
/// reading of `no_card_reaches_the_band_its_seat_writes_on`, which measures
/// a staged creature's front edge to within 0.0093 of the band at a ring).
/// Where a host already stands past it, nothing peeks out at all.
///
/// What is under a host peeks out whole or not at all, and where it folds
/// away the host wears the attachment mark with how many lie under it, and
/// only there (#305: an attachment folded under legibility is not lost).
///
/// And nothing under a card lies so low that Defender's wall beside it
/// stands over its face.
///
/// Every seat of rings of two to eight at three screens, one to four cards
/// under a host, tapped and untapped, staged and not. And both halves are
/// taken: an unstaged creature row shows every card's whole peek under an
/// untapped host, as a tapped host shows one in any row (and folds two, in
/// the footprint it has untapped), and a staged creature at a ring folds
/// what is under it away, so both the mark and its absence are seen.
#[test]
#[allow(clippy::too_many_lines)] // one host per row, and everything asked of it
fn nothing_tucked_under_a_card_reaches_what_stands_ahead_of_it() {
    let (mut whole, mut folded, mut checked) = (0, 0, 0);
    for seats in 2..=8u8 {
        for aspect in [4.0 / 3.0, 16.0 / 10.0, 16.0 / 9.0] {
            for under in 1..=4u32 {
                for tapped in [false, true] {
                    for staged in [false, true] {
                        let duel = table(seats, aspect, under, tapped, staged);
                        let all = placements(&duel);
                        for seat in 0..seats {
                            for lane in LaneKind::ALL {
                                let host = placed(&all, host(seat, lane));
                                let slot = host.slot;
                                let forward = slot.forward();
                                let depth = if tapped { CARD_WIDTH } else { CARD_HEIGHT };
                                let ahead = match lane {
                                    LaneKind::Creatures => slot
                                        .ledge_corners()
                                        .iter()
                                        .map(|c| c.dot(forward))
                                        .fold(f32::INFINITY, f32::min),
                                    LaneKind::Support | LaneKind::Lands => {
                                        let next = if lane == LaneKind::Support {
                                            LaneKind::Creatures
                                        } else {
                                            LaneKind::Support
                                        };
                                        slot.lane_center(next).dot(forward)
                                            - slot.lane_height() * 0.5
                                    }
                                };
                                let host_front = host.position.dot(forward) + depth * 0.5;
                                let table = format!(
                                    "{seats} seats at {aspect}, {under} under a {} \
                                     {lane:?} host{} at seat {seat}",
                                    if tapped { "tapped" } else { "untapped" },
                                    if staged { ", staged" } else { "" }
                                );
                                let mut last = host.position;
                                for k in 0..under {
                                    let card = placed(&all, hanging(seat, lane, k));
                                    let step = card.position - last;
                                    let peek = step.dot(forward);
                                    assert!(
                                        step.perp_dot(forward).abs() < 1e-4 && peek >= -1e-6,
                                        "{table}: card {k} lies {step} from the one before"
                                    );
                                    let front = card.position.dot(forward) + depth * 0.5;
                                    assert!(
                                        front <= ahead.max(host_front) + 1e-4,
                                        "{table}: card {k} reaches {front}, and what is \
                                         ahead starts at {ahead}"
                                    );
                                    assert_eq!(card.tapped, host.tapped, "{table}");
                                    // A tapped host keeps them inside the
                                    // footprint it has untapped.
                                    let untapped = host.position.dot(forward) + CARD_HEIGHT * 0.5;
                                    assert!(
                                        !tapped || front <= untapped + 1e-4,
                                        "{table}: card {k} reaches {front}, past {untapped}"
                                    );
                                    // Every host here is first in its row,
                                    // at the row's lowest.
                                    let face = CARD_LIFT + CARD_THICKNESS + card.lift;
                                    assert!(
                                        face > shellmat::WALL_HEIGHT,
                                        "{table}: card {k}'s face is at {face}, under the \
                                         top of a wall"
                                    );
                                    if (peek - ATTACH_PEEK).abs() < 1e-5 {
                                        whole += 1;
                                    } else if peek.abs() < 1e-5 {
                                        folded += 1;
                                    }
                                    checked += 1;
                                    last = card.position;
                                }
                                // Whole or not at all: no sliver.
                                let peek = (last - host.position).dot(forward);
                                #[expect(clippy::cast_precision_loss)] // four at most
                                let full = ATTACH_PEEK * under as f32;
                                let shown = (peek - full).abs() < 1e-4;
                                assert!(
                                    shown || peek.abs() < 1e-5,
                                    "{table}: what is under it peeks out {peek} of {full}"
                                );
                                // What folds lights the host's mark, with
                                // how many; what shows leaves it dark.
                                let mark = if shown {
                                    0
                                } else {
                                    cardplate::attached_word(under as usize)
                                };
                                assert_eq!(host.badge, mark, "{table}");
                                // An unstaged creature row shows every
                                // peek whole, and a tapped host one in any
                                // row, and never two.
                                if (!staged && !tapped && lane == LaneKind::Creatures)
                                    || (tapped && under == 1)
                                {
                                    assert!(shown, "{table}: what is under it folds");
                                }
                                if tapped && under > 1 {
                                    assert!(!shown, "{table}: what is under it shows");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        whole > 1_000 && folded > 500,
        "{checked} cards: {whole} peek out whole and {folded} lie folded"
    );
}
