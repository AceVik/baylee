//! The felt and the seats' mats, kept in step with the board.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// The well material for one pile kind.
pub(super) fn well_of(
    index: &SceneIndex,
    kind: baylee_client_core::PileKind,
) -> Option<Handle<StandardMaterial>> {
    let at = baylee_client_core::PileKind::ALL
        .iter()
        .position(|k| *k == kind)?;
    index.wells.get(at).cloned()
}

/// One seat's mat, as the handful of numbers its shader draws it from.
///
/// The size is the mat's own, in table units, and that is the difference the
/// whole material exists for: the corner and the rim are lengths on this
/// board rather than fractions of an image that was then stretched over it.
pub(super) fn mat_params(
    accent: Color,
    size: Vec2,
    mood: Mood,
    moving: bool,
    ledge_outer: bool,
) -> crate::matmat::MatParams {
    let rgb = accent.to_linear();
    crate::matmat::MatParams {
        accent: Vec4::new(rgb.red, rgb.green, rgb.blue, zone_brightness(mood)),
        size,
        corner: tabletop::MAT_CORNER,
        rim: tabletop::MAT_RIM,
        on_turn: if mood.on_turn { 1.0 } else { 0.0 },
        awaited: if mood.standing == Standing::Asked {
            1.0
        } else {
            0.0
        },
        held: if mood.held { 1.0 } else { 0.0 },
        motion: if moving {
            crate::cardmat::MOVING
        } else {
            crate::cardmat::STILL
        },
        ledge_outer: if ledge_outer { 1.0 } else { 0.0 },
    }
}

/// How bright a zone's mat is drawn, given what it is saying.
///
/// A seat that has lost fades most of the way out — its permanents are gone
/// and its zone should stop competing for attention — and the seat being
/// asked is the brightest thing on the felt, because that is the seat
/// everyone else is waiting for.
pub(super) fn zone_brightness(mood: Mood) -> f32 {
    // Every value here is a multiplier on the mat's **opacity**, so 1.0 is
    // the ceiling and anything past it is not brighter, it is clipped. It has
    // been the ceiling since the accent moved off the material's tint and
    // into the mat itself: at 1.311 and 1.0925 a local seat being asked
    // and a local seat merely taking its turn would be drawn identically,
    // which is precisely the distinction the mat exists to draw.
    //
    // Opacity rather than colour is what the shader does with it, and that
    // reading is the one the numbers were chosen for anyway: a seat that has
    // lost fades *into* the felt at 0.22, where scaling a colour would have
    // left it drawing a dark grey rim just as visible as everybody else's.
    let standing = match mood.standing {
        Standing::Lost => 0.22,
        Standing::Waiting => 0.62,
        Standing::Active => 0.78,
        Standing::Asked => 1.0,
    };
    // Being the viewing seat is a lift, never a rank. Which mat is mine is
    // answered by the gilt rim and does not need brightness spent on it, and
    // a `local` term big enough to outrank a standing would let my own idle
    // mat outshine the opponent everybody is actually waiting for.
    if mood.local {
        (standing * LOCAL_LIFT).min(1.0)
    } else {
        standing
    }
}

/// How much brighter the viewing seat's own mat is drawn at equal standing.
///
/// Small on purpose: see [`zone_brightness`]. The bound that keeps it honest
/// is `zone_tests::a_standing_always_outranks_being_the_local_seat`.
const LOCAL_LIFT: f32 = 1.10;

/// What the firewheel should be burning at, packed the way the shader reads
/// it: `(white, blue, black, red)` and `(green, up.x, up.y, spare)`.
///
/// The strengths come from what can make coloured mana on the **whole
/// table** — every seat's, not this one's — because the wheel in the middle
/// belongs to the table rather than to a chair. `up` is screen-up in table
/// space: the local seat's own inward direction, which is one direction for
/// all five flames and is what makes them five candles seen from one chair
/// instead of a sun glyph.
fn firewheel_of(
    duel: &Duel,
    board: &baylee_client_core::board::BoardModel,
    layout: &TableLayout,
) -> (Vec4, Vec4) {
    let seats = board.pods.len().max(1);
    let burn = duel.view.as_ref().map_or([0.0; 5], |view| {
        baylee_client_core::firewheel::strength(crate::manasources::table_mana(view), seats)
    });
    let up = layout
        .local()
        .and_then(|slot| (-slot.center).try_normalize())
        .unwrap_or(Vec2::Y);
    (
        Vec4::new(burn[0], burn[1], burn[2], burn[3]),
        Vec4::new(burn[4], up.x, up.y, 0.0),
    )
}

/// Cuts the slab to the table and keeps the rail lit by the turn.
///
/// Two jobs in one system because they need the same three things — the
/// layout, the board and the slab entity — and because the second is
/// meaningless without the first having run.
///
/// **Cutting.** The table is no longer a fixed quad. It is made to whatever
/// [`TableLayout::extent`] reports plus [`SLAB_MARGIN`] of table, which
/// changes with the seat count, the focused pod and the window, and it is cut
/// as a **racetrack** with a real thickness rather than as a rectangle one
/// pixel deep — see [`tabletop::table_corner`] for what the corners are for.
/// The cut is guarded on the size, so a table that has not changed shape
/// costs nothing.
///
/// **Lighting.** The step is on the board model, so this needs no engine and
/// no view of its own; the arithmetic — which colour a step is worth — lives
/// in [`tabletop::phase_light`], where it can be argued with in a test rather
/// than looked at in a screenshot. It writes only when the colour actually
/// moves: a lamp that reached its target and kept writing would touch a
/// material every frame for the rest of the game, which is exactly the
/// garbage [`sync_zones`] exists to avoid.
#[allow(clippy::too_many_lines)] // slab creation and incremental material update share one state
#[allow(clippy::too_many_arguments)] // Bevy system: the slab, its assets and the settings it reads
pub fn sync_table(
    mut commands: Commands,
    time: Res<Time>,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FeltMaterial>>,
    mut slabs: Query<(&mut Slab, &mut Mesh3d, &MeshMaterial3d<FeltMaterial>)>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    // Cut to the table the tear ends on: a slab that followed its stages
    // would be re-cut three times in a second.
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.settled_layout()) else {
        return;
    };
    let Some((min, max)) = layout.extent() else {
        return;
    };

    // Measured about the origin rather than about the extent's own centre,
    // because the origin is where the firewheel burns and where the pool
    // is centred. For every seat count that can actually be played the ring
    // is symmetric and the two agree.
    let reach = min.abs().max(max.abs());
    let span = (reach + Vec2::splat(SLAB_MARGIN)) * 2.0;

    let motion = if crate::quality::ambient_still(prefs.all().reduce_motion, quality.as_deref()) {
        crate::cardmat::STILL
    } else {
        crate::cardmat::MOVING
    };

    // Where the light enters: the active seat's own shore, running inward
    // round the rail. At two seats that is the reference photograph —
    // cool at one bank, molten at the other — and at six it is a pool lit
    // from whoever's turn it is, which is the same statement without needing
    // the table to have ends.
    let source = board
        .pods
        .iter()
        .find(|pod| pod.is_active)
        .and_then(|pod| layout.shown(pod.player))
        .map_or(Vec4::new(0.0, -1.0, 0.0, 1.0), |slot| {
            let inward = (-slot.center).try_normalize().unwrap_or(Vec2::Y);
            Vec4::new(slot.center.x, slot.center.y, inward.x, inward.y)
        });

    let (want_flames, want_tail) = firewheel_of(&duel, board, layout);
    let up = want_tail.yz();

    let want = crate::feltmat::wash_of(board.step);
    // Two rates, one decision: with reduce-motion on, both arrive at once.
    let (ease, fire_ease) = if prefs.all().reduce_motion {
        (1.0, 1.0)
    } else {
        let step = time.delta_secs();
        (
            1.0 - (-WASH_RATE * step).exp(),
            1.0 - (-FIRE_RATE * step).exp(),
        )
    };

    let Ok((mut slab, mut mesh, handle)) = slabs.single_mut() else {
        // No slab yet. Cut one, and let the next frame light it.
        let (veins, vein_offsets) = crate::feltmat::vein_points(span, duel.table_pattern.0);
        commands.spawn((
            DuelStage,
            crate::dial::DialFace::default(),
            Slab {
                cut: span,
                shown: Vec4::ZERO,
                source,
                // Cut dark and let the next frame light it, like the lamp
                // above: five flames at the pilot light is what zero means,
                // and a table cut before a board has arrived is simply the
                // table with its wheel banked down.
                flames: Vec4::ZERO,
                tail: Vec4::new(0.0, up.x, up.y, 0.0),
                motion,
            },
            // The floor of the scene answers no clicks: a pointer on bare
            // cloth means the table, not the thing under it.
            Pickable::IGNORE,
            Mesh3d(meshes.add(slab_mesh(span))),
            MeshMaterial3d(materials.add(FeltMaterial {
                params: crate::feltmat::FeltParams {
                    wash: Vec4::ZERO,
                    source,
                    // No light until the sky has read a clock. The cloth's
                    // own colour is what `a = 0` means, so a table that is
                    // cut before the first `sync_sky` is simply the table.
                    ambient: Vec4::new(1.0, 1.0, 1.0, 0.0),
                    flames: Vec4::ZERO,
                    flames_tail: Vec4::new(0.0, up.x, up.y, 0.0),
                    span,
                    corner: tabletop::table_corner(span),
                    rail: tabletop::RAIL_WIDTH,
                    motion,
                    gain: crate::feltmat::WASH_GAIN,
                    thickness: TABLE_THICKNESS,
                    seats: 0.0,
                    pattern: duel.table_pattern.0,
                    veins: vein_offsets,
                    // The dial is written by `dial::turn_the_dial` once a
                    // table is there; until then no hand and no jewel.
                    hands: Vec4::ZERO,
                    dial: Vec4::new(0.0, 0.0, -100.0, -100.0),
                    pulse: Vec4::new(-100.0, 0.0, 0.0, 0.0),
                    jewels: [Vec4::ZERO; 4],
                    tints: [Vec4::ZERO; 8],
                    teams: [Vec4::ZERO; 8],
                    rift: Vec4::ZERO,
                },
                veins: images.add(veins),
            })),
            Transform::from_xyz(0.0, TABLE_Y, 0.0)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        return;
    };

    let recut = (slab.cut - span).abs().max_element() > 1e-3;
    let next = slab.shown + (want - slab.shown) * ease;
    // Below a step this small nothing on screen changes, so stop. A slab that
    // reached its colour and went on writing would touch a material — and so
    // a uniform upload — every frame for the rest of the game.
    // The flames ease on their own rate, slower than the lamp: the lamp is
    // a step changing and the wheel is a board filling up.
    let next_flames = slab.flames + (want_flames - slab.flames) * fire_ease;
    let next_tail = slab.tail + (want_tail - slab.tail) * fire_ease;
    // The flicker itself runs on `globals.time` inside the shader and needs
    // no upload at all, so a wheel that has reached its heights stops
    // touching the material and goes on burning.
    let still = (next - slab.shown).abs().max_element() <= 1e-4
        && (next_flames - slab.flames).abs().max_element() <= 1e-4
        && (next_tail - slab.tail).abs().max_element() <= 1e-4
        && (slab.source - source).abs().max_element() <= 1e-4
        && (slab.motion - motion).abs() <= f32::EPSILON;
    if still && !recut {
        return;
    }
    slab.shown = next;
    slab.flames = next_flames;
    slab.tail = next_tail;
    slab.source = source;
    slab.motion = motion;

    if recut {
        slab.cut = span;
        *mesh = Mesh3d(meshes.add(slab_mesh(span)));
    }
    if let Some(mut material) = materials.get_mut(&handle.0) {
        if recut {
            material.params.span = span;
            material.params.corner = tabletop::table_corner(span);
            // A bigger slab reaches cells the old table did not hold.
            let (veins, vein_offsets) = crate::feltmat::vein_points(span, duel.table_pattern.0);
            images.remove(&material.veins);
            material.veins = images.add(veins);
            material.params.veins = vein_offsets;
        }
        material.params.wash = next;
        material.params.flames = next_flames;
        material.params.flames_tail = next_tail;
        material.params.source = source;
        material.params.motion = motion;
    }
}

/// The slab of a table this size: a racetrack with a real thickness, whose
/// **top face lies exactly at the plane every other thing on the stage is
/// placed against**.
///
/// The body hangs below that plane rather than standing on it, which is the
/// one thing that has to be got right: raise the surface by the thickness and
/// every mat, glow and card is buried inside the table.
pub(super) fn slab_mesh(span: Vec2) -> Mesh {
    rounded_slab_mesh(
        span.x,
        span.y,
        tabletop::table_corner(span),
        0.0,
        -TABLE_THICKNESS,
        SLAB_SEGMENTS,
    )
}

/// Keeps one mat and one glow per seat in step with the table.
///
/// Zones are spawned when a seat first appears and only ever re-tinted after
/// that: the layout does not move once a game has begun, and a mat rebuilt
/// every frame would be four meshes and four materials of pure garbage per
/// frame for a table nobody is looking at that hard.
#[allow(clippy::too_many_lines)] // one seat's ground, glow and piles in one pass
pub fn sync_zones(
    mut commands: Commands,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut index: ResMut<SceneIndex>,
    mut meshes: ResMut<Assets<Mesh>>,
    (mut mats, mut materials): (
        ResMut<Assets<crate::matmat::MatMaterial>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    mut placed: Query<(&Transform, Option<&mut Motion>, Option<&Seated>)>,
) {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return;
    };
    let Some(glow_image) = index.glow_image.clone() else {
        return;
    };
    let moving = !prefs.all().reduce_motion;

    let mut seen: HashSet<PlayerId> = HashSet::new();
    for pod in &board.pods {
        // A parked seat has no mat: left out of `seen`, its zone is taken
        // down below like a seat that left — unless it is swinging out of
        // a tear, when its mat goes with its cards.
        let _ = layout;
        let Some(slot) = duel.drawn_slot(pod.player) else {
            continue;
        };
        seen.insert(pod.player);
        let mood = Mood::of(pod);
        let accent = seat_accent(slot);
        let size = slot.half_extent * 2.0 + Vec2::splat(ZONE_MARGIN * 2.0);
        let params = mat_params(
            accent,
            size,
            mood,
            moving,
            baylee_client_core::layout::LEDGE_IS_OUTER,
        );
        // Two different colours, not one with a dimmer on it. The mat carries
        // the seat's colour in its own rim and nowhere else, so a tint over
        // the whole thing would put the accent on the felt as well; what the
        // mat takes from the mood is brightness alone, and it takes it as an
        // alpha. The glow beneath is the opposite case — a white falloff
        // whose entire job is to spill the seat's colour onto the table.
        let glow_tint = accent.to_linear() * zone_brightness(mood) * GLOW_STRENGTH;

        // The one fact about a pile the *places* depend on. A seat whose
        // library has run out loses on its next draw, and the table stops
        // showing a stack there — which is worth a rebuild; how many cards
        // are in a library that still has some is not, and is on the tab.
        let library_shown = pod.library_count.min(SHOWN_LIBRARY);
        // A seat that has moved or changed size is rebuilt rather than
        // updated: its mat is a mesh cut to a width, its glow is a quad and
        // its piles stand at fixed points, and not one of those is something
        // a uniform can carry. See [`Zone::slot`] for what this cost.
        // How high the seat's board rides: a piece of a tearing table turns
        // lifted over the rest, and the ground on it with its cards.
        let lift = duel.tear.as_ref().map_or(0.0, |t| t.lift(pod.player));
        let moved = index
            .zones
            .get(&pod.player)
            .is_some_and(|zone| zone.slot != *slot || (zone.raised - lift).abs() > 1e-5);
        // A zone whose seat only moved — the same ground, somewhere else or
        // turned (a tear's stages, a seat brought across) — glides there with
        // its cards: its mat, its glow and its piles are retargeted as one
        // rigid body. Only a change of size builds it again.
        if moved
            && let Some(zone) = index.zones.get_mut(&pod.player)
            && same_ground(&zone.slot, slot)
        {
            // Each part's place in its seat's own frame, read once off where
            // it was built and kept: every later target is that place in the
            // seat's frame as it stands now, so no move compounds on the one
            // before it — a carry composed on the last target drifted, and a
            // part that missed one frame's carry stayed off by it for good.
            let lifted = Mat4::from_translation(Vec3::Y * lift);
            for entity in [zone.mat, zone.glow]
                .into_iter()
                .chain(zone.piles.iter().copied())
            {
                let Ok((at, motion, seated)) = placed.get_mut(entity) else {
                    continue;
                };
                let local = seated.map_or_else(
                    || {
                        let rest = Mat4::from_translation(Vec3::Y * -zone.raised) * at.to_matrix();
                        seat_frame(&zone.slot).inverse() * rest
                    },
                    |seated| seated.0,
                );
                let target = Transform::from_matrix(lifted * seat_frame(slot) * local);
                match motion {
                    Some(mut motion) => motion.target = target,
                    None => {
                        commands.entity(entity).insert(Motion { target });
                    }
                }
                if seated.is_none() {
                    commands.entity(entity).insert(Seated(local));
                }
            }
            zone.slot = *slot;
            zone.raised = lift;
        } else if moved && let Some(zone) = index.zones.remove(&pod.player) {
            for entity in [zone.mat, zone.glow].into_iter().chain(zone.piles) {
                commands.entity(entity).despawn();
            }
        }
        if let Some(zone) = index.zones.get(&pod.player) {
            let stale_piles = zone.library_shown != library_shown;
            if zone.mood == mood && zone.accent == accent && !stale_piles {
                continue;
            }
            let old_piles = zone.piles.clone();
            // Only the numbers change; the mesh and the transform still hold.
            // The accent moved through here too — it is a uniform now rather
            // than a texture that would have to be generated again.
            if let Some(mut material) = mats.get_mut(&zone.mat_material) {
                material.params = params;
            }
            if let Some(mut material) = materials.get_mut(&zone.glow_material) {
                material.base_color = Color::LinearRgba(glow_tint);
            }
            let fresh = stale_piles.then(|| {
                for entity in old_piles {
                    commands.entity(entity).despawn();
                }
                spawn_piles(&mut commands, &index, slot, &pod.piles, pod.library_count)
            });
            index.zones.entry(pod.player).and_modify(|zone| {
                zone.mood = mood;
                zone.accent = accent;
                zone.library_shown = library_shown;
                if let Some(fresh) = fresh {
                    zone.piles = fresh;
                }
            });
            continue;
        }
        let mat_material = mats.add(crate::matmat::MatMaterial { params });
        let mat = commands
            .spawn((
                DuelStage,
                Mesh3d(meshes.add(Rectangle::new(size.x, size.y))),
                MeshMaterial3d(mat_material.clone()),
                Pickable::IGNORE,
                lying_flat(slot, ZONE_LIFT),
                SeatMat(pod.player),
            ))
            .id();
        let (glow, glow_material) = spawn_table_quad(
            &mut commands,
            &mut meshes,
            &mut materials,
            slot,
            TableQuad {
                size: size + Vec2::splat(GLOW_SPREAD * 2.0),
                lift: GLOW_LIFT,
                tint: glow_tint,
                texture: glow_image.clone(),
            },
        );
        let piles = spawn_piles(&mut commands, &index, slot, &pod.piles, pod.library_count);
        index.zones.insert(
            pod.player,
            Zone {
                slot: *slot,
                mat,
                glow,
                mat_material,
                glow_material,
                piles,
                // Built on its place at the table's height; a lift is the
                // next frame's carry.
                raised: 0.0,
                library_shown,
                mood,
                accent,
            },
        );
    }

    // A seat that left the table takes its zone with it.
    index.zones.retain(|player, zone| {
        if seen.contains(player) {
            return true;
        }
        for entity in [zone.mat, zone.glow].into_iter().chain(zone.piles.clone()) {
            commands.entity(entity).despawn();
        }
        false
    });
}

/// A seat's mat, by seat: what `/state.arrangement.mats` reads to say
/// where every mat is drawn against where its seat's slot is.
#[derive(Component, Clone, Copy, Debug)]
pub struct SeatMat(pub PlayerId);

/// Whether two slots are the same ground: a zone built for one can be
/// carried to the other whole.
fn same_ground(a: &SeatSlot, b: &SeatSlot) -> bool {
    a.half_extent.abs_diff_eq(b.half_extent, 1e-5)
        && (a.reclaimed - b.reclaimed).abs() <= 1e-5
        && (a.scale - b.scale).abs() <= 1e-5
}

/// A seat's own frame in the world: its centre on the felt, turned by its
/// facing.
pub(super) fn seat_frame(slot: &SeatSlot) -> Mat4 {
    Mat4::from_rotation_translation(
        Quat::from_rotation_y(-slot.facing),
        to_world(slot.center, 0.0),
    )
}

/// A zone part's place in its seat's own frame (`seat_frame`), at the
/// table's height: kept from the first time the zone moves, so every move
/// after is that place in the seat's frame as it then stands.
#[derive(Component, Clone, Copy, Debug)]
pub struct Seated(pub Mat4);

#[cfg(test)]
mod rigid_tests {
    use super::*;

    /// A thing lying on a seat's ground, carried to another seat's, lies on
    /// that ground exactly where it lay on the first: the mat's centre goes
    /// to the new centre, turned by the new facing.
    #[test]
    fn a_zone_carried_rigidly_lands_where_it_would_be_built() {
        let slot = |x: f32, y: f32, facing: f32| SeatSlot {
            player: baylee_core::ids::PlayerId::new(1),
            ring_index: 1,
            angle: facing,
            center: Vec2::new(x, y),
            facing,
            half_extent: Vec2::new(6.0, 3.0),
            reclaimed: 0.0,
            is_local: false,
            scale: 1.0,
            parked: false,
        };
        let (a, b) = (slot(3.0, 9.0, 1.2), slot(-4.0, 12.0, std::f32::consts::PI));
        for (from, to) in [(a, b), (b, a)] {
            // A part's place in its seat's own frame, read off where it was
            // built on `from`, and set into `to`'s frame.
            let carry = |at: Transform| {
                let local = seat_frame(&from).inverse() * at.to_matrix();
                Transform::from_matrix(seat_frame(&to) * local)
            };
            let built = lying_flat(&to, 0.01);
            let moved = carry(lying_flat(&from, 0.01));
            assert!(moved.translation.distance(built.translation) < 1e-4);
            assert!(moved.rotation.angle_between(built.rotation) < 1e-3);
            let pile =
                |s: &SeatSlot| card_transform(s, s.pile_center(PileKind::Graveyard), false, 0.0);
            let moved = carry(pile(&from));
            assert!(moved.translation.distance(pile(&to).translation) < 1e-4);
        }
    }
}
