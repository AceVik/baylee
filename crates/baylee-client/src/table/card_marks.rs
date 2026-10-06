//! What lies on and under a card: keyword strip, count badge, plate, floor light.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// The keyword strip lying on a card (#274): a marker, so a strip can be
/// found and counted without being taken for the card or its shadow.
#[derive(Component)]
pub struct KeywordStrip;

/// Where a card's keyword strip lies, in the card's own space: at
/// [`cardrail::quad_rect`], a share of its row's step over the face.
pub(super) fn strip_transform(rung: f32) -> Transform {
    let [x0, y0, x1, y1] = cardrail::quad_rect();
    Transform::from_xyz(
        (f32::midpoint(x0, x1) - 0.5) * CARD_WIDTH,
        CARD_HEIGHT * 0.5 - f32::midpoint(y0, y1) * DOWN_THE_CARD,
        CARD_THICKNESS + rung * STRIP_STEP_SHARE,
    )
}

/// Puts the strip on a card, changes it, or takes it off (#274, #298).
///
/// A diff like the rest of [`sync_scene`]: a card whose strip and row step
/// have not moved costs one lookup. The strip is a child of the card, so it
/// follows every glide, tap, lift and exit with nothing to keep in step, and
/// goes when the card does. It is not a [`CardShadow`] —
/// [`ground_the_shadows`] must not flatten it onto the felt — and it is not
/// pickable: a click on a mark is a click on the card.
pub(super) fn sync_strip(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<MarksMaterial>,
    card: Entity,
    (object, strip, rung): (ObjectId, cardrail::Strip, f32),
    motion: f32,
) {
    let current = index.marks.get(&object).copied();
    if current.is_some_and(|(said, at, _)| said == strip && at.to_bits() == rung.to_bits()) {
        return;
    }
    if strip.is_empty() {
        if let Some((.., entity)) = index.marks.remove(&object) {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Some(quad) = index.marks_quad.clone() else {
        return;
    };
    let material = index
        .marks_materials
        .entry(strip)
        .or_insert_with(|| materials.add(MarksMaterial::new(strip, motion)))
        .clone();
    let transform = strip_transform(rung);
    let entity = if let Some((.., entity)) = current {
        commands
            .entity(entity)
            .try_insert((MeshMaterial3d(material), transform));
        entity
    } else {
        let entity = commands
            .spawn((
                KeywordStrip,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(entity);
        entity
    };
    index.marks.insert(object, (strip, rung, entity));
}

/// The count badge at a merged card's corner (#261): a marker, so a badge
/// can be found and counted without being taken for the card or its strip.
#[derive(Component)]
pub struct CountBadge;

/// What a card's badge was put on for: the count, the row step, where it
/// stands, whether its card is tapped and whether it reads turned round.
pub(super) type BadgeKey = (u32, f32, BadgePlace, bool, bool);

/// A badge standing over its card ([`BadgePlace::Above`]) stays upright when
/// the card taps (the owner, 25.09): it is where it would be on the card
/// untapped, clear of the row, and tapping moves nothing else in a row
/// either. So does every plate ([`sync_plate`]), which under a tapped card
/// stands where `cardplate::plate_rect` puts it in the untapped card's
/// frame. Each is still the card's child, so it glides and goes with it;
/// [`keep_upright`] turns it back by as much as the card has turned.
#[derive(Component, Clone, Copy, Debug)]
pub struct Upright {
    /// The card's rotation untapped.
    base: Quat,
    /// Its transform on the card untapped.
    at: Transform,
}

impl Upright {
    /// The badge's transform on a card turned `rotation`: its place on the
    /// card untapped, turned back by as much as the card has turned about its
    /// own face's normal from untapped, which is a tap.
    ///
    /// Only that turn: a flier banks and pitches (`sync_scene`'s sway), and
    /// what lies on it tilts with it. Turned back from all of it, a plate
    /// lay flat while its card rocked round it, and half of each rock the
    /// print rose through the plate's end that lies on the card (the PM,
    /// 25.09: a Darksteel Gargoyle's 4/4 read "/4").
    fn on(&self, rotation: Quat) -> Transform {
        let back = tap_of(self.base.inverse() * rotation).inverse();
        Transform {
            translation: back * self.at.translation,
            rotation: back * self.at.rotation,
            scale: self.at.scale,
        }
    }
}

/// The part of `turn`, a card's rotation in its own frame, that is about its
/// face's normal, its `z`: the twist of a swing-twist split, which is the
/// tap. What is left is the swing, a tilt off the face's plane.
fn tap_of(turn: Quat) -> Quat {
    let twist = Quat::from_xyzw(0.0, 0.0, turn.z, turn.w);
    if twist.length_squared() < 1e-12 {
        Quat::IDENTITY
    } else {
        twist.normalize()
    }
}

/// Where a card's count badge lies, in the card's own space, untapped: at
/// [`cardplate::badge_quad_rect`] for `place`, at the strip's share of its
/// row's step over the face.
///
/// The strip's height and for the strip's reason: a badge lifted further
/// would stand over the card laid on this one.
pub(super) fn badge_transform(rung: f32, place: BadgePlace) -> Transform {
    on_card(rung, cardplate::badge_quad_rect(place))
}

/// Where a quad lies on its card, in the card's own space, untapped: at
/// `quad`, `[x0, y0, x1, y1]` in card widths from the card's top-left
/// corner, at the strip's share of its row's step over the face.
pub(super) fn on_card(rung: f32, quad: [f32; 4]) -> Transform {
    let [x0, y0, x1, y1] = quad;
    Transform::from_xyz(
        (f32::midpoint(x0, x1) - 0.5) * CARD_WIDTH,
        CARD_HEIGHT * 0.5 - f32::midpoint(y0, y1) * DOWN_THE_CARD,
        CARD_THICKNESS + rung * STRIP_STEP_SHARE,
    )
}

/// Whether writing lying on the table turned `rotation` reads upside down
/// through `eye` (the PO, 25.09): its tops, the quad's `+y`, point down the
/// screen. A seat's cards face their owner, so across the table an
/// opponent's do from the local seat, and the local seat's do once the
/// camera has gone round to frame that opponent. The plate and the badge
/// then draw their numbers a half turn round ([`cardplate::PLATE_TURNED`]);
/// the strip stays as its card lies. Writing lying sideways, as a side
/// seat's at a ring, is neither, and stays as it lies.
fn reads_upside_down(rotation: Quat, eye: &Transform) -> bool {
    (rotation * Vec3::Y).dot(eye.rotation * Vec3::Y) < -SIDEWAYS
}

/// How far past square a thing's tops must point from the screen's before
/// it reads upside down: a side seat's writing, square to the eye, stays as
/// it lies rather than turning on a rounding error.
const SIDEWAYS: f32 = 1e-3;

/// Keeps every badge standing over its card and every plate upright while
/// its card turns (#298): a tap glides, and each is the card's child, so it
/// is laid again from where the glide has the card this frame.
pub fn keep_upright(
    cards: Query<&Transform, Without<Upright>>,
    mut uprights: Query<(&ChildOf, &Upright, &mut Transform)>,
) {
    for (parent, upright, mut local) in &mut uprights {
        if let Ok(card) = cards.get(parent.parent()) {
            local.set_if_neq(upright.on(card.rotation));
        }
    }
}

/// Puts the count badge on a card, changes it, or takes it off (#261).
///
/// [`sync_strip`]'s diff, for [`sync_strip`]'s reasons: a child of the card,
/// so it follows every glide and goes with the card; not a [`CardShadow`];
/// not pickable, since a click on the count is a click on the card. Where it
/// stands is its seat's rows' ([`SeatSlot::badge_place`]): over the card it
/// is [`Upright`], beside it it turns with a tapped card. Where `eye` sees
/// it upside down, its count is drawn turned round ([`reads_upside_down`]).
pub(super) fn sync_badge(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<BadgeMaterial>,
    card: Entity,
    placement: &Placement,
    eye: &Transform,
) {
    let place = placement.slot.badge_place();
    let pose = |tapped| card_transform(&placement.slot, placement.position, tapped, placement.lift);
    let base = pose(false).rotation;
    // Where it comes to rest: an upright badge as its card lies untapped,
    // one beside the card as the card lies.
    let rest = if place == BadgePlace::Above {
        base
    } else {
        pose(placement.tapped).rotation
    };
    let turned = reads_upside_down(rest, eye);
    let key: BadgeKey = (
        placement.badge,
        placement.rung,
        place,
        placement.tapped,
        turned,
    );
    let current = index.badges.get(&placement.object).copied();
    if current.is_some_and(|((count, rung, at, tapped, turn), _)| {
        (count, at, tapped, turn) == (key.0, key.2, key.3, key.4)
            && rung.to_bits() == key.1.to_bits()
    }) {
        return;
    }
    if placement.badge == 0 {
        if let Some((_, badge)) = index.badges.remove(&placement.object) {
            commands.entity(badge).despawn();
        }
        return;
    }
    let Some(quad) = index.badge_quad.clone() else {
        return;
    };
    let material = index
        .badge_materials
        .entry((placement.badge, place, turned))
        .or_insert_with(|| materials.add(BadgeMaterial::new(placement.badge, place, turned)))
        .clone();
    let at = badge_transform(placement.rung, place);
    let upright = (place == BadgePlace::Above).then_some(Upright { base, at });
    // Laid for where the card will come to rest; `keep_upright` keeps an
    // upright one so on the way there.
    let transform = upright.map_or(at, |upright| upright.on(pose(placement.tapped).rotation));
    let badge = if let Some((_, badge)) = current {
        commands
            .entity(badge)
            .try_insert((MeshMaterial3d(material), transform));
        badge
    } else {
        let badge = commands
            .spawn((
                CountBadge,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(badge);
        badge
    };
    match upright {
        Some(upright) => {
            commands.entity(badge).try_insert(upright);
        }
        None => {
            commands.entity(badge).try_remove::<Upright>();
        }
    }
    index.badges.insert(placement.object, (key, badge));
}

/// The plate at a card's bottom right (the owner, 25.09): a marker, so a
/// plate can be found and counted without being taken for the card, its
/// strip or its badge.
#[derive(Component)]
pub struct CardPlate;

/// What a card's plate was put on for: what it says, where on the card it
/// stands (its body, [`cardplate::plate_rect`], as bits), the row step and
/// whether its card is tapped.
pub(super) type PlateKey = (PlateWords, [u32; 4], u32, bool);

/// Puts the plate on a card, moves it, or takes it off (the owner, 25.09).
///
/// [`sync_badge`]'s diff, for [`sync_strip`]'s reasons: a child of the card,
/// so it follows every glide and goes with the card; not a [`CardShadow`];
/// not pickable, since a click on the numbers is a click on the card.
/// `words` is `None` where the print says the numbers itself. Where it
/// stands is what the row leaves it ([`Placement::room`]): beside the
/// printed box, on the card's own foot, or under a tapped card — and
/// nowhere, and it goes, where a tapped card's neighbours leave it no air.
/// It is always [`Upright`]: on an untapped card that is where it lies
/// anyway. Where `eye` sees it upside down, its numbers are drawn turned
/// round ([`reads_upside_down`]).
pub(super) fn sync_plate(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<PlateMaterial>,
    card: Entity,
    placement: &Placement,
    words: Option<PlateWords>,
    eye: &Transform,
) {
    let place = placement.slot.badge_place();
    // A badge beside the card turns with it, and under a tapped card it
    // hangs where the plate would.
    let badge = (placement.badge > 0 && placement.tapped && place == BadgePlace::Beside)
        .then(|| cardplate::badge_quad_rect(place));
    let base = card_transform(&placement.slot, placement.position, false, placement.lift).rotation;
    let body = words.and_then(|words| {
        let words = words.turned(reads_upside_down(base, eye));
        let kind = words.word >> cardplate::KIND_SHIFT;
        cardplate::plate_rect(kind, placement.tapped, placement.room, badge)
            .map(|(body, _)| (words, body))
    });
    let current = index.plates.get(&placement.object).copied();
    let Some((words, body)) = body else {
        if let Some((_, plate)) = index.plates.remove(&placement.object) {
            commands.entity(plate).despawn();
        }
        return;
    };
    let key: PlateKey = (
        words,
        body.map(f32::to_bits),
        placement.rung.to_bits(),
        placement.tapped,
    );
    if current.is_some_and(|(said, _)| said == key) {
        return;
    }
    let Some(quad) = index.plate_quad.clone() else {
        return;
    };
    let material = index
        .plate_materials
        .entry(words)
        .or_insert_with(|| materials.add(PlateMaterial::new(words)))
        .clone();
    let upright = Upright {
        base,
        at: on_card(placement.rung, cardplate::plate_quad(body)),
    };
    // Laid for where the card will come to rest, as an upright badge is.
    let transform = upright.on(card_transform(
        &placement.slot,
        placement.position,
        placement.tapped,
        placement.lift,
    )
    .rotation);
    let plate = if let Some((_, plate)) = current {
        commands
            .entity(plate)
            .try_insert((MeshMaterial3d(material), transform, upright));
        plate
    } else {
        let plate = commands
            .spawn((
                CardPlate,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                upright,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(plate);
        plate
    };
    index.plates.insert(placement.object, (key, plate));
}

/// The offer's light on the felt under a card (#298): a marker, so a light
/// can be found and counted without being taken for the card, its shadow or
/// its strip.
#[derive(Component)]
pub struct FloorLight;

/// Where a card's light lies, in the card's own space: `depth` under the
/// card's own lift, so on the felt at [`FLOOR_RUNG`] however high the card's
/// row and deck have put it.
fn floor_transform(depth: f32) -> Transform {
    Transform::from_xyz(0.0, 0.0, -(CARD_LIFT - FLOOR_RUNG + depth))
}

/// Puts the offer's light under a card, changes it, or takes it away (#298).
///
/// [`sync_strip`]'s diff, for [`sync_strip`]'s reasons: a child of the card,
/// so it follows every glide and tap and goes with the card; not a
/// [`CardShadow`]; not pickable, since a click on the light round a card
/// means the table. It lies at the felt under the row's rise and the deck,
/// and rides a card lifted by a hover or by flying, whose light it still is.
/// A card standing in a pile's hover fan is tipped up in the air, so its
/// light lies just under it instead, as a halo round the card.
pub(super) fn sync_floor(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<FloorMaterial>,
    card: Entity,
    placement: &Placement,
    glow: u32,
    motion: f32,
) {
    let offers = glow & crate::cardmat::glow::OFFERS;
    let depth = if placement.fan.is_some() {
        0.0
    } else {
        placement.lift + stack_rise(placement.count.saturating_sub(1))
    };
    let current = index.floors.get(&placement.object).copied();
    if current.is_some_and(|(said, at, _)| said == offers && at.to_bits() == depth.to_bits()) {
        return;
    }
    if offers == 0 {
        if let Some((.., light)) = index.floors.remove(&placement.object) {
            commands.entity(light).despawn();
        }
        return;
    }
    let Some(quad) = index.floor_quad.clone() else {
        return;
    };
    let material = index
        .floor_materials
        .entry(offers)
        .or_insert_with(|| materials.add(FloorMaterial::new(offers, motion)))
        .clone();
    let transform = floor_transform(depth);
    let light = if let Some((.., light)) = current {
        commands
            .entity(light)
            .try_insert((MeshMaterial3d(material), transform));
        light
    } else {
        let light = commands
            .spawn((
                FloorLight,
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(light);
        light
    };
    index
        .floors
        .insert(placement.object, (offers, depth, light));
}
