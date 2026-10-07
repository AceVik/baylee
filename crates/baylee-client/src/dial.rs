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

/// The dial's state, on the slab, so a new table starts settled.
#[derive(Component, Default)]
pub struct DialFace(Dial);

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
    /// The dial's drawn diameter, logical pixels.
    pub dial_px: f32,
    /// The turn number's drawn size.
    pub number_px: f32,
    /// The turn number's drawn width.
    pub number_w: f32,
    /// How many times the uniforms were written: a settled dial adds none.
    pub uploads: u64,
}

/// The dial's drawn diameter through `lens`: the compass's radius along the
/// table's x axis, projected, twice (DESIGN-v7 §3.4).
#[must_use]
pub fn dial_px(lens: &crate::table::Lens) -> Option<f32> {
    let middle = lens.project(Vec2::ZERO)?;
    let edge = lens.project(Vec2::X * dial::COMPASS_R)?;
    Some(middle.distance(edge) * 2.0)
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
    (now, still): (f32, bool),
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
    let across = lens.and_then(dial_px).unwrap_or(0.0);
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
        number_px: number,
        number_w: dial::number_width(view.turn, number),
        uploads,
    }
}

/// Reads the table, moves the hands, and writes the felt's dial uniforms when
/// they changed.
#[allow(clippy::too_many_arguments)] // one system, the dial's inputs
pub(crate) fn turn_the_dial(
    duel: Res<crate::Duel>,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    shown: Res<crate::table::ShownRig>,
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
    let facts = DialFacts {
        jewels: dial::bearings(&seats),
        active: Some(view.active),
        awaiting: view.awaiting,
        deciding: deciding.clone(),
        me: Some(view.seat),
        over,
    };
    let still = prefs.all().reduce_motion;
    let now = time.elapsed_secs_wrapped();
    face.0.advance(&facts, now, time.delta_secs(), still);
    let hands = &face.0;

    let (jewels, tints, teams) = jewel_uniforms(&facts, view, layout, &team);
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
    #[allow(clippy::cast_precision_loss)] // eight seats at most
    let seat_count = facts.jewels.len().min(8) as f32;
    let want = (
        Vec4::new(turn.x, turn.y, prio.x, prio.y),
        Vec4::new(
            prio_len,
            if view.awaiting == Some(view.seat) {
                1.0
            } else {
                0.0
            },
            arrived(hands.turn_arrived_at),
            arrived(hands.priority_arrived_at),
        ),
        Vec4::new(pulse_at, pulse_kind, 0.0, 0.0),
        jewels,
        tints,
        teams,
        seat_count,
    );
    let have = materials.get(&handle.0).map(|m| {
        let p = &m.params;
        (
            p.hands, p.dial, p.pulse, p.jewels, p.tints, p.teams, p.seats,
        )
    });
    if have.is_some_and(|have| have != want)
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        let p = &mut material.params;
        (
            p.hands, p.dial, p.pulse, p.jewels, p.tints, p.teams, p.seats,
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
        (now, still),
        facts.deciding,
        report.uploads,
    );
    if report.turn_hand != next.turn_hand
        || report.priority_hand != next.priority_hand
        || report.deciding != next.deciding
        || (report.hub_pulse - next.hub_pulse).abs() > 1e-4
        || (report.dial_px - next.dial_px).abs() > 1e-3
        || (report.number_w - next.number_w).abs() > 1e-3
    {
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
}
