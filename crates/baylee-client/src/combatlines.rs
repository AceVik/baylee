//! The lines between the things that are fighting.
//!
//! `baylee_client_core::combat` decides *which* lines exist; this draws them.
//! Three things about the drawing are decisions rather than details:
//!
//! **A mesh, not a gizmo.** Bevy's gizmos would be the obvious tool and are
//! not available: the workspace takes bevy with `default-features = false`
//! and `bevy_gizmos` is not in the list. Everything else on this table is an
//! unlit quad, so a line is one too — a unit `Rectangle` laid flat, turned to
//! face along the segment and stretched to its length.
//!
//! **Recomputed from where the cards *are*, not from where they are going.**
//! Every card on this table moves through `Motion` and `glide`, so a line
//! built from `Motion::target` would snap to the card's destination while the
//! card was still travelling, and a player declaring an attack would watch the
//! line arrive before the attacker did. Reading the live `Transform` costs one
//! query and keeps the line welded to both ends throughout the glide.
//!
//! **Drawn between the felt and the cards.** `LINE_Y` sits above the
//! medallion and below `CARD_LIFT`, so a line never z-fights the table and
//! never crosses a card it passes under.

use crate::Duel;
use crate::hud::palette;
use crate::table::{CardVisual, DuelStage, to_world};
use baylee_client_core::combat::{Combat, Line, LineEnd, LineKind};
use baylee_client_core::layout::{CARD_WIDTH, SeatSlot};
use bevy::prelude::*;

/// How high above the table top a line lies.
///
/// Between the medallion (0.0015) and `CARD_LIFT` (0.01).
const LINE_Y: f32 = 0.005;

/// How wide a line is, in table units — a twentieth of a card.
const LINE_WIDTH: f32 = CARD_WIDTH * 0.05;

/// Opacity of a declaration the engine has accepted.
const STANDING: f32 = 0.85;

/// Opacity of one this seat is still building.
///
/// The difference is the whole point: a player has to be able to tell what
/// they have committed from what they are still choosing, and there is no
/// undo once it is sent.
const PROPOSED: f32 = 0.40;

/// One drawn line, and the declaration it stands for.
///
/// The declaration is kept so the material is only swapped when the line
/// actually changes kind or firms up, rather than on every frame.
#[derive(Component)]
pub struct CombatLine {
    line: Line,
}

/// The quad and the four materials every line shares.
#[derive(Resource, Default)]
pub struct LineAssets {
    quad: Option<Handle<Mesh>>,
    /// Indexed by [`shade`] — an array rather than a map, so the lookup has
    /// no ordering to be wrong about.
    materials: [Option<Handle<StandardMaterial>>; 4],
}

/// Which of the four materials a line uses.
const fn shade(kind: LineKind, standing: bool) -> usize {
    match (kind, standing) {
        (LineKind::Attack, true) => 0,
        (LineKind::Attack, false) => 1,
        (LineKind::Block, true) => 2,
        (LineKind::Block, false) => 3,
    }
}

/// The colour a line is drawn in.
///
/// Attacks in `DANGER` and blocks in `ACCENT`, which is the language the HUD
/// already speaks — the same two colours mean the same two things there.
fn tint(kind: LineKind, standing: bool) -> Color {
    let base = match kind {
        LineKind::Attack => palette::DANGER,
        LineKind::Block => palette::ACCENT,
    };
    base.with_alpha(if standing { STANDING } else { PROPOSED })
}

/// Where a line aimed at a seat ends.
///
/// Not the mat's centre, which is underneath that seat's own cards: the
/// near edge, so an attack reaches the front of the defender's play area and
/// stops there. Moving in from the centre by half the mat's depth is the
/// same edge whichever way the seat is turned.
fn seat_anchor(slot: &SeatSlot) -> Vec2 {
    let inward = slot.center.normalize_or_zero();
    slot.center - inward * (slot.mat_depth() / 2.0)
}

/// Brings the drawn lines in line with the combat the model reports.
pub fn sync_combat_lines(
    mut commands: Commands,
    duel: Res<Duel>,
    mut assets: ResMut<LineAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cards: Query<(&CardVisual, &Transform), Without<CombatLine>>,
    mut drawn: Query<
        (
            Entity,
            &mut CombatLine,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<CombatLine>,
    >,
) {
    let wanted = wanted_lines(&duel, &cards);

    // Reused in a stable order. Query iteration follows archetype order,
    // which is stable within a frame but says nothing across frames, so the
    // pool is sorted — otherwise a line could swap ends with another one
    // between frames and slide across the table for no reason.
    let mut pool: Vec<Entity> = drawn.iter().map(|(e, ..)| e).collect();
    pool.sort_unstable();

    let quad = assets
        .quad
        .get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0)))
        .clone();

    for (index, (line, from, to)) in wanted.iter().enumerate() {
        let material = material_for(&mut assets, &mut materials, line.kind, line.standing);
        let transform = span(*from, *to);
        if let Some(entity) = pool.get(index).copied() {
            let Ok((_, mut existing, mut at, mut mat)) = drawn.get_mut(entity) else {
                continue;
            };
            *at = transform;
            if existing.line != *line {
                existing.line = *line;
                *mat = MeshMaterial3d(material);
            }
        } else {
            commands.spawn((
                DuelStage,
                CombatLine { line: *line },
                Mesh3d(quad.clone()),
                MeshMaterial3d(material),
                transform,
            ));
        }
    }

    for entity in pool.into_iter().skip(wanted.len()) {
        commands.entity(entity).despawn();
    }
}

/// Every line that can be drawn right now, with both of its ends resolved.
///
/// A line whose ends are not both on the table is dropped rather than guessed
/// at: a planeswalker in a zone this seat cannot see has no position, and a
/// line to nowhere is worse than no line.
fn wanted_lines(
    duel: &Duel,
    cards: &Query<(&CardVisual, &Transform), Without<CombatLine>>,
) -> Vec<(Line, Vec3, Vec3)> {
    let (Some(view), Some(layout)) = (duel.view.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    let combat = Combat::read(view, duel.interaction.as_ref());
    if combat.lines.is_empty() {
        return Vec::new();
    }

    let at_object = |id| {
        cards
            .iter()
            .find(|(card, _)| card.object == id)
            .map(|(_, transform)| {
                Vec3::new(transform.translation.x, LINE_Y, transform.translation.z)
            })
    };
    let at_end = |end: LineEnd| match end {
        LineEnd::Object(id) => at_object(id),
        LineEnd::Seat(player) => layout
            .slot(player)
            .map(|slot| to_world(seat_anchor(slot), LINE_Y)),
    };

    combat
        .lines
        .iter()
        .filter_map(|line| {
            let from = at_object(line.from)?;
            let to = at_end(line.to)?;
            Some((*line, from, to))
        })
        .collect()
}

/// The transform of a unit quad stretched between two points on the table.
///
/// The quad is laid flat first and then turned about `y`, the same order
/// `card_transform` uses, so "flat" means the same thing for both. Its local
/// `+x` is the length axis after the lay-flat rotation, which is why the yaw
/// solves `R_y(θ)·x̂ = d̂` and the scale goes on `x`.
fn span(from: Vec3, to: Vec3) -> Transform {
    let delta = to - from;
    let length = delta.length();
    let yaw = (-delta.z).atan2(delta.x);
    Transform {
        translation: from.lerp(to, 0.5),
        rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        scale: Vec3::new(length.max(f32::EPSILON), LINE_WIDTH, 1.0),
    }
}

/// The shared material for one kind of line, made on first use.
fn material_for(
    assets: &mut LineAssets,
    materials: &mut Assets<StandardMaterial>,
    kind: LineKind,
    standing: bool,
) -> Handle<StandardMaterial> {
    let index = shade(kind, standing);
    assets.materials[index]
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: tint(kind, standing),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .clone()
}

/// The ring marking what a declaration made right now would be aimed at.
#[derive(Component)]
pub struct FocusRing;

/// The ring's mesh and material, made on first use.
#[derive(Resource, Default)]
pub struct FocusAssets {
    ring: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
}

/// Outer radius of the focus ring, in table units — a little wider than a
/// card, so it reads as *around* the thing rather than *on* it.
const RING_OUTER: f32 = CARD_WIDTH * 0.85;

/// Inner radius; the difference is the ring's thickness.
const RING_INNER: f32 = CARD_WIDTH * 0.74;

/// How far the ring breathes, as a fraction of its size.
const RING_SWELL: f32 = 0.08;

/// The tempo every "is" breath on this table shares.
///
/// The same number as `BEAT` in `shaders/card_common.wgsl`, and a test holds
/// the two together. `docs/design.md` §1.6 is the reason it is not a rate of
/// its own: six unrelated tempos is the fairground that section exists to
/// prevent, and a ring pulsing against the rail marks would be the sixth.
const BEAT: f32 = 1.15;

/// Draws the focus, and pulses it.
///
/// The focus is deliberately not one of the lines: it is *where the next line
/// would go*, which is a different claim and gets a different colour —
/// `ACTIVE`, the gold this HUD already uses for "the thing you are on".
#[expect(
    clippy::too_many_arguments,
    reason = "the assets, the clock, the preference and the two disjoint transform queries"
)]
pub fn sync_focus_ring(
    mut commands: Commands,
    duel: Res<Duel>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut assets: ResMut<FocusAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cards: Query<(&CardVisual, &Transform), Without<FocusRing>>,
    mut ring: Query<(Entity, &mut Transform), With<FocusRing>>,
) {
    let at = focus_position(&duel, &cards);
    let Some(at) = at else {
        for (entity, _) in &ring {
            commands.entity(entity).despawn();
        }
        return;
    };

    // A ring that breathed on a machine set to hold still would be the one
    // animation on the table that ignored the preference.
    let swell = if prefs.all().reduce_motion {
        1.0
    } else {
        let phase = time.elapsed_secs() * BEAT * std::f32::consts::TAU;
        RING_SWELL.mul_add(phase.sin(), 1.0)
    };
    let transform = Transform {
        translation: at,
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        scale: Vec3::splat(swell),
    };

    if let Some((_, mut existing)) = ring.iter_mut().next() {
        *existing = transform;
        return;
    }

    let mesh = assets
        .ring
        .get_or_insert_with(|| meshes.add(Annulus::new(RING_INNER, RING_OUTER)))
        .clone();
    let material = assets
        .material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: palette::ACTIVE.with_alpha(STANDING),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .clone();
    commands.spawn((
        DuelStage,
        FocusRing,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}

/// Where the focus ring belongs, if there is a focus and it can be placed.
fn focus_position(
    duel: &Duel,
    cards: &Query<(&CardVisual, &Transform), Without<FocusRing>>,
) -> Option<Vec3> {
    let view = duel.view.as_ref()?;
    let layout = duel.layout.as_ref()?;
    let focus = Combat::read(view, duel.interaction.as_ref()).focus?;
    match focus {
        LineEnd::Object(id) => cards
            .iter()
            .find(|(card, _)| card.object == id)
            .map(|(_, at)| Vec3::new(at.translation.x, LINE_Y, at.translation.z)),
        LineEnd::Seat(player) => layout
            .slot(player)
            .map(|slot| to_world(seat_anchor(slot), LINE_Y)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_breathes_at_the_shaders_tempo() {
        // One tempo for the whole table (`docs/design.md` §1.6). The shader
        // is a text file nothing but the GPU parses, so a drift between the
        // two is silent — and would show up as a ring pulsing against the
        // rail marks it sits beside.
        let src = include_str!("shaders/card_common.wgsl");
        let theirs = crate::cardmat::tests::wgsl_const(src, "BEAT");
        assert!(
            (BEAT - theirs).abs() < 1e-6,
            "BEAT: {BEAT} here, {theirs} in the shader"
        );
    }

    #[test]
    fn a_line_reaches_from_one_end_to_the_other() {
        // Geometry gets a geometry test: this client once shipped a card quad
        // that drew as a bowtie while every transform assertion passed.
        let from = Vec3::new(-2.0, LINE_Y, 1.0);
        let to = Vec3::new(3.0, LINE_Y, -1.5);
        let t = span(from, to);

        // The quad's own ends, pushed through the transform it was given.
        let head = t.transform_point(Vec3::new(0.5, 0.0, 0.0));
        let tail = t.transform_point(Vec3::new(-0.5, 0.0, 0.0));
        assert!(
            head.distance(to) < 1e-4 && tail.distance(from) < 1e-4,
            "the drawn quad spans exactly the two points it was given: \
             {tail:?}..{head:?} for {from:?}..{to:?}"
        );
    }

    #[test]
    fn a_line_lies_flat_and_keeps_its_width() {
        let t = span(Vec3::new(0.0, LINE_Y, 0.0), Vec3::new(0.0, LINE_Y, -4.0));
        let across = t.transform_point(Vec3::new(0.0, 0.5, 0.0))
            - t.transform_point(Vec3::new(0.0, -0.5, 0.0));
        assert!(
            across.y.abs() < 1e-5,
            "the width axis stays in the table plane, not standing up out of it"
        );
        assert!((across.length() - LINE_WIDTH).abs() < 1e-5);
    }

    #[test]
    fn a_seat_is_aimed_at_from_the_near_edge_of_its_mat() {
        use baylee_client_core::layout::TableLayout;
        use baylee_core::ids::PlayerId;

        let layout = TableLayout::new(
            &[PlayerId::new(0), PlayerId::new(1)],
            16.0 / 9.0,
            Some(PlayerId::new(0)),
        );
        let slot = layout.slot(PlayerId::new(1)).expect("the opposing seat");
        let anchor = seat_anchor(slot);
        assert!(
            anchor.length() < slot.center.length(),
            "the anchor is nearer the middle of the table than the mat's centre"
        );
    }
}

/// The two systems, actually run.
///
/// The tests above are about arithmetic and geometry, which is the half of
/// this file that can be wrong quietly. The other half is whether the systems
/// do anything at all, and this client has shipped the answer "no" before:
/// `Interaction::activate` was written and nothing ever called it. So every
/// assertion here is on an *outcome* — entities that exist, a transform that
/// moved, a ring at a named point — and never on `update()` having returned.
///
/// Measured rather than assumed: deleting `Prefs` from the harness fails six
/// of these. Bevy 0.19 is loud about a missing resource — the default error
/// handler panics with "Parameter … failed validation: Resource does not
/// exist" — but it panics on whichever task-pool thread ran the system, so
/// half of those six reported their own assertion failing instead. Either
/// way it is the outcome assertions that catch it.
#[cfg(test)]
mod running {
    use super::*;
    use crate::prefs::Prefs;
    use baylee_client_core::layout::TableLayout;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::{Defender, ObjectId, PlayerId};
    use baylee_view::{AttackerView, BlockerView, PlayerView};

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// Seat 1 attacks seat 0 with a Bear; seat 0 blocks it with an Ogre.
    ///
    /// Both come out of `view.combat`, so both are standing — the case a
    /// client cannot reach through its own `Interaction` at all, because the
    /// declaring seat is not this one.
    fn a_fight() -> PlayerView {
        ViewBuilder::new(2)
            .with_battlefield(1, vec![token(1, 1, "Bear", 2, 2)])
            .with_battlefield(0, vec![token(2, 0, "Ogre", 3, 3)])
            .with_combat(
                vec![AttackerView {
                    creature: obj(1),
                    defending: Defender::Player(PlayerId::new(0)),
                }],
                vec![BlockerView {
                    blocker: obj(2),
                    attacker: obj(1),
                }],
            )
            .build()
    }

    /// An app with both systems, the resources they ask for, and the two
    /// creatures standing on the table where the glide left them.
    fn harness(view: PlayerView) -> App {
        let mut app = App::new();
        app.insert_resource(Duel {
            view: Some(view),
            layout: Some(TableLayout::new(
                &[PlayerId::new(0), PlayerId::new(1)],
                16.0 / 9.0,
                None,
            )),
            ..Duel::default()
        })
        .init_resource::<LineAssets>()
        .init_resource::<FocusAssets>()
        .init_resource::<Time>()
        .init_resource::<Prefs>()
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_systems(Update, (sync_combat_lines, sync_focus_ring));

        for (index, slot) in [1_u32, 2].into_iter().enumerate() {
            let x = index as f32 * 2.0;
            app.world_mut().spawn((
                CardVisual {
                    object: obj(slot),
                    count: 1,
                },
                Transform::from_xyz(x, LINE_Y, 0.0),
            ));
        }
        app
    }

    fn lines(app: &mut App) -> Vec<(Line, Transform)> {
        let mut q = app.world_mut().query::<(&CombatLine, &Transform)>();
        let mut found: Vec<_> = q
            .iter(app.world())
            .map(|(line, at)| (line.line, *at))
            .collect();
        found.sort_by_key(|(line, _)| (line.from, line.kind == LineKind::Block));
        found
    }

    #[test]
    fn both_declarations_reach_the_table() {
        let mut app = harness(a_fight());
        app.update();

        let drawn = lines(&mut app);
        assert_eq!(drawn.len(), 2, "one attack and one block, drawn: {drawn:?}");
        assert!(
            drawn.iter().all(|(line, _)| line.standing),
            "the engine has accepted both, so neither is a proposal"
        );
        assert!(
            drawn.iter().any(|(line, _)| line.kind == LineKind::Attack)
                && drawn.iter().any(|(line, _)| line.kind == LineKind::Block),
            "one of each kind: {drawn:?}"
        );
    }

    #[test]
    fn a_second_frame_reuses_what_the_first_one_built() {
        // The failure this is aimed at is a line spawned per frame: the table
        // looks right and the entity count climbs forever. The mesh and the
        // materials are cached in `LineAssets`, so those must not grow either.
        let mut app = harness(a_fight());
        app.update();
        let after_one = app.world().resource::<Assets<Mesh>>().len();

        for _ in 0..5 {
            app.update();
        }

        assert_eq!(lines(&mut app).len(), 2, "still two lines, not twelve");
        assert_eq!(
            app.world().resource::<Assets<Mesh>>().len(),
            after_one,
            "the unit quad is built once and reused"
        );
    }

    #[test]
    fn a_line_follows_the_card_it_is_welded_to() {
        // The whole reason the lines are recomputed from live `Transform`s
        // rather than from `Motion::target`: mid-glide, the line has to be
        // where the card is, not where it is going.
        let mut app = harness(a_fight());
        app.update();
        let before = lines(&mut app)[0].1.translation;

        let mut cards = app
            .world_mut()
            .query_filtered::<&mut Transform, With<CardVisual>>();
        let mut moved = false;
        for mut at in cards.iter_mut(app.world_mut()) {
            at.translation.x += 5.0;
            moved = true;
        }
        assert!(moved, "the harness put creatures on the table");
        app.update();

        let after = lines(&mut app)[0].1.translation;
        assert!(
            before.distance(after) > 1.0,
            "the line moved with its ends: {before:?} -> {after:?}"
        );
    }

    #[test]
    fn combat_ending_takes_the_lines_with_it() {
        let mut app = harness(a_fight());
        app.update();
        assert_eq!(lines(&mut app).len(), 2);

        app.world_mut()
            .resource_mut::<Duel>()
            .view
            .as_mut()
            .expect("the harness put a view in")
            .combat = baylee_view::CombatView::default();
        app.update();

        assert!(
            lines(&mut app).is_empty(),
            "nothing is fighting, so nothing is drawn"
        );
    }

    /// The focus ring is the second system, and it is the one that draws
    /// nothing when there is nothing to point at — which is exactly how a
    /// skipped system looks. So it is given something to point at.
    #[test]
    fn the_ring_is_drawn_where_the_next_line_would_go() {
        use baylee_client_core::interaction::Interaction;
        use baylee_engine::choice::Pending;

        let mut app = harness(a_fight());
        app.world_mut().resource_mut::<Duel>().interaction = Some(Interaction::new(
            Pending::ChooseAttackers {
                player: PlayerId::new(0),
                attackers: vec![obj(2)],
                defenders: vec![Defender::Player(PlayerId::new(1))],
            },
            PlayerId::new(0),
        ));
        app.update();

        let mut rings = app
            .world_mut()
            .query_filtered::<&Transform, With<FocusRing>>();
        let at: Vec<_> = rings.iter(app.world()).map(|t| t.translation).collect();
        assert_eq!(at.len(), 1, "one ring, on the seat being aimed at");

        let layout = TableLayout::new(&[PlayerId::new(0), PlayerId::new(1)], 16.0 / 9.0, None);
        let slot = layout.slot(PlayerId::new(1)).expect("the opposing seat");
        let want = to_world(seat_anchor(slot), LINE_Y);
        assert!(
            at[0].distance(want) < 1e-4,
            "the ring sits at the seat's near edge: {:?} vs {want:?}",
            at[0]
        );
    }

    /// A quarter of the way through a beat the swell is at its peak, which is
    /// a number this test can name exactly rather than a wobble it has to
    /// sample. Holding still is the same claim with the preference set.
    #[test]
    fn the_ring_swells_on_the_beat_unless_asked_to_hold_still() {
        use baylee_client_core::interaction::Interaction;
        use baylee_engine::choice::Pending;

        let aiming = || {
            Some(Interaction::new(
                Pending::ChooseAttackers {
                    player: PlayerId::new(0),
                    attackers: vec![obj(2)],
                    defenders: vec![Defender::Player(PlayerId::new(1))],
                },
                PlayerId::new(0),
            ))
        };
        let quarter_beat = std::time::Duration::from_secs_f32(1.0 / (4.0 * BEAT));

        for (reduce_motion, want) in [(false, 1.0 + RING_SWELL), (true, 1.0)] {
            let mut app = harness(a_fight());
            app.world_mut().resource_mut::<Duel>().interaction = aiming();
            app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = reduce_motion;
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(quarter_beat);
            app.update();

            let mut rings = app
                .world_mut()
                .query_filtered::<&Transform, With<FocusRing>>();
            let scale = rings
                .iter(app.world())
                .next()
                .expect("the ring is drawn")
                .scale
                .x;
            assert!(
                (scale - want).abs() < 1e-3,
                "reduce_motion = {reduce_motion}: swelled to {scale}, not {want}"
            );
        }
    }
}
