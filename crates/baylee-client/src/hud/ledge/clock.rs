//! The decision clock, in its button or beside the answers.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

/// Where the countdown stands, and in which button when it is in one.
///
/// In the button the clock presses when it runs out, when this row has it
/// (#258, [`baylee_client_core::ledge::clock_answer`]); beside the question
/// otherwise. Only this seat's own question puts answers on the shelf, so a
/// clock in a button is always this seat's. An armed deed replaces the row,
/// and the answers it replaced cannot carry anything.
pub(super) fn clock_placement(
    duel: &Duel,
    shown: bool,
    armed: bool,
    answers: &[(Says, String)],
) -> (Clock, Option<PromptAction>) {
    if !shown {
        return (Clock::None, None);
    }
    let clocked = duel
        .interaction
        .as_ref()
        .filter(|_| !armed)
        .and_then(|i| baylee_client_core::ledge::clock_answer(i.pending()))
        .filter(|button| {
            answers
                .iter()
                .any(|(says, _)| *says == Says::Answer(*button))
        });
    match clocked {
        Some(_) => (Clock::InButton, clocked),
        None => (Clock::Beside, None),
    }
}

/// Where the decision countdown stands on the shelf, if it stands at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Clock {
    /// No countdown is shown.
    None,
    /// In its own cell left of the question: someone else's clock, or a
    /// question the house answers when it runs out.
    Beside,
    /// In the text of the button the clock presses (#258).
    InButton,
}

/// The countdown inside a button, after its words (#258).
///
/// The same [`DecisionClockLabel`] the cell is, so
/// [`count_down_the_decision`] writes whichever one was built, and the same
/// ink as the button's words, because it is part of what the button says:
/// "Pass 12" is what pressing nothing will do in twelve seconds.
/// Its width is fixed at two digits for the cell's reason, so the button
/// does not change size once a second.
pub(super) fn button_clock(commands: &mut Commands, fonts: &UiFonts, ink: Color) -> Entity {
    commands
        .spawn((
            DecisionClockLabel,
            Text::default(),
            super::tf_bold(fonts, LABEL_PT),
            TextColor(ink),
            Node {
                width: px(button_clock_width()),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The room [`button_clock`] takes: the widest number it holds, at the
/// button's own size.
pub(super) fn button_clock_width() -> f32 {
    super::text_width("60", LABEL_PT, true)
}

/// The countdown's cell, so the seconds can be written in place.
///
/// The same reason [`pool::PoolCount`] exists one file over: a
/// number that changes is read, not watched, and a tree rebuilt to carry it
/// would take every `Feel` on the shelf back to rest once a second.
#[derive(Component)]
pub struct DecisionClockLabel;

/// Counts the awaited seat's clock down and writes it where it stands.
///
/// The whole of the per-frame work, and it touches no `Node`: the cell was
/// given its width when it was spawned, and this only ever assigns a
/// `String`. The assignment is guarded on the text having actually changed,
/// which matters more than it looks — writing an equal `Text` still marks it
/// changed, and `bevy_text` re-lays every glyph of a component it is told
/// moved. Guarded, that happens about once a second instead of once a frame.
///
/// It also pushes the sound, because the threshold is crossed by *time* and
/// not by a view: at a table with a long limit no view arrives at the moment
/// sixty seconds are left, so a client that only listened to views would
/// never make the sound at all.
pub fn count_down_the_decision(
    time: Res<Time>,
    mut duel: ResMut<crate::Duel>,
    mut label: Query<&mut Text, With<DecisionClockLabel>>,
) {
    // Past change detection, as `sound::tell_the_cues_the_time` tells the
    // cue queue the time: a clock ticking is not the duel changing, and a
    // write through `DerefMut` reported the whole duel as moved on every
    // frame. A cue it hands over is a change, and is written as one.
    let clock = &mut duel.bypass_change_detection().clock;
    clock.advance(time.delta_secs());
    if let Some(cue) = clock.claim() {
        duel.cues.push(cue);
    }
    let Ok(mut text) = label.single_mut() else {
        return;
    };
    let says = duel
        .clock
        .shown()
        .map_or_else(String::new, |left| left.to_string());
    if text.0 != says {
        text.0 = says;
    }
}

/// The room the countdown takes, reserved for the widest number it holds.
///
/// `shown()` never exceeds `SHOW_AT`, so the cell is two digits wide and is
/// given that width **explicitly** rather than sized to its content. A cell
/// that resized as the digits changed would shove the sentence beside it
/// sideways once a second, which is the one thing a clock on a shelf must not
/// do — and it would do it through `Node`, which is exactly what the writing
/// system is kept away from.
pub(super) fn clock_width() -> f32 {
    super::text_width("60", SENTENCE_PT, true) + baylee_client_core::ledge::SENTENCE_GAP
}
