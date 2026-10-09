//! The dial in the middle of the felt (DESIGN-v7 §3), replacing the turning
//! compass: one jewel per seat, a turn hand and a priority hand, and the hub
//! plate the turn number stands on.
//!
//! `baylee_client_core::dial` decides where the hands point and how they get
//! there; this packs that into the felt's uniforms (`feltmat::FeltParams`'s
//! `hands`, `dial`, `pulse`, `jewels`, `tints`, `teams`) and writes them only
//! when a value changed — a settled dial uploads nothing, and the arrival
//! lights fade on the shader's own clock (`globals.time × motion`), so the
//! CPU does no work per frame for them either.

use baylee_client_core::dial::{self, Dial, DialFacts, Pulse};
use baylee_core::ids::PlayerId;
use bevy::prelude::*;

/// The dial's state, on the slab, so a new table starts settled, and the
/// scale its table asks for (`dial::scale_for` of the settled layout,
/// worked out again only when the duel changed).
#[derive(Component, Default)]
pub struct DialFace(Dial, f32);

/// How far back along its sweep a hand's trail reaches, in seconds.
const TRAIL_LAG: f32 = 0.11;

/// What the dial shows, for `/state.dial` (DESIGN-v7 §2.7).
#[derive(Resource, Default, Clone, Debug)]
pub struct DialReport {
    /// The turn hand's vector, table space.
    pub turn_hand: Option<Vec2>,
    /// The priority hand's vector, `None` while retracted or not drawn.
    pub priority_hand: Option<Vec2>,
    /// Seats choosing their opening hands (the arcs).
    pub deciding: Vec<PlayerId>,
    /// The hub's pulse, 0 to 1, at the last frame.
    pub hub_pulse: f32,
    /// The dial's drawn diameter (its compass), logical pixels.
    pub dial_px: f32,
    /// The dial's scale this frame (`dial::scale_for`, eased).
    pub scale: f32,
    /// Where its middle is drawn, logical pixels.
    pub centre: Option<Vec2>,
    /// Its rim's radius in table units (`dial::DIAL_OUTER` × scale): what
    /// the sizing rule keeps off the boards.
    pub radius: f32,
    /// Whether the continuous lights run (the shimmer, the breathing tips,
    /// the spark): off under reduced motion and still ambient effects.
    pub effects: bool,
    /// The turn number's drawn size.
    pub number_px: f32,
    /// The turn number's drawn width.
    pub number_w: f32,
    /// How many times the uniforms were written: a settled dial adds none.
    pub uploads: u64,
}

/// The dial's drawn diameter through `lens` at `scale`: the compass's
/// radius along the table's x axis, projected, twice (DESIGN-v7 §3.4).
#[must_use]
pub fn dial_px(lens: &crate::table::Lens, scale: f32) -> Option<f32> {
    let middle = lens.project(Vec2::ZERO)?;
    let edge = lens.project(Vec2::X * (dial::COMPASS_R * scale))?;
    Some(middle.distance(edge) * 2.0)
}

/// The scale the dial stands at for `duel`'s table once it has settled:
/// what the turn number is sized from.
#[must_use]
pub fn settled_scale(duel: &crate::Duel) -> f32 {
    duel.settled_layout()
        .map_or(dial::MIN_SCALE, dial::scale_for)
}

/// A colour as the shader reads it: display-referred `rgb`.
fn srgb(colour: Color) -> Vec3 {
    let c = colour.to_srgba();
    Vec3::new(c.red, c.green, c.blue)
}

/// The jewels' uniforms: two directions per vector, a colour and a state per
/// seat (`a`: 1 at the table, 0.4 left, plus 2 while choosing an opening
/// hand), and a team's colour round the jewel.
fn jewel_uniforms(
    facts: &DialFacts,
    view: &baylee_view::PlayerView,
    layout: &baylee_client_core::layout::TableLayout,
    team: &impl Fn(PlayerId) -> Option<u8>,
) -> ([Vec4; 4], [Vec4; 8], [Vec4; 8]) {
    let mut jewels = [Vec4::ZERO; 4];
    let mut tints = [Vec4::ZERO; 8];
    let mut teams = [Vec4::ZERO; 8];
    for (i, (player, dir)) in facts.jewels.iter().take(8).enumerate() {
        let pair = &mut jewels[i / 2];
        if i % 2 == 0 {
            (pair.x, pair.y) = (dir.x, dir.y);
        } else {
            (pair.z, pair.w) = (dir.x, dir.y);
        }
        let slot = layout.slot(*player);
        let colour = slot.map_or(Vec3::splat(0.5), |slot| {
            srgb(crate::table::seat_accent(slot))
        });
        let left = view
            .seats
            .iter()
            .find(|s| s.player == *player)
            .is_some_and(baylee_view::SeatView::has_lost);
        let state = if left { 0.4 } else { 1.0 }
            + if facts.deciding.contains(player) {
                2.0
            } else {
                0.0
            };
        tints[i] = colour.extend(state);
        if let Some(side) = team(*player) {
            teams[i] = srgb(crate::hud::team_color(Some(side))).extend(1.0);
        }
    }
    (jewels, tints, teams)
}

/// What `/state.dial` says this frame.
fn report_of(
    hands: &Dial,
    view: &baylee_view::PlayerView,
    lens: Option<&crate::table::Lens>,
    (now, still, effects): (f32, bool, bool),
    deciding: Vec<PlayerId>,
    uploads: u64,
) -> DialReport {
    let turn = hands.turn.direction * hands.turn.length;
    let prio_len = if hands.priority_shown {
        hands.priority.length
    } else {
        0.0
    };
    let prio = hands.priority.direction;
    let across = lens
        .and_then(|lens| dial_px(lens, hands.scale))
        .unwrap_or(0.0);
    let number = dial::number_px(across);
    let pulse = hands.pulse.map_or(0.0, |(_, at)| {
        let age = now - at;
        if still || !(0.0..=dial::ARRIVAL_DECAY).contains(&age) {
            0.0
        } else {
            (age / dial::ARRIVAL_DECAY * std::f32::consts::PI).sin() * 0.8
        }
    });
    DialReport {
        turn_hand: (hands.turn.length > 0.01).then_some(turn),
        priority_hand: (prio_len > 0.01).then_some(prio * prio_len),
        deciding,
        hub_pulse: pulse,
        dial_px: across,
        scale: hands.scale,
        centre: lens.and_then(|lens| lens.project(Vec2::ZERO)),
        radius: dial::DIAL_OUTER * hands.scale,
        effects,
        number_px: number,
        number_w: dial::number_width(view.turn, number),
        uploads,
    }
}

impl DialReport {
    /// Whether `next` says anything this report does not: what decides a
    /// write, so a settled dial leaves the resource untouched.
    fn differs(&self, next: &Self) -> bool {
        self.turn_hand != next.turn_hand
            || self.priority_hand != next.priority_hand
            || self.deciding != next.deciding
            || (self.hub_pulse - next.hub_pulse).abs() > 1e-4
            || (self.dial_px - next.dial_px).abs() > 1e-3
            || (self.scale - next.scale).abs() > 1e-4
            || self.centre != next.centre
            || self.effects != next.effects
            || (self.number_w - next.number_w).abs() > 1e-3
    }
}

/// The hands' uniforms: their vectors (`hands`), the priority hand's length,
/// whether it points at me and both arrival times (`dial`), the hub's pulse
/// and the face's scale (`pulse`), and where each hand pointed a moment ago
/// (`trail`).
fn hand_uniforms(hands: &Dial, at_me: bool) -> (Vec4, Vec4, Vec4, Vec4) {
    let turn = hands.turn.direction * hands.turn.length;
    let prio_len = if hands.priority_shown {
        hands.priority.length
    } else {
        0.0
    };
    let prio = hands.priority.direction;
    let arrived = |at: Option<f32>| at.unwrap_or(-100.0);
    let (pulse_at, pulse_kind) = match hands.pulse {
        Some((Pulse::Turn, at)) => (at, 1.0),
        Some((Pulse::Priority, at)) => (at, 2.0),
        None => (-100.0, 0.0),
    };
    let (t, p) = (hands.turn.trail(TRAIL_LAG), hands.priority.trail(TRAIL_LAG));
    (
        Vec4::new(turn.x, turn.y, prio.x, prio.y),
        Vec4::new(
            prio_len,
            if at_me { 1.0 } else { 0.0 },
            arrived(hands.turn_arrived_at),
            arrived(hands.priority_arrived_at),
        ),
        Vec4::new(pulse_at, pulse_kind, hands.scale, 0.0),
        Vec4::new(t.x, t.y, p.x, p.y),
    )
}

/// Reads the table, moves the hands, and writes the felt's dial uniforms when
/// they changed.
#[allow(clippy::too_many_arguments)] // one system, the dial's inputs
pub(crate) fn turn_the_dial(
    duel: Res<crate::Duel>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    shown: Res<crate::table::ShownRig>,
    quality: Option<Res<crate::quality::InUse>>,
    windows: Query<&Window>,
    mut slabs: Query<(&mut DialFace, &MeshMaterial3d<crate::feltmat::FeltMaterial>)>,
    mut materials: ResMut<Assets<crate::feltmat::FeltMaterial>>,
    mut report: ResMut<DialReport>,
) {
    let (Some(view), Some(layout)) = (duel.view.as_ref(), duel.layout.as_ref()) else {
        return;
    };
    let Ok((mut face, handle)) = slabs.single_mut() else {
        return;
    };
    let statics = duel.statics.as_ref();
    let team =
        |player: PlayerId| statics.and_then(|s| s.seats.iter().find(|i| i.player == player)?.team);
    let seats: Vec<_> = layout
        .slots
        .iter()
        .map(|slot| (slot.player, slot.center, team(slot.player)))
        .collect();
    let over = duel
        .interaction
        .as_ref()
        .is_some_and(|i| matches!(i.pending(), baylee_engine::choice::Pending::GameOver(_)));
    let deciding: Vec<PlayerId> = view.deciding.iter().collect();
    // The table's size, read again only when the duel changed: a settled
    // table asks nothing per frame.
    if duel.is_changed() || face.1 <= 0.0 {
        face.1 = settled_scale(&duel);
    }
    let facts = DialFacts {
        jewels: dial::bearings(&seats),
        active: Some(view.active),
        awaiting: view.awaiting,
        deciding: deciding.clone(),
        me: Some(view.seat),
        over,
        scale: face.1,
    };
    let still = prefs.all().reduce_motion;
    let effects = !crate::quality::ambient_still(still, quality.as_deref());
    let now = time.elapsed_secs_wrapped();
    face.0.advance(&facts, now, time.delta_secs(), still);
    let hands = &face.0;

    let (jewels, tints, teams) = jewel_uniforms(&facts, view, layout, &team);
    let (hand_vectors, dial_times, pulse, trail) =
        hand_uniforms(hands, view.awaiting == Some(view.seat));
    #[allow(clippy::cast_precision_loss)] // eight seats at most
    let seat_count = facts.jewels.len().min(8) as f32;
    let want = (
        hand_vectors,
        dial_times,
        pulse,
        jewels,
        tints,
        teams,
        seat_count,
        trail,
    );
    let have = materials.get(&handle.0).map(|m| {
        let p = &m.params;
        (
            p.hands, p.dial, p.pulse, p.jewels, p.tints, p.teams, p.seats, p.trail,
        )
    });
    if have.is_some_and(|have| have != want)
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        let p = &mut material.params;
        (
            p.hands, p.dial, p.pulse, p.jewels, p.tints, p.teams, p.seats, p.trail,
        ) = want;
        report.uploads += 1;
    }

    // The report, for `/state.dial`.
    let lens = shown.rig().zip(windows.single().ok()).map(|(rig, window)| {
        crate::table::Lens::new(rig, Vec2::new(window.width(), window.height()))
    });
    let next = report_of(
        &face.0,
        view,
        lens.as_ref(),
        (now, still, effects),
        facts.deciding,
        report.uploads,
    );
    if report.differs(&next) {
        *report = next;
    }
}

#[cfg(test)]
mod tests {
    use baylee_client_core::dial;

    /// The shader draws the dial at the model's radii: the compass the hands
    /// point at, the hub plate the number stands on, the stones' band and the
    /// two hands' reach. Read off `felt.wgsl` itself, so a radius edited on
    /// one side only fails here and not on a screenshot.
    #[test]
    fn the_shader_and_the_dial_model_agree() {
        let wgsl = include_str!("shaders/felt.wgsl");
        for (name, value) in [
            ("COMPASS_R", dial::COMPASS_R),
            ("HUB_R", dial::HUB_R),
            ("STONE_R", dial::STONE_R),
            ("TURN_TIP", dial::TURN_TIP),
            ("PRIO_TIP", dial::PRIO_TIP),
            ("BEZEL_R", dial::DIAL_OUTER),
        ] {
            let line = wgsl
                .lines()
                .find(|l| l.starts_with(&format!("const {name}: f32 = ")))
                .unwrap_or_else(|| panic!("{name} is not in felt.wgsl"));
            let said: f32 = line
                .rsplit("= ")
                .next()
                .and_then(|v| v.trim_end_matches(';').parse().ok())
                .expect("a number");
            assert!(
                (said - value).abs() < 1e-6,
                "{name}: the shader says {said}, the model {value}"
            );
        }
    }

    /// The hands' two colours are the model's (`dial::TURN_INK`,
    /// `dial::PRIORITY_INK`), which the seat plates' top border lines are
    /// drawn in too — one ivory and one teal on the whole table — and teal
    /// is the HUD's accent. The HUD takes both from the model's constants;
    /// the shader repeats them, so it is read here.
    #[test]
    fn the_hands_are_drawn_in_the_table_s_turn_and_priority_colours() {
        let wgsl = include_str!("shaders/felt.wgsl");
        for (name, ink) in [("IVORY", dial::TURN_INK), ("TEAL", dial::PRIORITY_INK)] {
            let line = wgsl
                .lines()
                .find(|l| l.starts_with(&format!("const {name}: vec3<f32> = vec3<f32>(")))
                .unwrap_or_else(|| panic!("{name} is not in felt.wgsl"));
            let said: Vec<f32> = line
                .split_once("vec3<f32>(")
                .and_then(|(_, rest)| rest.split_once(')'))
                .map(|(inside, _)| {
                    inside
                        .split(',')
                        .map(|v| v.trim().parse().expect("a number"))
                        .collect()
                })
                .expect("three components");
            assert_eq!(said, ink.to_vec(), "{name}");
        }
        let accent = crate::hud::palette::ACCENT.to_srgba();
        for (a, b) in [accent.red, accent.green, accent.blue]
            .into_iter()
            .zip(dial::PRIORITY_INK)
        {
            assert!((a - b).abs() < 1e-6, "teal is the accent");
        }
    }

    /// Every mark a hand makes is masked by `outside` (the hub plate's rim):
    /// its shadow, outline and body, its trail (zero inside `HUB_R`) and its
    /// arrival flare, so nothing of a hand is drawn under the turn number.
    /// Measured live as well: the plate drawn with and without the hands is
    /// byte-identical under reduced motion, and 1 506 of its 8 281 pixels
    /// change with this mask taken off (`docs/client.md`, the dial).
    #[test]
    fn nothing_of_a_hand_is_drawn_on_the_hub_plate() {
        let wgsl = include_str!("shaders/felt.wgsl");
        let body = |name: &str| {
            let start = wgsl
                .find(&format!("fn {name}("))
                .unwrap_or_else(|| panic!("{name}"));
            let rest = &wgsl[start + 3..];
            &rest[..rest.find("\nfn ").unwrap_or(rest.len())]
        };
        let paint = body("paint_hand");
        for mark in ["let shade =", "let edge =", "let body ="] {
            let line = paint
                .lines()
                .find(|l| l.contains(mark))
                .unwrap_or_else(|| panic!("{mark}"));
            assert!(line.contains("* outside"), "unmasked: {line}");
        }
        assert!(
            body("trail").contains("r < HUB_R"),
            "a trail starts at the plate's rim"
        );
        let face = body("clock_face");
        for line in face.lines().filter(|l| l.contains("flare(")) {
            assert!(line.contains("outside"), "unmasked flare: {line}");
        }
        assert_eq!(
            face.matches("paint_hand(").count(),
            2,
            "both hands go through the one masked painter"
        );
    }

    /// Reduced motion and still ambient effects hold the dial's every
    /// continuous light: in `felt.wgsl` the dial's functions read the clock
    /// only through the still-able `t` (`STILL_AT` when the table holds
    /// still) or behind a `params.motion` gate — so a settled, still table
    /// draws the same frame twice (DESIGN-v8: two screenshots byte-identical),
    /// and the face's moving lights (shimmer, breathing, spark) are off, not
    /// frozen mid-glint.
    #[test]
    fn the_dial_s_lights_stand_still_when_the_table_does() {
        let wgsl = include_str!("shaders/felt.wgsl");
        let start = wgsl.find("fn firewheel(").expect("the stones");
        let face = wgsl.find("fn clock_face(").expect("the face");
        let end = wgsl
            .find("/// The tear's line at")
            .expect("the end of the dial");
        let dial = &wgsl[start..end];
        let mut gated = false;
        for line in dial.lines() {
            if line.contains("params.motion") {
                gated = true;
            }
            if line.starts_with("fn ") {
                gated = false;
            }
            if line.contains("globals.time") {
                assert!(gated, "an ungated clock in the dial: {line}");
            }
        }
        // The face's continuous lights: each multiplied by `live`.
        let face = &wgsl[face..end];
        for light in ["shine", "breath", "spin"] {
            let at = face
                .lines()
                .find(|l| l.contains(&format!("let {light} =")))
                .unwrap_or_else(|| panic!("{light}"));
            let follows = &face[face.find(at).expect("found")..];
            let statement = &follows[..follows.find(';').expect("a statement")];
            assert!(
                statement.contains("live") || light == "spin",
                "{light} is not gated: {statement}"
            );
        }
        assert!(
            dial.contains("if live > 0.5 && off < 0.16"),
            "the spark runs only while the table moves"
        );
    }
}
