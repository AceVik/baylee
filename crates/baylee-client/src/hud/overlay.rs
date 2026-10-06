//! The retained HUD tree: the prompt slip, the stack, the tray, and the one
//! system that rebuilds all of it.
//!
//! Rebuilt only when [`HudRevision`] says something it draws has changed —
//! a rebuild per frame would cost more than the whole table does.
//!
//! What a seat *is* — its name, its life, its zones, the turn it is taking —
//! is no longer drawn here. That is [`super::seatbar`], written on each
//! seat's own mat, and the tab strip and the phase rail this module used to
//! open with are gone with it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::commanderdamage;

/// How wide the commander-damage track is drawn, in logical pixels.
///
/// The same for every seat, whatever that seat's worst source is, because the
/// bar's whole job is to be comparable: it stands for twenty-one
/// (`commanderdamage::LETHAL`) and nothing else, so a half-full bar means the
/// same thing under every name at the table.
#[expect(dead_code, reason = "the seat sheet draws the track next")]
const TRACK_W: f32 = 52.0;

/// The gap between two answers on the prompt slip, in logical pixels.
///
/// Small, and it has to be: the answers divide the sheet between them, so
/// every pixel here is a pixel off each button. Wide enough that two brass
/// edges never touch, narrow enough that the row still reads as one control
/// with parts rather than as scattered buttons.
pub(super) const BUTTON_GAP: f32 = 8.0;

/// The label on an answer, in logical pixels.
pub(super) const ANSWER_PT: f32 = 13.0;

/// The caption over the card underneath a copy, one line of nine-pixel type.
///
/// Named because it is the difference between bottom-aligning the *pair* with
/// the preview and bottom-aligning the card alone, which would leave the two
/// feet a caption apart.
const CAPTION_H: f32 = 13.0;

/// The second life total (CR 903.10a), drawn as a track: the commander glyph,
/// a fixed-width bar filled to the worst source, one tick per other
/// commander, and the worst number itself.
///
/// Shown like a life total and not like a row of numbers, because the
/// question is "how close am I to dying to this" and not "what do these add
/// up to" — they add up to nothing the rules recognise. `None` when no
/// commander has connected: a bar sitting at zero under every seat every game
/// would be noise, the same rule poison and energy follow.
///
/// Fixed width, and the same width for every seat: the eye learns what full
/// looks like once and then reads every other seat against it. A bar scaled
/// to its own worst source would make two seats with very different problems
/// look identical.
///
/// Every node of it is `Pickable::IGNORE`, and that is load-bearing rather
/// than tidy: anything pickable in front of a control stops
/// `PickingInteraction` at itself, so a `Feel` behind the track would go dead
/// across its whole width — the label finding again, and invisible in an
/// ordinary game for the same reason the overflow was.
///
/// It was written inside the seat tab and outlived it. It was lifted out
/// **before** the tab was deleted rather than after, which is the whole point
/// of it existing unused for one commit: the seat sheet is to redraw the
/// track exactly as the tab drew it, and "exactly" is not something that can
/// be recovered from a diff two commits later.
#[expect(dead_code, reason = "the seat sheet calls it next")]
pub(super) fn spawn_commander_track(
    commands: &mut Commands,
    fonts: &UiFonts,
    seat: &baylee_view::SeatView,
    muted: Color,
) -> Option<Entity> {
    let track = commanderdamage::Track::of(&seat.commander_damage)?;
    let color = if track.danger { palette::DANGER } else { muted };
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4.0),
                ..default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(glyph::COMMAND.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(color),
                Pickable::IGNORE,
            )],
        ))
        .id();

    let bar = commands
        .spawn((
            Node {
                width: px(TRACK_W),
                height: px(4.0),
                border_radius: BorderRadius::all(px(2.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(bar);

    let fill = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: px(TRACK_W * track.fill),
                height: px(4.0),
                border_radius: BorderRadius::all(px(2.0)),
                ..default()
            },
            BackgroundColor(color),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(fill);

    // One tick per other commander, on the same scale. They say how many more
    // clocks are running and how far along each is; which commander is which
    // is the tooltip's answer, the same half this panel already gives for a
    // counter's colour.
    for at in &track.ticks {
        let tick = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(TRACK_W * at),
                    top: px(0.0),
                    width: px(1.0),
                    height: px(4.0),
                    ..default()
                },
                BackgroundColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(tick);
    }

    let worst = commands
        .spawn((
            Text::new(track.worst.to_string()),
            tf(fonts, 11.0),
            TextColor(color),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(worst);
    Some(row)
}

mod answers;
mod preview;
mod surfaces;
mod sync;

pub(crate) use answers::*;
pub(crate) use preview::*;
pub use surfaces::*;
pub use sync::*;

#[cfg(test)]
mod tests;
