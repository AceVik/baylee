//! A text field: its box, its runs of text, its caret and the caret's blink.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// ----------------------------------------------------------- node makers

/// Everything a [`text_field`] draws that is not its label.
pub(crate) struct FieldLook<'a> {
    /// The text, the caret and the selection to draw.
    pub(crate) buffer: &'a TextBuffer,
    /// Whether this is the field with the caret.
    pub(crate) focused: bool,
    /// Set on a password, and `None` on every other box.
    ///
    /// Masked *here* and not by the caller: the caret and the selection are
    /// byte offsets into the real text, and a caller that handed over a
    /// string of bullets would be handing over offsets into a different
    /// string — a bullet is three bytes and the letter it stands for is one
    /// to four.
    pub(crate) mask: Option<Masked>,
    /// What a tap on it means.
    pub(crate) press: Press,
    /// A glyph button at the far end of the box.
    ///
    /// Its own field and not a second shape of [`FieldLook::mask`]: the eye
    /// belongs to a password and is *about* the text, and this is about what
    /// the box is for. A box may have both — a search box has a gear and no
    /// eye, and nothing has two of either.
    pub(crate) tail: Option<FieldTail>,
    /// A glyph from the icon face, drawn before the text.
    ///
    /// A search box is the one shape a player recognises without reading it,
    /// and the magnifier is what makes it that shape. It is `Pickable::IGNORE`
    /// like every other label inside a control, so the tap finds the box.
    pub(crate) lead: Option<char>,
    /// What the box says while nothing has been typed into it.
    ///
    /// Beside the caption above the box rather than instead of it: the caption
    /// says what the box *is* and survives being typed into, and this says
    /// what may go in it and is gone the moment anything does. A box with
    /// neither was a rectangle a player had to guess at.
    pub(crate) hint: Option<&'a str>,
}

/// A glyph button at the end of a field: what it draws and what it means.
#[derive(Clone, Copy)]
pub(crate) struct FieldTail {
    /// The mark, from the icon face.
    pub(crate) glyph: char,
    /// What a tap on it means.
    pub(crate) press: Press,
    /// Whether what it opens is open, which is what lights it.
    pub(crate) lit: bool,
}

/// A password box: what the eye beside it addresses, and whether it is open.
#[derive(Clone, Copy)]
pub(crate) struct Masked {
    /// The field the eye toggles; `None` for a secret with no eye, which is
    /// never shown in the clear (a model's key, `crate::seatpanel`).
    pub(crate) field: Option<Field>,
    /// Whether the player has asked to read what they are typing.
    pub(crate) shown: bool,
}

impl Masked {
    /// A box that is always masked and has no eye.
    pub(crate) const fn sealed() -> Self {
        Self {
            field: None,
            shown: false,
        }
    }
}

/// The caret drawn in the field that has it.
///
/// A component with a system of its own rather than a glyph in the string,
/// because the tree is retained: making a bar appear and disappear twice a
/// second by rebuilding it would rebuild every row of the table list with it.
/// `at` is where the caret stands and what stands around it, so a rebuild
/// that puts it back exactly where it was leaves the blink where it was too.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) struct Caret {
    /// The caret offset, the length of the selection, and the length of the
    /// text — enough for "the same caret in the same field" and no more.
    at: (usize, usize, usize),
}

/// How long the caret spends lit, and then dark.
///
/// 530 ms is the rate every desktop text field has blinked at for thirty
/// years. It is not a number to improve on: a caret is recognised rather
/// than read, and one blinking at a rate nothing else does reads as a fault.
const BLINK_SECS: f32 = 0.530;

/// Blinks the caret, and only the caret.
///
/// Touches one `BackgroundColor` and no state, so a quiet frame still costs
/// nothing and the tree is never rebuilt for it.
pub(super) fn blink(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut carets: Query<(&Caret, &mut BackgroundColor)>,
    mut since: Local<f32>,
    mut drawn: Local<Option<Caret>>,
) {
    let Ok((caret, mut colour)) = carets.single_mut() else {
        *drawn = None;
        return;
    };
    // A caret that has just moved, or has just had a letter typed at it, is a
    // caret being looked at: it goes solid and the cycle starts again.
    *since = if *drawn == Some(*caret) {
        (*since + time.delta_secs()) % (BLINK_SECS * 2.0)
    } else {
        0.0
    };
    *drawn = Some(*caret);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let want = if caret_lit(*since, still) {
        palette::INK
    } else {
        Color::NONE
    };
    if colour.0 != want {
        colour.0 = want;
    }
}

/// Whether the bar is drawn, `since` seconds after the caret last moved.
///
/// A phase rather than a toggle, so nothing has to be kept in step with
/// anything: the answer is a function of how long the caret has stood still,
/// and a system that missed a frame is right again on the next one.
pub(crate) fn caret_lit(since: f32, still: bool) -> bool {
    // `reduce_motion` is a promise that nothing moves, and a bar that comes
    // and goes twice a second is movement.
    still || since % (BLINK_SECS * 2.0) < BLINK_SECS
}

/// A labelled text box that takes the caret when tapped.
pub(crate) fn text_field(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    look: &FieldLook,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small * 0.8),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                overflow: Overflow::clip_x(),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                // Two pixels whether or not it has the caret, so that taking
                // the caret rings the box instead of moving everything in it
                // a pixel to the left.
                border: UiRect::all(px(2)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if look.focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.06)
            }),
            look.press,
        ))
        .id();
    if let Some(glyph) = look.lead {
        let mark = commands
            .spawn((
                Text::new(glyph.to_string()),
                crate::hud::icon_tf(fonts, metrics.small * 0.85),
                TextColor(palette::MUTED),
                Node {
                    margin: UiRect::right(px(metrics.gap * 0.5)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(boxed).add_child(mark);
    }
    for run in field_runs(commands, fonts, metrics, look) {
        commands.entity(boxed).add_child(run);
    }
    mark_runs(commands, boxed, metrics, look);
    // After the runs, so the caret stands in front of it the way a caret
    // stands in front of an empty `<input>`'s placeholder.
    if let Some(words) = look.hint.filter(|_| look.buffer.text().is_empty()) {
        let ghost = hint_ghost(commands, fonts, metrics, words);
        commands.entity(boxed).add_child(ghost);
    }
    if look.mask.is_some() || look.tail.is_some() {
        // Pushed to the far end of the row, so a button is in the same place
        // whatever is typed and the letters never run into it.
        let gap = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: px(metrics.gap),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(boxed).add_child(gap);
        let eye = look
            .mask
            .and_then(|m| eye_button(commands, fonts, metrics, m));
        commands.entity(boxed).add_children(eye.as_slice());
        if let Some(tail) = look.tail {
            let button = icon_button(commands, fonts, metrics, tail);
            commands.entity(boxed).add_child(button);
        }
    }
    // A box with no caption (the builder's search and title) is the box
    // alone: an empty line above it would be a line of nothing.
    if label.is_empty() {
        commands.entity(caption).despawn();
        commands.entity(column).add_child(boxed);
    } else {
        commands.entity(column).add_children(&[caption, boxed]);
    }
    column
}

/// The eye at the end of a password box.
///
/// A glyph and no word, which is the one place in this interface where that
/// is right: there is no room for a label beside the text inside a box this
/// tall, and the eye is read the same way in every language — every sign-in
/// form on the web has one. It is `Pickable` by default and the box around
/// it is not, so the click finds the eye rather than the field under it.
fn eye_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    masked: Masked,
) -> Option<Entity> {
    // A sealed box has no eye: what is in it is never shown.
    let field = masked.field?;
    Some(icon_button(
        commands,
        fonts,
        metrics,
        FieldTail {
            glyph: if masked.shown {
                crate::hud::glyph::EYE_SLASH
            } else {
                crate::hud::glyph::EYE
            },
            press: Press::Shared(SharedPress::Reveal(field)),
            lit: masked.shown,
        },
    ))
}

/// One glyph button at the end of a field.
fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    tail: FieldTail,
) -> Entity {
    let mark = commands
        .spawn((
            Text::new(tail.glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(if tail.lit {
                palette::ACCENT
            } else {
                palette::MUTED
            }),
            Pickable::IGNORE,
        ))
        .id();
    let button = commands
        .spawn((
            Node {
                // A finger's worth of height, and enough width to be hit
                // without pushing the text out of a narrow box.
                min_width: px(metrics.tap * 0.7),
                align_self: AlignSelf::Stretch,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            tail.press,
        ))
        .id();
    commands.entity(button).add_child(mark);
    button
}

/// The text inside a field: up to three runs with a bar between two of them.
///
/// No glyph metrics anywhere. The row is already measuring the letters, so a
/// caret put into it as the next thing in the row lands exactly where the
/// letters end — which is the one placement that cannot drift from the font.
fn field_runs(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    look: &FieldLook,
) -> Vec<Entity> {
    let seg = look.buffer.segments();
    let caret = look.focused.then(|| {
        spawn_caret(
            commands,
            metrics,
            Caret {
                at: (
                    look.buffer.cursor(),
                    seg.selected.len(),
                    look.buffer.text().len(),
                ),
            },
        )
    });
    // Head, caret, selection, tail — with the caret on the other side of the
    // selection when that is the end the player is holding. An empty run is
    // no node: a field with the caret at its start is one bar and nothing.
    let mut out = Vec::new();
    // Before the selection, or before the tail: the two sides of a selected
    // run, and the same seam when nothing is selected and the run is empty.
    let caret_at = usize::from(seg.caret_after_selection) + 1;
    // Only the field with the caret shows a selection. Both marks say the
    // same thing — *this is where the typing goes* — so a field that has
    // neither the caret nor the typing must show neither, and the accent
    // that rings the focused box was standing in two boxes at once.
    let runs = [
        (seg.head, false),
        (seg.selected, look.focused),
        (seg.tail, false),
    ];
    for (i, (text, selected)) in runs.into_iter().enumerate() {
        if i == caret_at
            && let Some(caret) = caret
        {
            out.push(caret);
        }
        if !text.is_empty() {
            let mask = look.mask.is_some_and(|masked| !masked.shown);
            out.push(spawn_run(commands, fonts, metrics, text, mask, selected));
        }
    }
    out
}

/// The box of the lobby field holding the caret, as [`text_field`] drew it.
///
/// The caret moving is the one edit that changes nothing outside its own box,
/// so the keyboard makes it without marking the lobby's state changed and
/// [`retrace_runs`] redraws the box's runs here instead: one field's three or
/// four nodes rather than the whole screen per arrow key (§10 #3 of the
/// shell design).
#[derive(Component)]
pub(super) struct RunsOf {
    field: Field,
    mask: Option<Masked>,
    /// Whether a lead glyph stands before the runs, which they go after.
    lead: bool,
    metrics: Metrics,
    /// The caret and the selection the runs were drawn for.
    at: (usize, Option<std::ops::Range<usize>>),
}

/// What an empty box says it is for, in muted ink, clipped rather than
/// wrapped.
fn hint_ghost(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, words: &str) -> Entity {
    commands
        .spawn((
            Text::new(words.to_string()),
            tf(fonts, metrics.text),
            TextLayout::no_wrap(),
            TextColor(palette::MUTED),
            Node {
                min_width: px(0),
                overflow: Overflow::clip_x(),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Marks the box of the field with the caret, so its runs can be redrawn in
/// place when only the caret moves ([`retrace_runs`]).
fn mark_runs(commands: &mut Commands, boxed: Entity, metrics: Metrics, look: &FieldLook) {
    if look.focused
        && let Press::Shared(SharedPress::Focus(field)) = look.press
    {
        commands.entity(boxed).insert(RunsOf {
            field,
            mask: look.mask,
            lead: look.lead.is_some(),
            metrics,
            at: (look.buffer.cursor(), look.buffer.selection()),
        });
    }
}

/// One run of a field's text, or its caret: what [`retrace_runs`] replaces.
#[derive(Component)]
pub(super) struct FieldRun;

/// Redraws the focused field's runs where only its caret or selection moved.
///
/// After [`ui`]: a rebuild this frame has already drawn them from the state,
/// and then this finds nothing to do.
pub(super) fn retrace_runs(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<UiFonts>>,
    mut boxes: Query<(Entity, &mut RunsOf, &Children)>,
    runs: Query<(), With<FieldRun>>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    let focus = state.lobby.focus();
    for (entity, mut drawn, children) in &mut boxes {
        if drawn.field != focus {
            continue;
        }
        let buffer = state.lobby.buffer(focus);
        let at = (buffer.cursor(), buffer.selection());
        if drawn.at == at {
            continue;
        }
        drawn.at = at;
        for child in children {
            if runs.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        let look = FieldLook {
            buffer,
            focused: true,
            mask: drawn.mask,
            press: Press::Shared(SharedPress::Focus(focus)),
            tail: None,
            lead: None,
            hint: None,
        };
        let fresh = field_runs(&mut commands, &fonts, drawn.metrics, &look);
        commands
            .entity(entity)
            .insert_children(usize::from(drawn.lead), &fresh);
    }
}

/// One run of a field's text, masked where the field is a password.
fn spawn_run(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    text: &str,
    mask: bool,
    selected: bool,
) -> Entity {
    let shown = if mask {
        "\u{2022}".repeat(text.chars().count())
    } else {
        text.to_string()
    };
    let mut run = commands.spawn((
        FieldRun,
        Text::new(shown),
        tf(fonts, metrics.text),
        TextColor(palette::INK),
        Pickable::IGNORE,
    ));
    if selected {
        run.insert(BackgroundColor(palette::SELECTION));
    }
    run.id()
}

/// The bar itself: one logical pixel, half of it borrowed from each side so
/// that showing it moves no letter.
fn spawn_caret(commands: &mut Commands, metrics: Metrics, caret: Caret) -> Entity {
    commands
        .spawn((
            FieldRun,
            caret,
            Node {
                width: px(1),
                // Bevy lays a text node out at 1.2 times the font size, so a
                // shorter bar would stand lower than the selection beside it
                // and read as a fault rather than as a caret.
                height: px(metrics.text * 1.2),
                margin: UiRect::horizontal(px(-0.5)),
                ..default()
            },
            BackgroundColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
}
