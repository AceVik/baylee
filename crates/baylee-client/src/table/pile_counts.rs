//! The numbers on the felt beside a seat's piles: how many cards its
//! library, its graveyard, its exile and its hand hold, and how long its
//! suspended cards still wait (the owner, 12.11.2026: "wenn neben den
//! Feldern für Deck, Friedhof, Exil noch Zahlen stehen"; 13.11.: the hand's
//! too, and suspended cards shown anew).
//!
//! Each is a count badge of its own ([`BadgeMaterial::pile`]): a mark for
//! what it counts and the figures, on a small dark plate lying on the felt
//! ([`pile_count_transform`] says where). It belongs to the *place* and not
//! to a card: a library has no card to hang it on, and a hand has no pile at
//! all. It lies on no print, and a pointer over it means the table.
//!
//! **It is written for the one reading it.** A number set as a merged
//! card's count is — with its card, at the card's size and the way the card
//! lies — was five pixels tall beside a library at a table of four and lay
//! on its side at the seats to the left and right (played 12.11.2026). So
//! every one of these stands upright on the screen at every seat
//! ([`upright`]), and its figures are [`cardplate::PILE_FIGURE_PX`] tall
//! there whatever the table's size ([`cardplate::pile_badge_scale`]): the
//! seat plate's counts are read at that size, and this is the same fact said
//! where the eye already is.
//!
//! So it is not carried with its seat's zone as the mat and the piles are
//! (`sync_zones`): it is laid where its seat stands now, and during an
//! arrangement's move it is there a moment before its pile.
//!
//! A library's and a hand's are always there, nought included: a seat that
//! must draw from an empty library loses, and an empty hand is a fact the
//! table plays on. A graveyard's and exile's stand from their first card. A
//! waiting place wears one number per card, the time counters of the one
//! that comes back next first ([`WAITING_SHOWN`]).
//!
//! **At rest it writes nothing.** Each badge has a material of its own,
//! rewritten when its number changes, and a place, written again when its
//! seat, the camera or the window has moved and only if that moved it. Its
//! zone's record is borrowed mutably only when a number or the seat's place
//! changed: a camera going round writes transforms and not the index.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;
use baylee_client_core::cardplate::Tally;
use baylee_client_core::{PileKind, SeatPod};

/// One number on the felt: a marker, so they can be found and counted.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PileCount {
    /// Whose.
    pub player: PlayerId,
    /// What it counts.
    pub tally: Tally,
    /// Which of that tally's numbers it is: nought, but for a waiting place,
    /// whose cards are numbered from the one on top.
    pub nth: u8,
}

/// One number, as its seat's [`Zone`] keeps it.
pub(super) struct PileCountBadge {
    /// What it counts, and which of those numbers it is.
    what: (Tally, u8),
    /// The badge, which a zone taken down takes along.
    pub(super) badge: Entity,
    /// Its own material: see the module's note on rest.
    material: Handle<BadgeMaterial>,
    /// What it was last drawn saying.
    shown: u32,
    /// Where it was last laid.
    laid: Laid,
}

/// What a number's place is made of: its seat's slot, how far the seat's
/// zone is raised, how many numbers stand in its column, and how many cards
/// stand in each pile a number lies beside ([`stood`]).
pub(super) type Laid = (SeatSlot, f32, u8, [u32; 4]);

/// Which of [`Laid`]'s piles a tally's number lies beside.
pub(super) const fn pile_at(tally: Tally) -> usize {
    match tally {
        Tally::Library | Tally::Hand => 0,
        Tally::Graveyard => 1,
        Tally::Exile => 2,
        Tally::Waiting => 3,
    }
}

/// How many cards stand in each pile a seat's numbers lie beside: no more
/// than a pile is drawn high for, so a library of ninety drawing a card
/// moves nothing.
pub(super) fn stood(pod: &SeatPod) -> [u32; 4] {
    let mut out = [0; 4];
    for tally in [
        Tally::Library,
        Tally::Graveyard,
        Tally::Exile,
        Tally::Waiting,
    ] {
        out[pile_at(tally)] = pod
            .piles
            .iter()
            .find(|pile| pile.kind == tally.pile())
            .map_or(0, |pile| pile.count.min(SHOWN_LIBRARY + 1));
    }
    out
}

/// How high the top of a pile of `cards` stands over the felt, at a seat
/// grown `scale` times.
pub(super) fn high(cards: u32, scale: f32) -> f32 {
    if cards == 0 {
        return 0.0;
    }
    (CARD_LIFT + stack_rise(cards as usize - 1) + CARD_THICKNESS) * scale
}

/// How many of a waiting place's cards wear their number where there is a
/// column for them: the top three, which at the largest size is a column as
/// tall as the card they stand beside. The zone dialog lists every one.
pub(super) const WAITING_SHOWN: usize = 3;

/// The most numbers one seat wears.
const NUMBERS: usize = 4 + WAITING_SHOWN;

/// How far over a lying card's own height the number lies, in card widths:
/// clear of the felt and of the glow under the mat, and under every card.
const PILE_COUNT_LIFT: f32 = 0.004;

/// The air between two numbers of one column, in card widths at the plate's
/// own size.
const COLUMN_GAP: f32 = 0.03;

/// What a seat's numbers say: each tally, which of its numbers, and the
/// number. `waiting` is how many of a waiting place's cards there is room to
/// number ([`Reader::column`]).
///
/// Exile's is the zone's: the pile and the waiting place together, which is
/// what the seat plate and the zone dialog count.
pub(super) fn numbers_of(pod: &SeatPod, waiting: usize) -> [Option<(Tally, u8, u32)>; NUMBERS] {
    let pile = |kind| pod.piles.iter().find(|pile| pile.kind == kind);
    let cards = |kind| pile(kind).map_or(0, |pile| pile.count);
    let exiled = cards(PileKind::Exile) + cards(PileKind::Waiting);
    let mut out = [None; NUMBERS];
    out[0] = pile(PileKind::Library).map(|library| (Tally::Library, 0, library.count));
    out[1] = Some(cards(PileKind::Graveyard))
        .filter(|cards| *cards > 0)
        .map(|cards| (Tally::Graveyard, 0, cards));
    out[2] = (exiled > 0 && pile(PileKind::Exile).is_some()).then_some((Tally::Exile, 0, exiled));
    out[3] = pile(PileKind::Library).map(|_| (Tally::Hand, 0, pod.hand_count));
    if let Some(place) = pile(PileKind::Waiting) {
        for (nth, card) in (0u8..).zip(place.fan.iter().take(waiting.min(WAITING_SHOWN))) {
            out[4 + usize::from(nth)] =
                card.time.map(|time| (Tally::Waiting, nth, u32::from(time)));
        }
    }
    out
}

/// The turn that lays writing flat on the table with its tops up the
/// screen, for whoever looks through `eye`.
///
/// One answer for the whole table: the number at a seat across or to the
/// side is written for the viewer and not for that seat.
pub(super) fn upright(eye: &Transform) -> Quat {
    let mut up = eye.rotation * Vec3::Y;
    up.y = 0.0;
    let up = up.try_normalize().unwrap_or(Vec3::NEG_Z);
    Quat::from_mat3(&Mat3::from_cols(up.cross(Vec3::Y), up, Vec3::Y))
}

/// How squarely a seat's piles must stand to one side of its mat on the
/// screen for their numbers to lie to the right of them: the cosine between
/// that side and the screen's own left and right.
///
/// At the reader's own seat and the one across the piles stand in a column
/// up the screen, and each number lies to its pile's right as its seat has
/// right (the owner, 13.11.2026: "rechts vom Stapel", which across the table
/// is its mirror image, as that board is). For the library, the graveyard
/// and exile that is the open felt outside the pile; for the waiting place,
/// on the seat's left, it is the felt between it and the mat, so that two
/// seats side by side never write into the same strip. At a seat seen from
/// the side a pile's right is up or down the screen, where the window ends
/// and the piles run along the screen with no room between them: there
/// every number lies between its pile and the mat.
///
/// Squarely means squarely: a seat turned part of the way, as the far two
/// are at a table of five, has a neighbour's piles where its own outside
/// would be, so it is counted as seen from the side. The line is drawn clear
/// of every angle a seat is laid at, and mirror-image seats answer alike.
const BESIDE_FROM: f32 = 0.9;

/// How far from its pile's card a number may reach, in card widths at its
/// seat's size: less than the felt between a pile and its mat's rows
/// ([`baylee_client_core::layout::PILE_REACH`] less half a card), and less
/// than the cloth past the outermost pile. A number that would reach
/// further at the size asked for is drawn smaller.
const ROOM: f32 = 0.9;

/// Whether the piles of a seat at `slot` stand beside its mat on the screen,
/// for one reading `upright` ([`BESIDE_FROM`]).
pub(super) fn beside(slot: &SeatSlot, upright: Quat) -> bool {
    let side = Vec3::new(slot.facing.cos(), 0.0, slot.facing.sin());
    side.dot(upright * Vec3::X).abs() >= BESIDE_FROM
}

/// How much longer than the one badge mesh a pile's quad is: its plate has
/// a mark before its figures ([`cardplate::pile_badge_quad_rect`]).
pub(super) fn quad_stretch() -> f32 {
    let [x0, _, x1, _] = cardplate::pile_badge_quad_rect();
    (x1 - x0) / crate::badgemat::quad_size().x
}

/// Where the `nth` of a tally's `of` numbers lies, for a seat at `slot`: its
/// plate's middle, turned `upright` and grown `scale` times.
///
/// A pile's number lies beside the pile's card and level with the card's
/// own middle: to its right as its seat has right where the piles stand
/// beside the mat on the screen, between pile and mat where they do not
/// ([`beside`]). A waiting place's numbers stand there as a column, the top
/// card's uppermost. The hand's lies past the library's end on its
/// owner's side: the pile a hand is drawn from, at the edge its owner sits
/// at.
///
/// A pile is a block of cardboard, and one standing between the eye and its
/// number covers the felt just behind it: `hidden` is that patch, which way
/// over the table and how far ([`Reader::hidden`]), and a number lying that
/// way begins past it. Played 13.11.2026: a library of ninety-two covered
/// the lower third of its hand's number at the seat across.
///
/// The plate is what is placed and not the quad it is drawn in: the quad is
/// the widest number's and its shadow's, so its middle is nowhere a player
/// can see.
pub(super) fn pile_count_transform(
    slot: &SeatSlot,
    (tally, nth, of): (Tally, u8, u8),
    count: u32,
    upright: Quat,
    scale: f32,
    hidden: Vec3,
) -> Transform {
    let word = cardplate::pile_word(count, tally);
    let [x0, y0, x1, y1] = cardplate::pile_badge_rect(word);
    let kind = tally.pile();
    // The seat's own axes on the table: sideways, as `SeatSlot::pile_center`
    // has it, and towards the table's middle.
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
    let (runs, tops) = (upright * Vec3::X, upright * Vec3::Y);
    // Which way from the pile's middle, and how far its card reaches there.
    let (way, card) = if tally == Tally::Hand {
        (-away, 0.5 * CARD_HEIGHT)
    } else if beside(slot, upright) {
        // The seat's own right: outside the piles of its right-hand column,
        // and towards the mat from the waiting place on its left.
        (side, 0.5 * CARD_WIDTH)
    } else {
        (side * -kind.side(), 0.5 * CARD_WIDTH)
    };
    // How far the plate reaches along that way from its middle, turned as
    // it is, at its own size; and the size that keeps it in its room.
    let way_world = Vec3::new(way.x, 0.0, -way.y);
    let reach = (way_world.dot(runs).abs() * (x1 - x0) + way_world.dot(tops).abs() * (y1 - y0))
        * (0.5 * CARD_WIDTH);
    let behind = way_world.dot(hidden).max(0.0);
    let room = (ROOM - cardplate::PILE_BADGE_AIR) * CARD_WIDTH - behind / slot.scale;
    let scale = scale.min(room / (2.0 * reach));
    let grown = scale * slot.scale;
    let half = Vec2::new(x1 - x0, y1 - y0) * (0.5 * CARD_WIDTH * grown);
    let reach = reach * grown;
    let off = (card + cardplate::PILE_BADGE_AIR * CARD_WIDTH) * slot.scale + behind + reach;
    let mut at = card_transform(slot, slot.pile_center(kind) + way * off, false, 0.0).translation;
    at.y += PILE_COUNT_LIFT * slot.scale;
    // A column's numbers step down the screen from its top one.
    let step = 2.0 * half.y + COLUMN_GAP * CARD_WIDTH * grown;
    at += tops * (step * (0.5 * (f32::from(of) - 1.0) - f32::from(nth)));
    // From the plate's middle to the quad's, in the badge's own space.
    let body = on_card(0.0, [x0, y0, x1, y1]).translation;
    let quad = on_card(0.0, cardplate::pile_badge_quad_rect()).translation;
    let within = (quad - body).truncate().extend(0.0);
    Transform {
        translation: at + upright * (within * grown),
        rotation: upright,
        // The one mesh, longer for the mark.
        scale: Vec3::new(grown * quad_stretch(), grown, grown),
    }
}

/// How long one card width lying up the screen is drawn beside a pile, in
/// the interface's pixels.
fn card_px(lens: &Lens, slot: &SeatSlot, kind: PileKind, upright: Quat) -> Option<f32> {
    let at = card_transform(slot, slot.pile_center(kind), false, 0.0).translation;
    let up = upright * Vec3::Y * (CARD_WIDTH * slot.scale);
    Some(
        lens.project_world(at)?
            .distance(lens.project_world(at + up)?),
    )
}

/// The eye a frame's numbers are laid for: which way is up the screen, and
/// the projection that says how large a card is drawn.
pub(super) struct Reader {
    /// The turn that stands writing upright on the screen ([`upright`]).
    pub(super) upright: Quat,
    /// Where the eye is.
    eye: Vec3,
    /// The projection, where there is a window.
    lens: Option<Lens>,
}

impl Reader {
    /// The one looking through `rig` at a `window`, where there is one.
    pub(super) fn of(rig: CameraRig, window: Option<Vec2>) -> Self {
        let eye = rig.eye();
        Self {
            upright: upright(&eye),
            eye: eye.translation,
            lens: window.map(|window| Lens::new(rig, window)),
        }
    }

    /// What a pile standing `high` at `at` covers of the felt behind it for
    /// this eye: which way over the table, and how far.
    pub(super) fn hidden(&self, at: Vec3, high: f32) -> Vec3 {
        let above = self.eye.y - TABLE_Y - high;
        if high <= 0.0 || above <= 0.0 {
            return Vec3::ZERO;
        }
        (at - self.eye).with_y(0.0) * (high / above)
    }

    /// How many of a waiting place's cards a seat at `slot` has room to
    /// number: a column where its piles stand beside its mat on the screen,
    /// the top card's alone at a seat seen from the side.
    pub(super) fn column(&self, slot: &SeatSlot) -> usize {
        if beside(slot, self.upright) {
            WAITING_SHOWN
        } else {
            1
        }
    }

    /// Where the `nth` number of `tally` saying `count` lies, for a seat
    /// laid as `laid`.
    pub(super) fn lay(&self, laid: &Laid, (tally, nth): (Tally, u8), count: u32) -> Transform {
        let (slot, raised, column, stood) = laid;
        let pile = card_transform(slot, slot.pile_center(tally.pile()), false, 0.0).translation;
        let hidden = self.hidden(pile, high(stood[pile_at(tally)], slot.scale));
        let px = self
            .lens
            .as_ref()
            .and_then(|lens| card_px(lens, slot, tally.pile(), self.upright));
        let scale = cardplate::pile_badge_scale(cardplate::pile_word(count, tally), px);
        let of = if tally == Tally::Waiting { *column } else { 1 };
        let mut place =
            pile_count_transform(slot, (tally, nth, of), count, self.upright, scale, hidden);
        place.translation.y += raised;
        place
    }
}

/// How a seat's `wanted` numbers are laid: at its slot, on its lift, its
/// waiting times in a column as long as there are of them.
fn laid_of(zone: &Zone, pod: &SeatPod, wanted: &[Option<(Tally, u8, u32)>]) -> Laid {
    let column = wanted
        .iter()
        .flatten()
        .filter(|(tally, ..)| *tally == Tally::Waiting)
        .count();
    (
        zone.slot,
        zone.raised,
        u8::try_from(column).unwrap_or(u8::MAX),
        stood(pod),
    )
}

/// Whether a seat already wears exactly the `wanted` numbers, laid as `laid`.
fn kept(zone: &Zone, wanted: &[Option<(Tally, u8, u32)>], laid: &Laid) -> bool {
    wanted.iter().flatten().count() == zone.counts.len()
        && wanted.iter().flatten().all(|&(tally, nth, count)| {
            zone.counts.iter().any(|drawn| {
                drawn.what == (tally, nth) && drawn.shown == count && drawn.laid == *laid
            })
        })
}

/// Keeps every drawn seat's numbers on the felt.
///
/// After [`sync_zones`], which says which seats have a zone and where; a
/// zone that is taken down takes its numbers with it there.
#[allow(clippy::too_many_arguments)] // Bevy system: the table, the eye and the window it is read through
pub fn sync_pile_counts(
    mut commands: Commands,
    duel: Res<Duel>,
    shown: Res<ShownRig>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    mut index: ResMut<SceneIndex>,
    mut materials: ResMut<Assets<BadgeMaterial>>,
    mut places: Query<&mut Transform, With<PileCount>>,
    mut seen: Local<Option<(CameraRig, Option<Vec2>)>>,
) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let Some(quad) = index.badge_quad.clone() else {
        return;
    };
    let rig = shown.rig().unwrap_or_default();
    let space = crate::hud::scale::window_space(&windows, ui.as_deref());
    // The eye or the window moved: every number is laid again.
    let looked = *seen != Some((rig, space));
    if looked {
        *seen = Some((rig, space));
    }
    let reader = Reader::of(rig, space);
    for pod in &board.pods {
        let Some(zone) = index.zones.get(&pod.player) else {
            continue;
        };
        // A fanned waiting place lays its cards where its numbers stand,
        // and they step aside for as long.
        let fanned = index
            .fanned
            .values()
            .any(|(_, player, kind)| *player == pod.player && *kind == PileKind::Waiting);
        let column = if fanned { 0 } else { reader.column(&zone.slot) };
        let wanted = numbers_of(pod, column);
        let laid = laid_of(zone, pod, &wanted);
        if kept(zone, &wanted, &laid) {
            // Nothing of the record changes. Only the eye may have, and that
            // moves the badges without a write to the index.
            if looked {
                for count in &zone.counts {
                    let place = reader.lay(&laid, count.what, count.shown);
                    if let Ok(mut lies) = places.get_mut(count.badge) {
                        lies.set_if_neq(place);
                    }
                }
            }
            continue;
        }
        let Some(zone) = index.zones.get_mut(&pod.player) else {
            continue;
        };
        // What is no longer said goes.
        zone.counts.retain(|drawn| {
            let stays = wanted
                .iter()
                .flatten()
                .any(|&(tally, nth, _)| drawn.what == (tally, nth));
            if !stays {
                commands.entity(drawn.badge).despawn();
            }
            stays
        });
        for &(tally, nth, number) in wanted.iter().flatten() {
            let place = reader.lay(&laid, (tally, nth), number);
            if let Some(count) = zone
                .counts
                .iter_mut()
                .find(|drawn| drawn.what == (tally, nth))
            {
                if count.shown != number
                    && let Some(mut material) = materials.get_mut(&count.material)
                {
                    *material = BadgeMaterial::pile(number, tally);
                }
                if let Ok(mut lies) = places.get_mut(count.badge) {
                    lies.set_if_neq(place);
                }
                count.shown = number;
                count.laid = laid;
            } else {
                let material = materials.add(BadgeMaterial::pile(number, tally));
                let badge = commands
                    .spawn((
                        DuelStage,
                        PileCount {
                            player: pod.player,
                            tally,
                            nth,
                        },
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(material.clone()),
                        place,
                        Pickable::IGNORE,
                    ))
                    .id();
                zone.counts.push(PileCountBadge {
                    what: (tally, nth),
                    badge,
                    material,
                    shown: number,
                    laid,
                });
            }
        }
    }
}
