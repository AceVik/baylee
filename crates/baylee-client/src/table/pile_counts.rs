//! The number beside a pile: how many cards a library, a graveyard or an
//! exile pile holds (the owner, 12.11.2026: "wenn neben den Feldern für
//! Deck, Friedhof, Exil noch Zahlen stehen").
//!
//! Each is a count badge of its own ([`BadgeMaterial::pile`]) lying on the
//! felt beside its pile, on the side of it its seat's mat is on. It belongs
//! to the *place* and not to the pile's top card. A library has no card to
//! hang it on, and a fan lifting a graveyard's cards opens over the number
//! and leaves it where it was. It lies on no print, and a pointer over it
//! means the table.
//!
//! **It is written for the one reading it.** A number set as a merged
//! card's count is — with its card, at the card's size and the way the card
//! lies — was five pixels tall beside a library at a table of four and lay
//! on its side at the seats to the left and right (played 12.11.2026). So a
//! pile's number stands upright on the screen at every seat ([`upright`]),
//! and its figures are [`cardplate::PILE_FIGURE_PX`] tall there whatever
//! the table's size ([`cardplate::pile_badge_scale`]): the seat plate's
//! counts are read at that size, and this is the same fact said where the
//! eye already is.
//!
//! So it is not carried with its seat's zone as the mat and the piles are
//! (`sync_zones`): it is laid where its seat stands now, and during an
//! arrangement's move it is there a moment before its pile.
//!
//! A library's is always there, nought included, since a seat that must draw
//! from an empty library loses; a graveyard's and an exile pile's stand from
//! their first card.
//!
//! **At rest it writes nothing.** Each badge has a material of its own,
//! rewritten when its count changes, and a place, written again when its
//! seat, the camera or the window has moved and only if that moved it. Its
//! zone's record is borrowed mutably only when a count or the seat's place
//! changed: a camera going round writes transforms and not the index.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;
use baylee_client_core::{PileKind, SeatPod};

/// The number beside one pile: a marker, so they can be found and counted.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PileCount {
    /// Whose pile.
    pub player: PlayerId,
    /// Which of their piles.
    pub kind: PileKind,
}

/// One pile's number, as its seat's [`Zone`] keeps it.
pub(super) struct PileCountBadge {
    /// Which pile it counts.
    kind: PileKind,
    /// The badge, which a zone taken down takes along.
    pub(super) badge: Entity,
    /// Its own material: see the module's note on rest.
    material: Handle<BadgeMaterial>,
    /// How many it was last drawn for.
    shown: u32,
    /// The seat's place it was last laid beside.
    laid: (SeatSlot, f32),
}

/// The piles that wear their number.
const COUNTED: [PileKind; 3] = [PileKind::Library, PileKind::Graveyard, PileKind::Exile];

/// How far over a lying card's own height the number lies, in card widths:
/// clear of the felt and of the glow under the mat, and under every card.
const PILE_COUNT_LIFT: f32 = 0.004;

/// What a pile's number says, or `None` where it wears none: a library's
/// always, the others' from their first card.
fn count_of(pod: &SeatPod, kind: PileKind) -> Option<u32> {
    let cards = pod.piles.iter().find(|pile| pile.kind == kind)?.count;
    (kind == PileKind::Library || cards > 0).then_some(cards)
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

/// Where a pile's number lies, for a seat at `slot`: its body's middle
/// beside the pile's card, on the mat's side of it, level with the card's
/// own middle, turned `upright` and grown `scale` times.
///
/// The body is what is placed and not the quad it is drawn in: the quad is
/// the widest number's and its shadow's, so its middle is nowhere a player
/// can see.
pub(super) fn pile_count_transform(
    slot: &SeatSlot,
    kind: PileKind,
    count: u32,
    upright: Quat,
    scale: f32,
) -> Transform {
    let word = cardplate::pile_word(count);
    let [x0, y0, x1, y1] = cardplate::pile_badge_rect(word);
    let grown = scale * slot.scale;
    // The body's half size on the table, and the way to the mat: the seat's
    // own sideways axis (`SeatSlot::fan_pose` has the same two lines).
    let half = Vec2::new(x1 - x0, y1 - y0) * (0.5 * CARD_WIDTH * grown);
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let to_mat = side * -kind.side();
    let to_mat_world = Vec3::new(to_mat.x, 0.0, -to_mat.y);
    // How far the body reaches along that way, turned as it is.
    let reach = (to_mat_world.dot(upright * Vec3::X)).abs() * half.x
        + (to_mat_world.dot(upright * Vec3::Y)).abs() * half.y;
    let off = (0.5 + cardplate::PILE_BADGE_AIR) * CARD_WIDTH * slot.scale + reach;
    let mut at = card_transform(slot, slot.pile_center(kind) + to_mat * off, false, 0.0).translation;
    at.y += PILE_COUNT_LIFT * slot.scale;
    // From the body's middle to the quad's, in the badge's own space.
    let body = on_card(0.0, [x0, y0, x1, y1]).translation;
    let quad = on_card(0.0, cardplate::pile_badge_quad_rect()).translation;
    let within = (quad - body).truncate().extend(0.0);
    Transform::from_translation(at)
        * Transform::from_rotation(upright)
        * Transform::from_scale(Vec3::splat(grown))
        * Transform::from_translation(within)
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
    /// The projection, where there is a window.
    pub(super) lens: Option<Lens>,
}

impl Reader {
    /// Where the number of `cards` lies beside this pile of a seat at `laid`.
    pub(super) fn lay(&self, laid: &(SeatSlot, f32), kind: PileKind, cards: u32) -> Transform {
        let (slot, raised) = laid;
        let px = self
            .lens
            .as_ref()
            .and_then(|lens| card_px(lens, slot, kind, self.upright));
        let scale = cardplate::pile_badge_scale(cardplate::pile_word(cards), px);
        let mut place = pile_count_transform(slot, kind, cards, self.upright, scale);
        place.translation.y += raised;
        place
    }
}

/// Keeps the number beside every drawn seat's library, graveyard and exile
/// pile.
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
    let reader = Reader {
        upright: upright(&rig.eye()),
        lens: space.map(|space| Lens::new(rig, space)),
    };
    for pod in &board.pods {
        let Some(zone) = index.zones.get(&pod.player) else {
            continue;
        };
        let laid = (zone.slot, zone.raised);
        let wanted = COUNTED.map(|kind| count_of(pod, kind));
        let kept = COUNTED.into_iter().zip(wanted).all(|(kind, wanted)| {
            let drawn = zone.counts.iter().find(|count| count.kind == kind);
            wanted == drawn.map(|count| count.shown) && drawn.is_none_or(|count| count.laid == laid)
        });
        if kept {
            // Nothing of the record changes. Only the eye may have, and that
            // moves the badges without a write to the index.
            if looked {
                for count in &zone.counts {
                    let place = reader.lay(&laid, count.kind, count.shown);
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
        for (kind, wanted) in COUNTED.into_iter().zip(wanted) {
            let at = zone.counts.iter().position(|count| count.kind == kind);
            let Some(cards) = wanted else {
                if let Some(at) = at {
                    commands.entity(zone.counts.swap_remove(at).badge).despawn();
                }
                continue;
            };
            let place = reader.lay(&laid, kind, cards);
            if let Some(at) = at {
                let count = &mut zone.counts[at];
                if count.shown != cards
                    && let Some(mut material) = materials.get_mut(&count.material)
                {
                    *material = BadgeMaterial::pile(cards);
                }
                if let Ok(mut lies) = places.get_mut(count.badge) {
                    lies.set_if_neq(place);
                }
                count.shown = cards;
                count.laid = laid;
            } else {
                let material = materials.add(BadgeMaterial::pile(cards));
                let badge = commands
                    .spawn((
                        DuelStage,
                        PileCount {
                            player: pod.player,
                            kind,
                        },
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(material.clone()),
                        place,
                        Pickable::IGNORE,
                    ))
                    .id();
                zone.counts.push(PileCountBadge {
                    kind,
                    badge,
                    material,
                    shown: cards,
                    laid,
                });
            }
        }
    }
}
