//! Cards in motion: glides, contact shadows, entrances and exits.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// How quickly a card settles onto its mark, as a fraction of the remaining
/// distance per second.
///
/// Exponential rather than a fixed duration, because the thing being animated
/// is a *correction*: a card whose lane repacked by half a millimetre and a
/// card that just entered the battlefield are the same code path, and the
/// first must not take as long as the second. At 16 the long move reads as a
/// deal and the short one as a settle, which is what a hand on a real table
/// looks like.
const SETTLE: f32 = 16.0;

/// Below this, a card is simply put on its mark: the last hundredth of a
/// millimetre of an exponential curve is not worth a frame of work, and
/// leaving it unfinished is what makes a "still" board quietly never idle.
const SETTLED: f32 = 0.0008;

/// How far above the table a card appears before dropping onto it.
///
/// Direction-agnostic on purpose. A card could fly in from its owner's hand,
/// and at four seats around a ring that means four different directions and a
/// card that flies *across* two other players' boards to get home. Dropping
/// in reads as "this arrived" from every chair.
const ENTRANCE_RISE: f32 = 1.4;

/// How small a card is when it appears, before it settles to full size.
const ENTRANCE_SCALE: f32 = 0.86;

/// How quickly the camera settles, in the same units as [`SETTLE`].
///
/// Faster than the cards: a drag that lags behind the pointer feels broken,
/// while a card that snaps feels cheap. Same mechanism, different answer.
pub(super) const CAMERA_SETTLE: f32 = 24.0;

/// A drawn card, and the group it stands for.
#[derive(Component)]
pub struct CardVisual {
    /// The object the card represents and that input reports.
    pub object: ObjectId,
    /// How many permanents it stands for.
    pub count: usize,
}

/// Where a card stands when nothing is touching it.
///
/// [`sync_scene`] builds a card's pose in two stages — its place on the felt,
/// and then what the pointer or an armed deed is doing to it ([`HOVER_LIFT`]
/// with [`HOVER_SCALE`], [`SELECTED_LIFT`] with [`SELECTED_SCALE`]) — and this
/// is the first stage kept on its own.
///
/// It exists because something finally had to **point** at a card rather than
/// be one. The ability sheet stands beside a permanent for as long as a player
/// is reading it, and anchored to the live pose it was dragged about by the
/// [`HOVER_LIFT`] and [`HOVER_SCALE`] the pointer applies to a card: a sheet
/// that jumped whenever the hand moved across the thing it was describing.
///
/// Nothing *draws* from it, which is what keeps it from being a second opinion
/// about where a card is — [`Motion::target`] is still the only one.
#[derive(Component)]
pub struct CardRest(pub Transform);

/// Everything [`sync_scene`] writes on a card that is already on the table.
///
/// Named because it is five terms long and appears in three places in that
/// one function, not because it is a concept: a card on the felt is its pose,
/// what it stands for, what it is made of, whether it is in the air, and
/// where it would be with nothing touching it.
pub(super) type DrawnCard = (
    &'static mut Motion,
    &'static mut CardVisual,
    &'static mut MeshMaterial3d<CardMaterial>,
    Has<Floating>,
    &'static mut CardRest,
    &'static mut Visibility,
);

/// A card's visibility: its parent's while it is shown, none while a
/// scrolled row leaves it out.
pub(super) fn shown_as(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// A card that is off the felt because what it stands for has flying.
///
/// A marker and nothing more: the height is written into [`Motion::target`]
/// by [`sync_scene`] like every other reason a card is where it is, so there
/// is no second number here that could disagree with it. What the marker is
/// for is the *shadow* — [`ground_the_shadows`] asks which cards' shadows
/// have to be left behind on the table, and this is the answer.
#[derive(Component)]
pub struct Floating;

/// The contact shadow under a card, as a child of that card.
///
/// It was unmarked while it was only ever set once at spawn. It has to be
/// found again now, because a card that leaves the ground has to leave it
/// behind.
#[derive(Component)]
pub struct CardShadow;

/// Where a card is going.
///
/// The scene diff writes the *target* and never the transform itself, so
/// every source of movement — a lane repacking, a tap, a hover, a card
/// entering play — arrives through one door and animates for free. It also
/// means the animation cannot desynchronise from the board model: there is
/// nothing to keep in step, because the target is recomputed from the model
/// every frame.
#[derive(Component, Clone, Copy)]
pub struct Motion {
    /// The transform the card belongs at right now.
    pub target: Transform,
}

/// Moves every card towards its mark.
///
/// Frame-rate independent: the fraction covered is `1 - e^(-rate · dt)`, so
/// the same motion plays out identically at 30 and at 144 frames per second.
/// A naive `lerp(0.2)` per frame does not — it makes the whole table twice as
/// fast on a better machine, which is the bug this shape exists to avoid.
pub fn glide(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut cards: Query<(&Motion, &mut Transform, Option<&crate::combatfx::Recoil>)>,
) {
    let still = prefs.all().reduce_motion;
    let t = 1.0 - (-SETTLE * time.delta_secs()).exp();
    for (motion, mut transform, recoil) in &mut cards {
        let mut target = motion.target;
        if !still && let Some(recoil) = recoil {
            target.translation += recoil.offset(time.elapsed_secs());
        }
        let there = transform.translation.distance_squared(target.translation) < SETTLED * SETTLED
            && transform.rotation.angle_between(target.rotation) < SETTLED
            && transform.scale.distance_squared(target.scale) < SETTLED * SETTLED;
        if still || there {
            if *transform != target {
                *transform = target;
            }
            continue;
        }
        transform.translation = transform.translation.lerp(target.translation, t);
        transform.rotation = transform.rotation.slerp(target.rotation, t);
        transform.scale = transform.scale.lerp(target.scale, t);
    }
}

/// Only live cards own grounded shadows; departing cards keep their exit pose.
/// The exclusion makes the card and shadow transform queries disjoint.
type ShadowOwners = (With<CardVisual>, Without<CardShadow>);

/// The materials of the objects lying on and round a card: its strip, its
/// count badge, its plate, the offer's light on the felt round it and its
/// shell.
pub(super) type CardCompanions<'w> = (
    ResMut<'w, Assets<MarksMaterial>>,
    ResMut<'w, Assets<BadgeMaterial>>,
    ResMut<'w, Assets<PlateMaterial>>,
    ResMut<'w, Assets<FloorMaterial>>,
    ResMut<'w, Assets<ShellMaterial>>,
);

/// Ground flying shadows using the card's live pose, preserving its tapped
/// heading while removing its bank. Spread grows with height up to a fixed cap.
/// Remember the original child pose so losing flying restores a contact shadow;
/// ordinary cards and pile fans retain their existing shadows.
pub fn ground_the_shadows(
    cards: Query<(&Transform, Has<Floating>), ShadowOwners>,
    mut shadows: Query<(Entity, &ChildOf, &mut Transform), With<CardShadow>>,
    mut resting: Local<HashMap<Entity, Transform>>,
) {
    resting.retain(|entity, _| shadows.get(*entity).is_ok());
    for (entity, parent, mut at) in &mut shadows {
        let Ok((card, flying)) = cards.get(parent.parent()) else {
            resting.remove(&entity);
            continue;
        };
        if !flying {
            if let Some(original) = resting.remove(&entity) {
                *at = original;
            }
            continue;
        }
        resting.entry(entity).or_insert(*at);
        // How far off the felt the card itself is. `CARD_LIFT` is the hair
        // every card is given so it does not z-fight the cloth, so a card
        // lying down measures zero here and its shadow keeps the placement it
        // was spawned with.
        let height = (card.translation.y - TABLE_Y - CARD_LIFT).max(0.0);
        let spread = 1.0 + height.min(FLOAT_SHADOW_CAP) * DECK_SHADOW_SPREAD;
        // Undo the parent bank so the shadow remains flat on the table.
        let ground = Vec3::new(
            card.translation.x + height * 0.12,
            TABLE_Y + CARD_LIFT * 0.5,
            card.translation.z + height * 0.18,
        );
        let right = card.rotation * Vec3::X;
        let yaw = (-right.z).atan2(right.x);
        let wanted = Transform {
            translation: card.to_matrix().inverse().transform_point3(ground),
            rotation: card.rotation.inverse()
                * Quat::from_rotation_y(yaw)
                * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(spread, spread, 1.0),
        };
        if *at != wanted {
            *at = wanted;
        }
    }
}

/// Where a card is when it first appears: above its mark, and a little small.
pub(super) fn entrance(target: &Transform) -> Transform {
    let mut start = *target;
    start.translation.y += ENTRANCE_RISE;
    start.scale *= ENTRANCE_SCALE;
    start
}

/// How long a card that has left the board is kept on the table.
///
/// Long enough for [`glide`] to carry it all but the last thousandth of the
/// way to its exit pose at [`SETTLE`], and short enough that a board being
/// swept does not leave a drift of ghosts behind it. A player who has turned
/// motion off never sees any of it: `glide` puts the card on its mark in one
/// frame, and what is left is a rectangle under a pile or a speck above the
/// felt for half a second.
pub(super) const EXIT_LIFE: f32 = 0.55;

/// How far under a pile's own card a card joining that pile slides.
///
/// A pile draws exactly one card — its top — so a second card arriving there
/// has to end up *behind* it or the two fight for the same depth. Half a
/// millimetre is enough for that and small enough that the card is hidden
/// rather than merely lower.
pub(super) const PILE_TUCK: f32 = 0.0005;

/// What a card shrinks to when it leaves for somewhere with no floor.
const VANISH_SCALE: f32 = 0.02;

/// How far a bounced card rises on its way off the table.
///
/// Larger than [`ENTRANCE_RISE`], because this is the entrance played
/// backwards and the card has to be *gone* by the time it is despawned rather
/// than merely high.
pub(super) const BOUNCE_RISE: f32 = 2.6;

/// A card that has left the board and is playing its way off it.
///
/// It is out of [`SceneIndex::cards`] and has lost its [`CardVisual`] the
/// moment it is marked, so nothing looks it up any more: no hover, no preview,
/// no combat line, no click. All that is left is a [`Motion`] target it is
/// gliding towards and the time it has to get there — the component moves
/// nothing itself, because everything on this table moves through one door.
#[derive(Component, Clone, Copy)]
pub struct Departing {
    /// Seconds left before it is despawned.
    pub left: f32,
}

/// A pile that is a *place* rather than a card — which is to say a library.
///
/// Every other pile is drawn through its top card, and a top card is an object
/// wearing a [`CardVisual`], so the pointer finding it is the ordinary hover
/// that finds any permanent. A library has no top card at all: it is face down
/// to everyone, its owner included (CR 401.2), and what stands there is a
/// stack of blank slabs. This is what lets the pointer find *those* — carried
/// by the topmost slab and by the backs a hover fans out of it, because the
/// rest of the deck is depth rather than cards.
#[derive(Component, Clone, Copy)]
pub struct PileVisual {
    /// Whose pile.
    pub player: PlayerId,
    /// Which pile.
    pub kind: PileKind,
}

/// Where the cards of one seat's pile stand, when that pile is drawn.
///
/// `None` for a place that has no pile on this table — the battlefield, the
/// stack, a hand — and for a seat this layout has no slot for. It is the same
/// call [`placements`] makes for a pile's top card, so a card sent here is
/// sent to the pile it is really in and not to an approximation of it.
pub(super) fn pile_stand(duel: &Duel, place: Option<Place>) -> Option<Transform> {
    let (player, kind) = match place? {
        Place::Graveyard(player) => (player, PileKind::Graveyard),
        Place::Exile(player) => (player, PileKind::Exile),
        // The second commander has a slot of its own, and this sends a
        // partner to the first one. A card is under the pile or despawned by
        // the time it matters, so the two slots are half a card apart for a
        // fraction of a second and never at rest.
        Place::Command(player) => (player, PileKind::Command),
        Place::Battlefield | Place::Stack | Place::Hand => return None,
    };
    let slot = duel.layout.as_ref()?.slot(player)?;
    Some(card_transform(slot, slot.pile_center(kind), false, 0.0))
}

/// Where a card goes as it leaves the table, by where it went.
///
/// Three exits, and which one is taken is decided by `stand` first: a card
/// bound for a pile this table draws glides *to that pile* and slides under
/// its top card, which is what the pile's own top card is doing on the same
/// frame through the update-in-place branch of [`sync_scene`]. Two creatures
/// dying together must not be treated differently for the accident of which
/// of them ends up on top.
///
/// The other two are for zones with no place on the felt. A bounce is the
/// arrival run backwards. A card this seat cannot follow at all — put on the
/// bottom of a library, or into an opponent's hand — gets the neutral shrink,
/// because the only thing that can be said about it is that it is no longer
/// here; see [`baylee_client_core::zones`] for why that is not guessed at.
pub(super) fn exit(to: Option<Place>, stand: Option<Transform>, from: &Transform) -> Transform {
    if let Some(pile) = stand {
        let mut out = pile;
        out.translation.y -= PILE_TUCK;
        return out;
    }
    let mut out = *from;
    match to {
        Some(Place::Hand) => {
            out.translation.y += BOUNCE_RISE;
            out.scale *= ENTRANCE_SCALE * 0.5;
        }
        _ => out.scale *= VANISH_SCALE,
    }
    out
}

/// Where a card is when it first appears, by where it came from.
///
/// A card coming back from a pile starts *on* that pile, so a resurrection
/// and a flicker are the pile-bound exit run backwards. Every other arrival is
/// the one this table has always drawn: a creature cast from hand comes from
/// the stack, a token comes from nowhere at all, and both of them belong
/// dropping onto their mark.
///
/// This is the branch a *buried* card takes. A card that was its pile's top
/// was already on the table and glides home through the update-in-place
/// branch, from the same point this returns — which is the whole reason the
/// two are one call.
pub(super) fn entrance_from(stand: Option<Transform>, target: &Transform) -> Transform {
    stand.unwrap_or_else(|| entrance(target))
}

/// Puts a departing card's door on the material it is already wearing.
///
/// Answers the handle to wear instead, or `None` for a card that is leaving
/// through no door worth drawing — most of them: a stale group re-keying, a
/// permanent going somewhere this seat cannot see, a card leaving on a table
/// where the player has turned motion off.
///
/// A function of its own rather than six lines inside [`sync_scene`], and the
/// reason is that this is the one frame on which it can happen at all. The
/// card is out of the board model and out of [`SceneIndex::cards`] by the
/// time this runs, so nothing will ever build it a look again — which is why
/// its exit is *written on to* the material it has rather than looked up by a
/// [`CardLook`], and why a version of this that quietly did nothing would be
/// invisible in every test that goes through the cache.
pub(super) fn dress_the_exit(
    materials: &mut Assets<CardMaterial>,
    worn: &Handle<CardMaterial>,
    step: zones::Move,
    now: f32,
    motion: f32,
) -> Option<Handle<CardMaterial>> {
    let door = step.passage().filter(|door| door.is_departure())?;
    let mut leaving = materials.get(worn).cloned()?;
    // `wear` makes the decision a card holding still makes, in the one place
    // an arrival makes it too: no sweep at all, rather than a sweep on a
    // stopped clock.
    crate::cardmat::wear(
        &mut leaving.params,
        Some(crate::sheen::Sweep::leaving(now, door, EXIT_LIFE)),
        motion,
    );
    Some(materials.add(leaving))
}

/// Despawns cards that have finished leaving.
///
/// A plain countdown rather than [`glide`]'s settled test, because one exit
/// ends at a scale of nearly zero and one ends behind another card: "has it
/// arrived" is the wrong question for a card whose destination is nowhere.
pub fn retire(
    time: Res<Time>,
    mut commands: Commands,
    mut leaving: Query<(Entity, &mut Departing)>,
) {
    for (entity, mut departing) in &mut leaving {
        departing.left -= time.delta_secs();
        if departing.left <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
