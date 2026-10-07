//! Spawning and despawning the stage: the slab, the meshes, the piles.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// Marks everything spawned for the duel, so closing it is one despawn.
#[derive(Component)]
pub struct DuelStage;

/// The table itself: one slab of baize inside a padded rail.
///
/// It carries the two things that have to survive a frame. `cut` is the size
/// the slab was last made at, so a mesh is only rebuilt when the table
/// actually changes shape. `shown` is the *eased* phase lamp, because easing
/// towards a target needs somewhere to keep where it started.
#[derive(Component)]
pub struct Slab {
    /// The world size the slab was cut to.
    pub(super) cut: Vec2,
    /// The lamp currently on the rail: `rgb` its colour, `w` its energy.
    pub(super) shown: Vec4,
    /// Where the lamp was last entering the rail from.
    pub(super) source: Vec4,
    /// The firewheel's five eased strengths, packed the way the shader
    /// reads them: `flames` is white, blue, black, red and `tail.x` green.
    ///
    /// Eased for the same reason the lamp is, and on the same clock: a
    /// flame that grows over a second and a half is a fire being fed, and
    /// one that jumps when a land enters play is a notification.
    pub(super) flames: Vec4,
    pub(super) tail: Vec4,
    /// The motion setting the pulse was last running at.
    pub(super) motion: f32,
}

/// A card's corner radius.
///
/// A real card is 63 mm wide with a 3 mm corner, and this is exactly that —
/// 4.76%, the same `PRINTED_CORNER` the two card shaders cut at. The geometry
/// takes the scanner's white corner away and the shader inks the sliver of
/// pixels the mesh edge antialiases through, which only works while both
/// agree: a mesh cut wider than the print leaves the ink nothing to do, and a
/// mesh cut narrower shows white outside it. It used to be 10%, which removed
/// the white by removing a tenth of the card with it and made every permanent
/// read as a token.
pub const CARD_CORNER: f32 = CARD_WIDTH * 0.0476;

/// How thick a card is, in table units.
///
/// A real card at this scale is about a fiftieth of this — it would be a
/// single pixel at any camera distance a player uses. The point of the
/// thickness is not accuracy but that a card reads as an *object lying on*
/// the table rather than a decal printed into it, so it is exaggerated until
/// the edge is visible and stopped well before a card looks like a tile.
pub const CARD_THICKNESS: f32 = CARD_WIDTH * 0.055;

/// How far past the card its contact shadow spreads, as a fraction of the
/// card's width.
const SHADOW_SPREAD: f32 = 0.22;

/// A card: a rounded rectangle with the printed face on top and a thin wall
/// around its edge.
///
/// The face is UV-mapped exactly like Bevy's `Rectangle` (uv.x left→right,
/// uv.y top→bottom of the printed face) and the wall borrows the UV of the
/// face vertex above it — so a card's edge is whatever colour its border is,
/// which for most cards is the black frame and reads as exactly the right
/// thing. There is no bottom face: the camera rig never goes below the table,
/// and two hundred cards is four hundred triangles worth saving.
pub(super) fn rounded_card_mesh(width: f32, height: f32, radius: f32) -> Mesh {
    rounded_slab_mesh(width, height, radius, CARD_THICKNESS, 0.0, 4)
}

/// The same slab at any thickness, and with any number of corner segments.
///
/// The table is built by this too, and the two want opposite things from it:
/// a card is 63 mm with a 3 mm corner and four segments is more than the eye
/// can find, while the table is a **racetrack** whose corners are metres
/// across and would read as a chamfer at four. `top` and `bottom` are the two
/// heights the wall runs between, so a card sits *on* the plane it is placed
/// at (`CARD_THICKNESS` to 0) and the table hangs *below* it (0 to
/// `-TABLE_THICKNESS`) — which is what keeps the table's surface exactly
/// where every other thing on the stage is positioned against.
pub(super) fn rounded_slab_mesh(
    width: f32,
    height: f32,
    radius: f32,
    top: f32,
    bottom: f32,
    segments: usize,
) -> Mesh {
    let segments = segments.max(1);
    let (hw, hh) = (width / 2.0, height / 2.0);
    // A radius past half the short side has no shape left to describe, and
    // would fold the outline through itself exactly the way the bowtie did.
    let r = radius.clamp(0.0, hw.min(hh));
    // Corner arc centres in CCW order with the quarter turn each one sweeps,
    // angles measured the usual way (0° = +x, 90° = +y).
    //
    // Every centre owns the quarter that points *away* from the middle of the
    // card, and it has to: pair a centre with any other quarter and the
    // outline folds back through the centre, so the fan below stitches
    // crossing slivers instead of a card. On a table that reads as a small
    // bright X where a permanent should be.
    let corners: [([f32; 2], f32); 4] = [
        ([hw - r, hh - r], 90.0),
        ([-hw + r, hh - r], 180.0),
        ([-hw + r, -hh + r], 270.0),
        ([hw - r, -hh + r], 360.0),
    ];
    // The outline, with the outward direction at each point — which for an
    // arc point is simply the angle it was drawn at, and is what the wall's
    // normals are.
    let mut outline: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for ([cx, cy], end_deg) in corners {
        let start_deg = end_deg - 90.0;
        for i in 0..=segments {
            #[expect(clippy::cast_precision_loss)] // a handful of segments
            let a = (start_deg + (end_deg - start_deg) * i as f32 / segments as f32).to_radians();
            let (dx, dy) = (a.cos(), a.sin());
            outline.push(([cx + r * dx, cy + r * dy], [dx, dy]));
        }
    }

    // Same mapping as Rectangle: [hw,hh]→[1,0], [-hw,-hh]→[0,1].
    //
    // Clamped, because the outline is built as `centre + r·cos θ`, and at
    // θ = 0 that is `hw - r + r`, which in binary is not always `hw`. A UV a
    // ten-millionth outside the texture samples the wrap or the clamp
    // depending on the backend, so the card would grow a bright thread down
    // one edge on exactly one machine.
    let uv_of = |x: f32, y: f32| {
        [
            f32::midpoint(x / hw, 1.0).clamp(0.0, 1.0),
            ((1.0 - y / hh) * 0.5).clamp(0.0, 1.0),
        ]
    };

    // The face: a centre vertex and the outline, all facing straight up.
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, top]];
    let mut normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]];
    let mut uvs: Vec<[f32; 2]> = vec![[0.5, 0.5]];
    for ([x, y], _) in &outline {
        positions.push([*x, *y, top]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push(uv_of(*x, *y));
    }
    // A triangle fan from the centre, wound counter-clockwise as seen from
    // +z — the side the printed face is on, and the side the camera is on
    // once the card is laid down. The material does not disable back-face
    // culling, so the other winding is an invisible card.
    let m = outline.len();
    let mut indices = Vec::with_capacity(m * 9);
    for i in 1..m {
        indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
    }
    indices.extend_from_slice(&[0, m as u32, 1]);

    // The wall: the outline again at both heights, with its own normals, so
    // the face above it keeps a clean flat shade.
    let wall = positions.len() as u32;
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, top]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for ([x, y], [nx, ny]) in &outline {
        positions.push([*x, *y, bottom]);
        normals.push([*nx, *ny, 0.0]);
        uvs.push(uv_of(*x, *y));
    }
    for i in 0..m {
        let next = (i + 1) % m;
        let (t0, t1) = (wall + i as u32, wall + next as u32);
        let (b0, b1) = (t0 + m as u32, t1 + m as u32);
        // Down the near edge, along the bottom, back up: that is the order
        // whose normal points away from the card. The other one is a card you
        // can see straight through from the side.
        indices.extend_from_slice(&[t0, b0, b1, t0, b1, t1]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

/// Builds the stage: camera, light, and felt.
pub fn spawn_stage(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cards: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut index: ResMut<SceneIndex>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
) {
    index.quad = Some(meshes.add(rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER)));
    // The objects lying on and round a card, each one quad for the whole
    // table, sized in card widths across and down the card.
    let mut quad =
        |size: Vec2| Some(meshes.add(Rectangle::new(size.x * CARD_WIDTH, size.y * DOWN_THE_CARD)));
    index.marks_quad = quad(crate::marksmat::quad_size());
    index.badge_quad = quad(crate::badgemat::quad_size());
    index.plate_quad = quad(crate::platemat::quad_size());
    index.floor_quad = quad(floormat::quad_size());
    let [inner, outer] = shellmat::RIM_EDGES;
    index.rim_mesh = Some(meshes.add(shellmat::band_mesh(inner, outer)));
    for band in Band::ALL {
        let [inner, outer] = band.edges();
        index
            .ring_meshes
            .insert(band, meshes.add(shellmat::band_mesh(inner, outer)));
    }
    for dome in shellmat::Dome::ALL {
        for (step, shape) in shellmat::DOME_STEPS.into_iter().enumerate() {
            index.dome_meshes.insert(
                (dome, step),
                meshes.add(shellmat::dome_mesh(dome.row(), shape)),
            );
            if let Some(shade) = shellmat::dome_shade_mesh(dome.row(), shape) {
                index.dome_shades.insert((dome, step), meshes.add(shade));
            }
        }
    }
    index.wall_mesh = Some(meshes.add(shellmat::wall_mesh()));
    index.wall_shade = Some(meshes.add(shellmat::wall_shade_mesh()));
    index.wave_mesh = Some(meshes.add(shellmat::wave_mesh()));

    // The contact shadow: a quad a little larger than a card, carrying a
    // painted halo that is dense under the card and gone by its own edge.
    // Sized from the card so the falloff is the same width on all four sides
    // — a square texture stretched over a card-shaped quad would be wider at
    // the top than at the sides, which is the sort of thing nobody can name
    // but everybody sees.
    let spread = CARD_WIDTH * SHADOW_SPREAD;
    let (shadow_w, shadow_h) = (CARD_WIDTH + 2.0 * spread, CARD_HEIGHT + 2.0 * spread);
    index.shadow_quad = Some(meshes.add(Rectangle::new(shadow_w, shadow_h)));
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "a texture size derived from two positive constants"
    )]
    let shadow_px = (128.0 * shadow_h / shadow_w).round() as u32;
    index.shadow_material = Some(materials.add(StandardMaterial {
        base_color_texture: Some(images.add(image_of(&tabletop::card_shadow(
            128,
            shadow_px,
            spread / shadow_w,
            CARD_CORNER / CARD_WIDTH,
        )))),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        // A card's shadow is the highest thing the table's ground carries,
        // and it is what the air is kept underneath.
        depth_bias: sort_bias(CARD_LIFT * 0.5),
        ..default()
    }));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a texture size derived from two positive constants"
    )]
    let well_px = (f32::from(u16::try_from(RECESS_PX).unwrap_or(u16::MAX)) * CARD_HEIGHT
        / CARD_WIDTH)
        .round() as u32;
    index.wells = baylee_client_core::PileKind::ALL
        .into_iter()
        .map(|pile| {
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image_of(&tabletop::card_well(
                    RECESS_PX,
                    well_px,
                    CARD_CORNER / CARD_WIDTH,
                    pile.mark(),
                )))),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect();
    // The card back: no finish, no glow, and no picture *yet*. It is what a
    // library is drawn as, slab by slab (CR 401.2: face down), and what a
    // card this seat may not see wears. The slabs under a card on the table
    // are not backs but frames (`sync_stack`). The printed back is
    // fetched like any other image, so `sync_scene` dresses this material in
    // it the frame it lands; until then it is the flat colour below.
    // A second duel in one session gets a second material, and the flag is
    // about *this* one: left standing, the new material would never be
    // dressed and every hidden card would go back to being a dark rectangle.
    index.back_dressed = false;
    index.blank = Some(cards.add(material(
        CardLook::flat(BACK_COLOR, FinishTreatment::Plain),
        None,
        BACK_COLOR,
        // No finish and no glow: the clock drives the foil sheen and a card
        // back is plain, so there is nothing on this material for it to
        // reach even once the picture is on it. That is why this one is
        // exempt from the cache the two below live in, and why dressing it
        // in the back is a change to the material rather than a new one.
        MOVING,
    )));

    commands.spawn((
        DuelStage,
        TableCamera,
        Camera3d::default(),
        // Looking down the table from behind the local seat (the default
        // rig; apply_camera_rig owns the transform from here on).
        Transform::from_xyz(0.0, 15.0, 13.2).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV,
            ..default()
        }),
        // No tone mapping. Bevy attaches none to a camera by default, so
        // this is belt and braces rather than a fix — but it is the right
        // thing to say out loud: everything in this scene is unlit and
        // display-referred (a generated texture says what the table should
        // *look* like, and a card's art is the same PNG the hand draws
        // unaltered through the UI pass), so a tone mapper reading those
        // numbers as radiance would be wrong. Naming it here stops a future
        // default from quietly doing that.
        Tonemapping::None,
        // The same answer the lobby's camera gets, from the same place:
        // `Msaa` is a component in bevy 0.19, so a driver workaround has to
        // be repeated on every camera rather than set once. Both need it
        // rather than only the one the defect was caught on — it is in the
        // tiler's multisample resolve, not in anything the lobby does.
        crate::gpu::msaa(adapter.as_deref()),
    ));

    // Nothing below is lit, and nothing above it is either: card art must
    // never be tinted by scene lighting, because a player has to be able to
    // read a card's colour identity at a glance. The table gets its depth
    // from painted-in shading instead — which is what `tabletop` generates,
    // and why this stage has no light in it at all.
    index.glow_image = Some(images.add(image_of(&tabletop::glow(128))));

    // The slab itself — the baize and the rail around it — is not spawned
    // here. It is cut to the layout, and at this point there may not be one:
    // `sync_table` makes it the first time a board arrives and re-cuts it
    // whenever the seating or the window changes.

    // Nothing is spawned for the middle of the table either. The colour
    // wheel used to be a quad here with a 512-texel medallion on it — five
    // soft discs of the pie on two rings of worn gold — and it is five
    // flames painted into `felt.wgsl` now, because the light a fire throws
    // has to be *added to the cloth* to have the weave show through it, and
    // a quad blended over the felt can only ever cover it.
    // `baylee_client_core::firewheel` is normative.
}

/// Wraps a generated texture in an `Image` the renderer can bind.
pub(crate) fn image_of(texture: &tabletop::Texture) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture.rgba.clone(),
        // sRGB: the generator writes what the table should *look* like, not
        // light values, so the samples are display-referred.
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // A mat is stretched over a quad much larger than the texture is;
    // without a linear filter its soft edges come out as stairs.
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// The colour a seat's zone is drawn in.
///
/// The viewing seat is gilt, matching the firewheel's rings: whatever else is
/// on the table, "mine" is the one edge a player never has to look for. The
/// others take the colours of the pie in ring order, which makes a four-way
/// game four distinguishable places rather than three anonymous opponents.
pub(crate) fn seat_accent(slot: &SeatSlot) -> Color {
    if slot.is_local {
        return Color::srgb(0.78, 0.63, 0.33);
    }
    let hue = tabletop::PIE[(slot.ring_index + 3) % tabletop::PIE.len()];
    Color::srgb(hue[0], hue[1], hue[2])
}

/// One flat thing lying on the table.
///
/// A struct rather than four more parameters, because it already needs a seat
/// and two asset stores and clippy's argument budget is seven.
///
/// It used to describe both halves of a zone. The mat is drawn by its own
/// material now — see [`crate::matmat`] — so what still comes through here is
/// the glow underneath, which is a soft round falloff with no edge in it and
/// therefore the one case a stretched image is actually good at.
pub(super) struct TableQuad {
    /// Extent on the felt, in table units.
    pub(super) size: Vec2,
    /// How far above [`TABLE_Y`] it lies. The order of these decides what
    /// draws over what, through [`sort_bias`] — which is newer than this
    /// comment, and is what finally made the sentence true.
    pub(super) lift: f32,
    /// Multiplied into the texture, so a white texel comes out this colour.
    pub(super) tint: LinearRgba,
    /// The image, whose alpha is the shape.
    pub(super) texture: Handle<Image>,
}

/// Spawns one of them, lying flat and facing its seat.
///
/// Returns the material beside the entity, because the caller keeps it: a
/// zone re-tints what it spawned rather than looking it back up through a
/// query on an entity it already has in hand.
pub(super) fn spawn_table_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    slot: &SeatSlot,
    quad: TableQuad,
) -> (Entity, Handle<StandardMaterial>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(quad.tint),
        base_color_texture: Some(quad.texture),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        depth_bias: sort_bias(quad.lift),
        ..default()
    });
    let entity = commands
        .spawn((
            DuelStage,
            Mesh3d(meshes.add(Rectangle::new(quad.size.x, quad.size.y))),
            MeshMaterial3d(material.clone()),
            // Ground and glow are both scenery. Only cards are
            // pointed at, so only cards are pickable.
            Pickable::IGNORE,
            lying_flat(slot, quad.lift),
        ))
        .id();
    (entity, material)
}

/// Where a flat thing lying on a seat's ground stands: on the seat's centre,
/// turned to face it, `lift` above the table.
///
/// One function because the mat and the glow are drawn by two different
/// materials now and would otherwise write the same transform twice — and a
/// mat and the pool of light under it that disagreed by a rotation would be
/// very hard to see and impossible to miss.
pub(super) fn lying_flat(slot: &SeatSlot, lift: f32) -> Transform {
    Transform {
        translation: to_world(slot.center, TABLE_Y + lift),
        rotation: Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        // A quad cut to the pod's own size, drawn at its scale.
        scale: Vec3::splat(slot.scale),
    }
}

/// How wide the texture of a pile's empty place is drawn.
///
/// The height follows from the card's aspect, so the lip's corners stay round
/// rather than becoming ellipses.
const RECESS_PX: u32 = 128;

/// How far below a real card a pile's empty place lies.
const RECESS_LIFT: f32 = -CARD_LIFT * 0.5;

/// One seat's four pile places, and the face-down library standing on one.
///
/// Drawn here rather than as placements because neither has an object behind
/// it: a library is face down to everybody, its owner included (CR 401.2),
/// and an empty pile has no card in it at all. What both of them are is a
/// *place* — and a place that appeared and vanished as cards moved through it
/// would make the table rearrange itself in the middle of a game.
pub(super) fn spawn_piles(
    commands: &mut Commands,
    index: &SceneIndex,
    slot: &SeatSlot,
    piles: &[baylee_client_core::ZonePile],
    library: u32,
) -> Vec<Entity> {
    let Some(quad) = index.quad.clone() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for pile in piles {
        // A kind with no well is a kind added to the model and not to the
        // startup that builds their materials; skipping it draws no place
        // rather than drawing the wrong one.
        let Some(recess) = well_of(index, pile.kind) else {
            continue;
        };
        let at = slot.pile_center(pile.kind);
        out.push(
            commands
                .spawn((
                    DuelStage,
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(recess),
                    card_transform(slot, at, false, RECESS_LIFT),
                    // A click near a pile's empty place means the table, the
                    // same as a click near the edge of a card does.
                    Pickable::IGNORE,
                ))
                .id(),
        );
    }

    // The library, as the deck it is: face down, and as tall as it has cards
    // in it up to [`MAX_STACK_RISE`]. The exact count is on the seat's tab —
    // a physical library does not tell you how many cards are in it either —
    // but its *thickness* is the one thing a real one does say across a
    // table, and a seat playing down to nothing should be able to watch it
    // happen.
    if library > 0
        && piles
            .iter()
            .any(|p| p.kind == baylee_client_core::PileKind::Library)
    {
        let at = slot.pile_center(baylee_client_core::PileKind::Library);
        let under = library as usize;
        let layers = stack_layers(under).max(1);
        let rise = stack_rise(under);
        for i in 0..layers {
            let lift = rise * i as f32 / layers as f32;
            let mut slab = commands.spawn((
                DuelStage,
                Mesh3d(quad.clone()),
                MeshMaterial3d(index.blank.clone().unwrap_or_default()),
                card_transform(slot, at, false, lift),
            ));
            // The deck under the top slab is depth, not cards, so it is as
            // unpickable as the backing under a graveyard's top card. The
            // slab on top is the one thing here a pointer can mean, and it
            // has to be findable: a library is the one pile with no top card
            // to hover, and the fan it opens is drawn out of *this*.
            if i + 1 == layers {
                slab.insert(PileVisual {
                    player: slot.player,
                    kind: baylee_client_core::PileKind::Library,
                });
            } else {
                slab.insert(Pickable::IGNORE);
            }
            out.push(slab.id());
        }
    }
    out
}

/// What [`despawn_stage`] takes down: everything spawned for the duel, and
/// every card, which carries both markers.
type StageOrCard = Or<(With<DuelStage>, With<CardVisual>)>;

/// Tears the stage down.
///
/// Every entity is despawned **once**. A card wears both [`DuelStage`] and
/// [`CardVisual`], and this used to walk the two lists one after the other,
/// so every card on the table was despawned twice and the second command
/// found nothing: one "Entity despawned … is invalid" warning per card, 46
/// of them in one millisecond at the owner's table on leaving it (#321).
/// One query over either marker yields each entity once, and
/// [`despawn_tops`] leaves out anything a despawned ancestor already takes
/// with it, which is the same fault one level down.
pub fn despawn_stage(
    mut commands: Commands,
    stage: Query<Entity, StageOrCard>,
    parents: Query<&ChildOf>,
    mut index: ResMut<SceneIndex>,
    mut watch: ResMut<ZoneWatch>,
) {
    despawn_tops(&mut commands, stage.iter(), &parents);
    index.cards.clear();
    index.materials.clear();
    index.face_materials.clear();
    index.faces.clear();
    index.marks.clear();
    index.marks_materials.clear();
    index.badges.clear();
    index.badge_materials.clear();
    index.plates.clear();
    index.plate_materials.clear();
    index.floors.clear();
    index.floor_materials.clear();
    index.shells.clear();
    index.shell_materials.clear();
    index.stacks.clear();
    watch.clear();
    // The zones were spawned with `DuelStage`, so they have just gone with
    // it; what is left is the bookkeeping that would otherwise point at
    // entities that no longer exist.
    index.zones.clear();
}

/// Despawns each of `doomed` once, and none whose ancestor is among them.
///
/// A despawn takes the entity's descendants with it, so a second command for
/// a descendant — or for the same entity, named twice — lands on an entity
/// that is already gone and bevy warns about it. Which ones a caller's
/// queries overlap on is a fact about every spawner in the client rather
/// than about the caller, so this asks the hierarchy instead of trusting it.
pub(crate) fn despawn_tops(
    commands: &mut Commands,
    doomed: impl IntoIterator<Item = Entity>,
    parents: &Query<&ChildOf>,
) {
    let doomed: std::collections::BTreeSet<Entity> = doomed.into_iter().collect();
    for &entity in &doomed {
        let taken_by_an_ancestor = parents
            .iter_ancestors(entity)
            .any(|ancestor| doomed.contains(&ancestor));
        if !taken_by_an_ancestor {
            commands.entity(entity).despawn();
        }
    }
}
