//! The arrows between the things that are fighting.
//!
//! `baylee_client_core::combat` decides *which* arrows exist and which way
//! each one points; this draws them. Four things about the drawing are
//! decisions rather than details:
//!
//! **A mesh, not a gizmo.** Bevy's gizmos would be the obvious tool and are
//! not available: the workspace takes bevy with `default-features = false`
//! and `bevy_gizmos` is not in the list. Everything else on this table is an
//! unlit quad, so an arrow is one too — a unit `Rectangle` laid flat, turned
//! to face along the chord and stretched to cover the curve.
//!
//! **A curve with a head, and both are load-bearing.** These were straight
//! stretched rectangles, and a straight line has two ends and no direction:
//! which end was the attacker was something the player had to know already.
//! Worse, everything attacking one seat ends at the same point, so the lines
//! lay on top of each other near it. A bow separates them — two arcs leave a
//! shared end at different angles — and a head says which way to read one.
//! The arithmetic is `shaders/arrow.wgsl`; this file decides where the ends
//! are and hands over lengths in table units.
//!
//! **Recomputed from where the cards *are*, not from where they are going.**
//! Every card on this table moves through `Motion` and `glide`, so an arrow
//! built from `Motion::target` would snap to the card's destination while the
//! card was still travelling, and a player declaring an attack would watch the
//! arrow arrive before the attacker did. Reading the live `Transform` costs one
//! query and keeps the arrow welded to both ends throughout the glide.
//!
//! **Drawn between the felt and the cards.** `LINE_Y` sits above the
//! medallion and below `CARD_LIFT`, so an arrow never z-fights the table and
//! never crosses a card it passes under.

use crate::Duel;
use crate::arrowmat::{ArrowMaterial, ArrowParams};
use crate::hud::palette;
use crate::table::{CardVisual, DuelStage, to_world};
use baylee_client_core::combat::{Combat, Line, LineEnd, LineKind};
use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH, SeatSlot};
use bevy::prelude::*;

/// How high above the table top a line lies.
///
/// Between the medallion (0.0015) and `CARD_LIFT` (0.01).
const LINE_Y: f32 = 0.005;

/// Half the width of an arrow's shaft, in table units — a fortieth of a card.
const SHAFT: f32 = CARD_WIDTH * 0.032;

/// Half the width of the head where it is widest.
///
/// Four times the shaft. A head only reads as a head if it is plainly wider
/// than the thing it is on the end of, and at this camera the shaft is about
/// three physical pixels.
const HEAD_WIDTH: f32 = SHAFT * 4.0;

/// How much of the curve the head takes.
///
/// A fraction rather than a length, so a long arrow across the table and a
/// short one between neighbours both look like arrows. The cost is that a
/// very long arrow gets a very long head; `bow` caps the curvature for the
/// same reason and by the same argument.
const HEAD_FRAC: f32 = 0.13;

/// How far an arrow bows off its own chord, as a fraction of that chord.
const BOW: f32 = 0.09;

/// The most it may bow, in table units.
///
/// A cap, because the fraction is what separates two arrows sharing an end
/// and a whole-table arc would sail over everybody's board on the way. One
/// card wide is enough to tell two arcs apart and small enough to stay in the
/// gap between two mats.
const BOW_CAP: f32 = CARD_WIDTH;

/// How far short of a card an arrow stops, when it is pointing at one.
///
/// A little over half a card's height, so the head sits outside the card at
/// any approach angle. Without it an arrow ends at the card's *centre* — and
/// the arrows are drawn under the cards on purpose, between the medallion and
/// `CARD_LIFT`, so the head of a block arrow was being drawn underneath the
/// blocker it was pointing at and could not be seen at all. The tail is left
/// alone: an arrow coming out from under the card that owns it reads as
/// leaving that card, which is what it is doing.
///
/// A seat's end needs none of this. [`seat_anchor`] is already the near edge
/// of that seat's mat, and nothing stands on it.
const CLEAR: f32 = CARD_HEIGHT * 0.52;

/// The most of a short arrow the clearance may eat, at each end.
///
/// A blocker standing right in front of the attacker it blocks is two cards
/// apart, which is less than twice the clearance, and an arrow that had its
/// whole length taken off it would leave a head lying on the felt pointing at
/// nothing.
const CLEAR_SHARE: f32 = 0.3;

/// How many dashes the current is cut into over a whole arrow.
const DASHES: f32 = 7.0;

/// How many of them pass a fixed point each second.
///
/// The table's own tempo, and the same number `BEAT` is — one dash per beat.
/// `docs/design.md` §1.6 is why it is not a rate of its own: the current is
/// on the felt beside the keyword marks and the focus ring, and a third clock
/// among them is the fairground that section exists to prevent.
const CURRENT: f32 = BEAT;

/// Opacity of a declaration the engine has accepted.
const STANDING: f32 = 0.85;

/// Opacity of one this seat is still building.
///
/// The difference is the whole point: a player has to be able to tell what
/// they have committed from what they are still choosing, and there is no
/// undo once it is sent.
const PROPOSED: f32 = 0.40;

/// One drawn arrow, the declaration it stands for, and the uniform it was
/// last written with.
///
/// The uniform is kept so that a frame which changes nothing writes nothing:
/// the current runs off `globals.time` inside the shader, so a standing arrow
/// between two cards that are not moving needs no upload at all, and the
/// comparison is what turns that into a fact rather than a hope.
#[derive(Component)]
pub struct CombatLine {
    line: Line,
    params: ArrowParams,
}

/// The one quad every arrow is drawn on.
///
/// There is no material here, unlike every other cached-asset resource in
/// this client: an arrow's geometry *is* its uniform, so two arrows are never
/// the same material and there is nothing to share.
#[derive(Resource, Default)]
pub struct LineAssets {
    quad: Option<Handle<Mesh>>,
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

/// Aim at the displayed life value, sharing the identity's projection.
/// The ledge fallback is used only before a camera/window exists.
pub(crate) fn player_end(slot: &SeatSlot, lens: Option<&crate::table::Lens>) -> Vec3 {
    lens.and_then(|lens| {
        crate::hud::seatbar::attached::life_anchor(slot, lens)
            .and_then(|point| lens.on_plane(point, LINE_Y))
    })
    .unwrap_or_else(|| to_world(seat_anchor(slot), LINE_Y))
}

fn seat_anchor(slot: &SeatSlot) -> Vec2 {
    let [a, b, _, d] = slot.ledge_corners();
    a.midpoint(d).lerp(b, 0.12)
}

/// Brings the drawn arrows in line with the combat the model reports.
#[expect(
    clippy::too_many_arguments,
    reason = "the quad, the two asset tables, the preference and the two disjoint queries"
)]
pub fn sync_combat_lines(
    mut commands: Commands,
    duel: Res<Duel>,
    shown: Option<Res<crate::table::ShownRig>>,
    windows: Query<&Window>,
    prefs: Res<crate::prefs::Prefs>,
    mut assets: ResMut<LineAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrowMaterial>>,
    cards: Query<(&CardVisual, &Transform), Without<CombatLine>>,
    mut drawn: Query<
        (
            Entity,
            &mut CombatLine,
            &mut Transform,
            &MeshMaterial3d<ArrowMaterial>,
        ),
        With<CombatLine>,
    >,
) {
    let lens = shown
        .as_ref()
        .and_then(|s| s.rig())
        .zip(windows.single().ok())
        .map(|(rig, w)| crate::table::Lens::new(rig, Vec2::new(w.width(), w.height())));
    let wanted = wanted_lines(&duel, &cards, lens.as_ref());
    let motion = crate::cardmat::motion_of(prefs.all().reduce_motion);

    // Reused in a stable order. Query iteration follows archetype order,
    // which is stable within a frame but says nothing across frames, so the
    // pool is sorted — otherwise an arrow could swap ends with another one
    // between frames and slide across the table for no reason.
    let mut pool: Vec<Entity> = drawn.iter().map(|(e, ..)| e).collect();
    pool.sort_unstable();

    let quad = assets
        .quad
        .get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0)))
        .clone();

    for (index, (line, from, to)) in wanted.iter().enumerate() {
        let (transform, params) = arrow(*line, *from, *to, motion);
        if let Some(entity) = pool.get(index).copied() {
            let Ok((_, mut existing, mut at, mat)) = drawn.get_mut(entity) else {
                continue;
            };
            *at = transform;
            existing.line = *line;
            // Only when it moved. `get_mut` on an asset marks it changed
            // whatever is written, so the comparison has to happen out here
            // or every arrow re-uploads its uniform sixty times a second for
            // as long as combat lasts.
            if existing.params != params
                && let Some(mut material) = materials.get_mut(&mat.0)
            {
                existing.params = params;
                material.params = params;
            }
        } else {
            commands.spawn((
                DuelStage,
                CombatLine {
                    line: *line,
                    params,
                },
                Mesh3d(quad.clone()),
                MeshMaterial3d(materials.add(ArrowMaterial { params })),
                transform,
            ));
        }
    }

    for entity in pool.into_iter().skip(wanted.len()) {
        commands.entity(entity).despawn();
    }
}

/// Every arrow that can be drawn right now, tail first and head second.
///
/// The two ends come from [`Line::points_from`] and [`Line::points_at`] and
/// not from `from`/`to`, because a block's arrow runs the other way: the
/// declaration's near end is the blocker and the *arrow* leaves the attacker
/// and lands on it. That decision is in the model, where a test reaches it.
///
/// An arrow whose ends are not both on the table is dropped rather than
/// guessed at: a planeswalker in a zone this seat cannot see has no position,
/// and an arrow to nowhere is worse than no arrow.
fn wanted_lines(
    duel: &Duel,
    cards: &Query<(&CardVisual, &Transform), Without<CombatLine>>,
    lens: Option<&crate::table::Lens>,
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
        LineEnd::Seat(player) => layout.slot(player).map(|slot| player_end(slot, lens)),
    };

    combat
        .lines
        .iter()
        .filter_map(|line| {
            let tail = at_end(line.points_from())?;
            let head = at_end(line.points_at())?;
            Some((*line, tail, head))
        })
        .collect()
}

/// How far this arrow bows off its chord.
///
/// A fraction of the chord up to a cap. Two arrows that share an end need to
/// leave it at different angles or they lie on top of each other, and a
/// fraction is what gives a short arrow any curvature at all; the cap is what
/// stops a long one from arcing over somebody's board on the way.
fn bow(chord: f32) -> f32 {
    (chord * BOW).min(BOW_CAP)
}

/// The quad an arrow is drawn on, and the uniform that draws it.
///
/// The quad is laid flat first and then turned about `y`, the same order
/// `card_transform` uses, so "flat" means the same thing for both. Its local
/// `+x` is the chord after the lay-flat rotation, which is why the yaw solves
/// `R_y(θ)·x̂ = d̂`.
///
/// It is sized to the whole *curve* and not to the chord: the box is longer
/// than the chord by the head's own half-width at each end and deep enough for
/// the bow plus that head, because a distance field clipped by its own quad
/// loses exactly the part that was furthest from the straight line — which is
/// the part the bow exists for.
fn arrow(line: Line, tail: Vec3, head: Vec3, motion: f32) -> (Transform, ArrowParams) {
    let reach = head - tail;
    let span = reach.length();
    // Stop short of the card at the pointed-at end, so the head is not drawn
    // underneath it. Guarded against a zero span: two cards at the same place
    // are a frame of a glide, not a geometry error.
    let head = if line.points_at().object().is_some() && span > f32::EPSILON {
        head - reach / span * CLEAR.min(span * CLEAR_SHARE)
    } else {
        head
    };

    let delta = head - tail;
    let chord = delta.length();
    let yaw = (-delta.z).atan2(delta.x);
    let bulge = bow(chord);
    let margin = HEAD_WIDTH + SHAFT;
    let box_size = Vec2::new(chord + margin * 2.0, (bulge + margin) * 2.0);
    let colour = tint(line.kind, line.standing).to_linear();
    (
        Transform {
            translation: tail.lerp(head, 0.5),
            rotation: Quat::from_rotation_y(yaw)
                * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(box_size.x.max(f32::EPSILON), box_size.y, 1.0),
        },
        ArrowParams {
            color: Vec4::new(colour.red, colour.green, colour.blue, colour.alpha),
            box_size,
            chord,
            bulge,
            width: SHAFT,
            head_width: HEAD_WIDTH,
            head_frac: HEAD_FRAC,
            dashes: DASHES,
            speed: CURRENT,
            motion,
        },
    )
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
    shown: Option<Res<crate::table::ShownRig>>,
    windows: Query<&Window>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut assets: ResMut<FocusAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cards: Query<(&CardVisual, &Transform), Without<FocusRing>>,
    mut ring: Query<(Entity, &mut Transform), With<FocusRing>>,
) {
    let lens = shown
        .as_ref()
        .and_then(|s| s.rig())
        .zip(windows.single().ok())
        .map(|(rig, w)| crate::table::Lens::new(rig, Vec2::new(w.width(), w.height())));
    let at = focus_position(&duel, &cards, lens.as_ref());
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
    lens: Option<&crate::table::Lens>,
) -> Option<Vec3> {
    let view = duel.view.as_ref()?;
    let layout = duel.layout.as_ref()?;
    let focus = Combat::read(view, duel.interaction.as_ref()).focus?;
    match focus {
        LineEnd::Object(id) => cards
            .iter()
            .find(|(card, _)| card.object == id)
            .map(|(_, at)| Vec3::new(at.translation.x, LINE_Y, at.translation.z)),
        LineEnd::Seat(player) => layout.slot(player).map(|slot| player_end(slot, lens)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_arrows_project_to_the_life_value_on_both_sides() {
        use crate::table::{CameraRig, Canvas, Lens};
        use baylee_client_core::layout::TableLayout;
        use baylee_core::ids::PlayerId;
        let layout = TableLayout::new(
            &[PlayerId::new(0), PlayerId::new(1)],
            16.0 / 9.0,
            Some(PlayerId::new(0)),
        );
        let canvas = Canvas::hud(Vec2::new(1728.0, 1052.0));
        let lens = Lens::new(CameraRig::home(&layout, canvas), canvas.window);
        for slot in &layout.slots {
            let expected = crate::hud::seatbar::attached::life_anchor(slot, &lens).unwrap();
            let actual = lens.project_world(player_end(slot, Some(&lens))).unwrap();
            assert!(
                actual.distance(expected) < 0.1,
                "{actual:?} vs {expected:?}"
            );
        }
    }

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

    /// The shader has no clock of its own, and stops when the table does.
    ///
    /// Two rules in one text assertion, because both are invisible from Rust:
    /// `docs/design.md` §1.6 allows this table one tempo, and every animation
    /// here has to land on `reduce_motion`. `globals.time` reaching a fragment
    /// unmultiplied would break the second silently — the arrow would keep
    /// running for a player who asked the table to hold still, and no test
    /// that could not read the WGSL would ever notice.
    #[test]
    fn the_arrow_has_no_clock_of_its_own_and_stops_when_the_table_does() {
        let src = include_str!("shaders/arrow.wgsl");
        let uses: Vec<&str> = src
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| line.contains("globals.time"))
            .collect();
        assert_eq!(
            uses.len(),
            1,
            "one reading of the clock, and it is the one handed in: {uses:?}"
        );
        assert!(
            uses[0].contains("params.motion"),
            "the clock is multiplied by the motion preference: {}",
            uses[0]
        );
        assert!(
            uses[0].contains("params.speed"),
            "and by the rate this file hands it, not one of its own: {}",
            uses[0]
        );
    }

    fn a_line(kind: LineKind) -> Line {
        use baylee_core::ids::{ObjectId, PlayerId};
        Line {
            from: ObjectId::new(1, 0),
            to: LineEnd::Seat(PlayerId::new(1)),
            kind,
            standing: true,
        }
    }

    /// The curve leaves the card it belongs to and reaches the seat it names.
    ///
    /// Geometry gets a geometry test: this client once shipped a card quad
    /// that drew as a bowtie while every transform assertion passed. What
    /// spans the two points is the *curve*, not the quad — the quad is
    /// longer, because a head needs room past the tip — so the ends are
    /// measured where the shader puts them, at ±chord/2 of the box.
    #[test]
    fn an_arrow_reaches_from_one_end_to_the_other() {
        let tail = Vec3::new(-2.0, LINE_Y, 1.0);
        let head = Vec3::new(3.0, LINE_Y, -1.5);
        let (t, params) = arrow(a_line(LineKind::Attack), tail, head, 1.0);

        let end = curve_end(&t, &params);
        assert!(
            end(1.0).distance(head) < 1e-4 && end(-1.0).distance(tail) < 1e-4,
            "an arrow at a seat reaches it exactly, and leaves its own card: \
             {:?}..{:?} for {tail:?}..{head:?}",
            end(-1.0),
            end(1.0)
        );
    }

    /// An arrow pointing at a *card* stops short of it.
    ///
    /// The arrows are drawn under the cards on purpose, so a head that landed
    /// on a card's centre was drawn beneath that card and was not there at
    /// all. This is the one thing about the picture that a person looking at
    /// it could miss and a test cannot.
    #[test]
    fn an_arrow_pointing_at_a_card_stops_in_front_of_it() {
        use baylee_core::ids::ObjectId;
        let mut line = a_line(LineKind::Block);
        line.to = LineEnd::Object(ObjectId::new(7, 0));
        // A block leaves `to` (the attacker) and lands on `from` (the
        // blocker), so both ends of this one are cards.
        let tail = Vec3::new(0.0, LINE_Y, 0.0);
        let head = Vec3::new(0.0, LINE_Y, -6.0);
        let (t, params) = arrow(line, tail, head, 1.0);

        let end = curve_end(&t, &params);
        assert!(
            (end(-1.0).distance(tail)) < 1e-4,
            "it still comes out from under the card it leaves"
        );
        // Measured against the *card*, not against `CLEAR`. Comparing the gap
        // to the constant that produced it is a test that agrees with any
        // value the constant happens to have, including zero — which is the
        // bug. What has to be true is that the head is outside the card, and
        // half a card's width is the shortest distance from a card's middle
        // to its own edge.
        let gap = end(1.0).distance(head);
        assert!(
            gap >= CARD_WIDTH * 0.5,
            "the head stops {gap} from the card's middle, which is on top of it"
        );
    }

    /// Two cards standing next to each other still get a whole arrow.
    #[test]
    fn a_short_arrow_keeps_most_of_its_length() {
        use baylee_core::ids::ObjectId;
        let mut line = a_line(LineKind::Block);
        line.to = LineEnd::Object(ObjectId::new(7, 0));
        let tail = Vec3::new(0.0, LINE_Y, 0.0);
        let head = Vec3::new(0.0, LINE_Y, -1.0);
        let (_, params) = arrow(line, tail, head, 1.0);
        // A literal, for the same reason the gap above is measured against
        // the card: `1.0 - CLEAR_SHARE` would agree with any share at all.
        assert!(
            params.chord >= 0.6,
            "a one-unit arrow kept {} of itself, which is not an arrow any more",
            params.chord
        );
    }

    /// The two ends of the drawn curve, in table space.
    fn curve_end<'a>(t: &'a Transform, params: &'a ArrowParams) -> impl Fn(f32) -> Vec3 + 'a {
        move |sign| {
            t.transform_point(Vec3::new(
                sign * params.chord / (2.0 * params.box_size.x),
                0.0,
                0.0,
            ))
        }
    }

    /// The quad holds the whole curve, head and all.
    ///
    /// A distance field clipped by its own quad loses the part furthest from
    /// the chord — which is exactly the bow, the thing the curve exists for —
    /// and it does it silently: the arrow simply comes out flat-topped.
    #[test]
    fn an_arrow_lies_flat_and_its_box_holds_the_whole_curve() {
        let (t, params) = arrow(
            a_line(LineKind::Block),
            Vec3::new(0.0, LINE_Y, 0.0),
            Vec3::new(0.0, LINE_Y, -4.0),
            1.0,
        );
        let across = t.transform_point(Vec3::new(0.0, 0.5, 0.0))
            - t.transform_point(Vec3::new(0.0, -0.5, 0.0));
        assert!(
            across.y.abs() < 1e-5,
            "the across axis stays in the table plane, not standing up out of it"
        );
        assert!(
            params.box_size.y >= 2.0 * (params.bulge + params.head_width),
            "the box is {} deep for a bow of {} and a head of {}",
            params.box_size.y,
            params.bulge,
            params.head_width
        );
        assert!(
            params.box_size.x >= params.chord + 2.0 * params.head_width,
            "and long enough for the head to finish inside it"
        );
    }

    #[test]
    fn a_seat_is_aimed_at_its_identity_side() {
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
            anchor.distance(slot.ledge_corners()[0]) < anchor.distance(slot.ledge_corners()[1]),
            "the anchor belongs to the identity end of the ledge"
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
                    blocked: true,
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
        .insert_resource(Assets::<crate::arrowmat::ArrowMaterial>::default())
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
        // looks right and the entity count climbs forever. The quad is cached
        // in `LineAssets` and must not grow — and neither may the material
        // table, which is the newer of the two failures: an arrow owns its
        // uniform, so an `add` on the wrong branch leaks one material per
        // arrow per frame while the table goes on looking exactly right.
        let mut app = harness(a_fight());
        app.update();
        let meshes = app.world().resource::<Assets<Mesh>>().len();
        let mats = app.world().resource::<Assets<ArrowMaterial>>().len();

        for _ in 0..5 {
            app.update();
        }

        assert_eq!(lines(&mut app).len(), 2, "still two lines, not twelve");
        assert_eq!(
            app.world().resource::<Assets<Mesh>>().len(),
            meshes,
            "the unit quad is built once and reused"
        );
        assert_eq!(
            app.world().resource::<Assets<ArrowMaterial>>().len(),
            mats,
            "an arrow keeps the material it was given"
        );
    }

    /// A player who has asked the table to hold still gets a still arrow.
    ///
    /// Through the system rather than through `arrow`, because the claim is
    /// that the *preference* reaches the uniform: `arrow` is handed a number
    /// and passing it on proves only that it was passed on. What can break is
    /// the one line that reads `Prefs`, and this is the only test that runs
    /// it.
    #[test]
    fn reduce_motion_reaches_the_current() {
        for (reduce_motion, want) in [
            (false, crate::cardmat::MOVING),
            (true, crate::cardmat::STILL),
        ] {
            let mut app = harness(a_fight());
            app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = reduce_motion;
            app.update();

            let mut q = app.world_mut().query::<&CombatLine>();
            let motions: Vec<f32> = q.iter(app.world()).map(|line| line.params.motion).collect();
            assert_eq!(motions.len(), 2, "both arrows are drawn: {motions:?}");
            assert!(
                motions.iter().all(|m| (m - want).abs() < f32::EPSILON),
                "reduce_motion = {reduce_motion} wants {want}, got {motions:?}"
            );
        }
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
