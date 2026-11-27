//! A text field: its box, its runs of text, its caret and the caret's blink.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// ----------------------------------------------------------- node makers

/// Verified Font Awesome Free (`fa-solid-900.ttf`) prefix glyphs for lobby fields.
pub(crate) mod lead_glyph {
    /// `fa-user` (`U+F007`): username / account handle field.
    pub(crate) const USER: char = '\u{f007}';
    /// `fa-id-card` (`U+F2C2`): display name / guest handle field.
    pub(crate) const ID_CARD: char = '\u{f2c2}';
    /// `fa-lock` (`U+F023`): password / password confirmation field.
    pub(crate) const LOCK: char = crate::hud::glyph::LOCK;
    /// `fa-key` (`U+F084`): new password / reset credential field.
    pub(crate) const KEY: char = '\u{f084}';
    /// `fa-globe` (`U+F0AC`): gateway URL / server address field.
    pub(crate) const GLOBE: char = '\u{f0ac}';
    /// `fa-magnifying-glass` (`U+F002`): search / filter field.
    pub(crate) const SEARCH: char = crate::hud::glyph::MAGNIFIER;
}

/// Marker on the leading prefix-icon slot inside a [`text_field`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldLeadIcon;

/// Horizontal scroll viewport holding a single-line field's text runs and caret.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FieldTrack;

/// Unfocused prefix-icon tint: muted warm brass/parchment.
pub(crate) const LEAD_REST_INK: Color = Color::srgba(0.76, 0.70, 0.56, 0.76);
/// Focused prefix-icon tint: warm luminous gold/ivory.
pub(crate) const LEAD_FOCUS_INK: Color = Color::srgba(0.96, 0.85, 0.58, 1.0);

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

/// The kit's focus stops a field carries (the front door's, WP1): the
/// box's, with the field it types into, and its eye's.
#[derive(Clone, Copy)]
pub(crate) struct FieldStops {
    /// The box's stop.
    pub(crate) field: crate::shellkit::focus::Stop,
    /// The lobby's field the box types into (`None`: a box whose own
    /// editor takes the keys once pressed, the seat panel's).
    pub(crate) typed: Option<Field>,
    /// The eye's stop, on a password.
    pub(crate) eye: Option<crate::shellkit::focus::Stop>,
}

/// A labelled text box that takes the caret when tapped.
pub(crate) fn text_field(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    look: &FieldLook,
) -> Entity {
    text_field_hinted(commands, fonts, metrics, label, look, None, None)
}

/// [`text_field`], carrying the kit's focus stops.
pub(crate) fn text_field_with(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    look: &FieldLook,
    stops: Option<FieldStops>,
) -> Entity {
    text_field_hinted(commands, fonts, metrics, label, look, stops, None)
}

fn field_box_shadow() -> BoxShadow {
    BoxShadow(vec![ShadowStyle {
        color: Color::srgba(0.0, 0.0, 0.0, 0.28),
        x_offset: px(0.0),
        y_offset: px(2.0),
        spread_radius: px(0.0),
        blur_radius: px(6.0),
    }])
}

/// [`text_field_with`], with an optional tooltip on the trailing action button.
#[allow(clippy::too_many_lines)]
pub(super) fn text_field_hinted(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    look: &FieldLook,
    stops: Option<FieldStops>,
    tail_hint: Option<&str>,
) -> Entity {
    use crate::shellkit::tokens;
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
    if !label.is_empty() {
        let caption = commands
            .spawn((
                Text::new(label),
                tf(fonts, metrics.small * 0.82),
                TextColor(Color::srgb(0.82, 0.85, 0.89)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(column).add_child(caption);
    }
    let radius = if look.lead == Some(crate::hud::glyph::MAGNIFIER) {
        tokens::RADIUS_PILL
    } else {
        tokens::RADIUS_CONTROL
    };
    let rest_border = Color::srgba(0.64, 0.70, 0.78, 0.34);
    let border = if look.focused {
        tokens::ACCENT
    } else {
        rest_border
    };
    let box_bg = if look.focused {
        Color::srgba(0.026, 0.054, 0.092, 0.74)
    } else {
        Color::srgba(0.018, 0.038, 0.068, 0.60)
    };
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                overflow: Overflow::clip_x(),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.72), px(2)),
                // Constant 1 px border whether or not it has the caret, so that
                // taking the caret rings the box instead of shifting its text.
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(box_bg),
            BorderColor::all(border),
            field_box_shadow(),
            look.press,
        ))
        .id();
    if !look.focused {
        commands.entity(boxed).insert(crate::ambience::FeelBorder {
            rest: rest_border,
            hot: tokens::PRIMARY_EDGE,
            press: tokens::ACCENT,
        });
    }
    if let Some(stops) = stops {
        commands
            .entity(boxed)
            .insert((stops.field, crate::shellkit::role::Role::Field));
        if let Some(typed) = stops.typed {
            commands
                .entity(boxed)
                .insert(super::front::keys::LobbyField(typed));
        }
    }
    if let Some(glyph) = look.lead {
        let mark = lead_mark(commands, fonts, metrics, glyph, look.focused);
        commands.entity(boxed).add_child(mark);
    }
    let track = commands
        .spawn((
            FieldTrack,
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                height: percent(100),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(2)),
                overflow: Overflow::scroll_x(),
                ..default()
            },
            ScrollPosition::default(),
            Pickable::IGNORE,
        ))
        .id();
    for run in field_runs(commands, fonts, metrics, look) {
        commands.entity(track).add_child(run);
    }
    mark_runs(commands, track, metrics, look);
    commands.entity(boxed).add_child(track);
    append_field_tail(commands, fonts, metrics, boxed, look, stops, tail_hint);
    commands.entity(column).add_child(boxed);
    column
}

fn append_field_tail(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    boxed: Entity,
    look: &FieldLook,
    stops: Option<FieldStops>,
    tail_hint: Option<&str>,
) {
    if look.mask.is_none() && look.tail.is_none() {
        return;
    }
    // Keeps a steady gutter between the scrolling text viewport and the tail
    // icons, while the viewport's `flex_grow` pins the buttons to the far end.
    let gap = commands
        .spawn((
            Node {
                width: px(metrics.gap * 0.65),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(boxed).add_child(gap);
    let eye = look
        .mask
        .and_then(|m| eye_button(commands, fonts, metrics, m));
    if let (Some(eye), Some(stop)) = (eye, stops.and_then(|s| s.eye)) {
        commands.entity(eye).insert(stop);
    }
    commands.entity(boxed).add_children(eye.as_slice());
    if let Some(tail) = look.tail {
        let button = icon_button(commands, fonts, metrics, tail, tail_hint);
        commands.entity(boxed).add_child(button);
    }
}

/// Fixed-width, vertically centered prefix-icon slot inside an input box.
///
/// Wrapped in its own container `Node` so every field's text aligns at the
/// exact same left edge regardless of glyph advance, and muted warm
/// brass/parchment brightens to warm gold/ivory when the field holds focus.
fn lead_mark(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    glyph: char,
    focused: bool,
) -> Entity {
    let ink = if focused {
        LEAD_FOCUS_INK
    } else {
        LEAD_REST_INK
    };
    commands
        .spawn((
            FieldLeadIcon,
            Node {
                width: px(18),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                margin: UiRect::right(px(metrics.gap * 0.65)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.small * 0.90),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
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
        None,
    ))
}

/// One glyph button at the end of a field.
fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    tail: FieldTail,
    hint: Option<&str>,
) -> Entity {
    use crate::shellkit::tokens;
    // Syntax help in search fields always uses the `?` circle-question glyph,
    // never filter sliders (`\u{f1de}`).
    let glyph = if tail.glyph == '\u{f1de}'
        && matches!(tail.press, Press::Build(super::BuildPress::ToggleSyntax))
    {
        '\u{f059}'
    } else {
        tail.glyph
    };
    let rest_bg = if tail.lit {
        tokens::SELECTED
    } else {
        Color::NONE
    };
    let size = (metrics.tap - 8.0).max(24.0);
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(if tail.lit {
                tokens::ACCENT
            } else {
                tokens::MUTED
            }),
            Pickable::IGNORE,
        ))
        .id();
    let button = commands
        .spawn((
            crate::shellkit::role::Role::Button,
            Node {
                min_width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_self: AlignSelf::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(rest_bg),
            crate::ambience::Feel::tinting_to(rest_bg, tokens::HOVER),
            tail.press,
        ))
        .id();
    if let Some(hint) = hint.filter(|s| !s.is_empty()) {
        commands
            .entity(button)
            .insert(super::hint::HoverHint(hint.to_string()));
    }
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
    // After the runs, so the caret stands in front of it the way a caret
    // stands in front of an empty `<input>`'s placeholder.
    if let Some(words) = look.hint.filter(|_| look.buffer.text().is_empty()) {
        out.push(hint_ghost(commands, fonts, metrics, words));
    }
    out
}

/// The box of the lobby field holding the caret, as [`text_field`] drew it.
///
/// Editing or moving the caret in a plain text field changes nothing outside
/// its own box, so the keyboard makes the edit without marking the lobby's
/// state changed and [`retrace_runs`] redraws the box's runs here instead:
/// one field's three or four nodes rather than the whole screen per keystroke
/// (§10 #3 of the shell design).
#[derive(Component)]
pub(super) struct RunsOf {
    field: Field,
    mask: Option<Masked>,
    hint: Option<String>,
    metrics: Metrics,
    /// The text's revision, the caret and the selection the runs were drawn
    /// for: never the text itself, which for a password would be a copy
    /// outside the buffer's zeroizing ([`baylee_client_core::textbuf::TextBuffer::revision`]).
    at: (u64, usize, Option<std::ops::Range<usize>>),
}

/// What an empty box says it is for, in muted ink, clipped rather than
/// wrapped.
fn hint_ghost(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, words: &str) -> Entity {
    commands
        .spawn((
            FieldRun,
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

/// Marks the text track of the field with the caret, so its runs can be
/// redrawn in place when only its text or caret moves ([`retrace_runs`]).
fn mark_runs(commands: &mut Commands, track: Entity, metrics: Metrics, look: &FieldLook) {
    if look.focused
        && let Press::Shared(SharedPress::Focus(field)) = look.press
    {
        commands.entity(track).insert(RunsOf {
            field,
            mask: look.mask,
            hint: look.hint.map(str::to_string),
            metrics,
            at: (
                look.buffer.revision(),
                look.buffer.cursor(),
                look.buffer.selection(),
            ),
        });
    }
}

/// One run of a field's text, its placeholder, or its caret: what [`retrace_runs`] replaces.
#[derive(Component)]
pub(super) struct FieldRun;

/// Redraws the focused field's runs where only its text, caret or selection moved.
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
    // A parked caret (focus on a control, WP1) is drawn in no field.
    if state.lobby.caret_parked() {
        return;
    }
    for (entity, mut drawn, children) in &mut boxes {
        if drawn.field != focus {
            continue;
        }
        let buffer = state.lobby.buffer(focus);
        let at = (buffer.revision(), buffer.cursor(), buffer.selection());
        if drawn.at == at {
            continue;
        }
        drawn.at = at;
        for child in children {
            if runs.contains(*child) {
                commands.entity(*child).try_despawn();
            }
        }
        let look = FieldLook {
            buffer,
            focused: true,
            mask: drawn.mask,
            press: Press::Shared(SharedPress::Focus(focus)),
            tail: None,
            lead: None,
            hint: drawn.hint.as_deref(),
        };
        let fresh = field_runs(&mut commands, &fonts, drawn.metrics, &look);
        commands.entity(entity).insert_children(0, &fresh);
    }
}

/// Scrolls each focused single-line field's [`FieldTrack`] horizontally so
/// its [`Caret`] stays visible inside the box whenever text overflows.
///
/// Runs after [`bevy::ui::UiSystems::Layout`] and before
/// [`bevy::ui::UiSystems::PostLayout`], shifting descendants'
/// [`UiGlobalTransform`] in the same frame so newly typed characters or tree
/// rebuilds never flash un-scrolled for a frame.
#[allow(clippy::type_complexity)]
pub(super) fn keep_caret_in_view(
    mut tracks: Query<(Entity, &ComputedNode, &mut ScrollPosition, &Children), With<FieldTrack>>,
    carets: Query<(), With<Caret>>,
    children_q: Query<&Children>,
    mut transforms: Query<&mut UiGlobalTransform>,
) {
    const EDGE_PAD: f32 = 2.0;
    for (track, computed, mut scroll, kids) in &mut tracks {
        let inv = computed.inverse_scale_factor();
        let view_w = computed.size().x * inv;
        let content_w = computed.content_size().x * inv;
        if view_w <= 0.0 {
            continue;
        }
        let max_x = (content_w - view_w).max(0.0);
        let mut target_x = scroll.x.clamp(0.0, max_x);
        if max_x <= 0.0 {
            target_x = 0.0;
        } else if let Some(caret) = kids.iter().find(|&k| carets.contains(k))
            && let (Ok(track_at), Ok(caret_at)) = (transforms.get(track), transforms.get(caret))
        {
            let track_left = track_at.translation.x * inv - view_w * 0.5;
            let caret_x = caret_at.translation.x * inv;
            let caret_content_x = (caret_x - track_left) + scroll.x;
            if caret_content_x + EDGE_PAD > target_x + view_w {
                target_x = (caret_content_x + EDGE_PAD - view_w).clamp(0.0, max_x);
            } else if caret_content_x - EDGE_PAD < target_x {
                target_x = (caret_content_x - EDGE_PAD).clamp(0.0, max_x);
            }
        }
        let delta = target_x - scroll.x;
        if delta.abs() > 0.25 {
            scroll.x = target_x;
            if inv > 0.0 {
                let shift_phys = -delta / inv;
                for desc in children_q.iter_descendants(track) {
                    if let Ok(mut t) = transforms.get_mut(desc) {
                        *t = UiGlobalTransform::from(bevy::math::Affine2 {
                            matrix2: t.matrix2,
                            translation: t.translation + Vec2::new(shift_phys, 0.0),
                        });
                    }
                }
            }
        }
    }
}

/// How much larger a password's bullets are set than its letters would be.
const MASK_SCALE: f32 = 1.5;

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
        TextLayout::no_wrap(),
        TextColor(palette::INK),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
    if mask {
        // The bullets a size up (owner, 09.10.2026: "bigger bullets"), on
        // the line the letters would stand on, so the box and the caret keep
        // their height whether the password is shown or not.
        run.insert((
            tf(fonts, metrics.text * MASK_SCALE),
            bevy::text::LineHeight::Px(metrics.text * crate::hud::UI_SCALE * 1.2),
        ));
    }
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
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_field_never_wraps_and_scrolls_to_keep_caret_visible() {
        let measure = |sample: &str, move_to_start: bool| {
            let (mut app, fonts) = crate::face::tests::layout_app();
            app.add_systems(
                PostUpdate,
                keep_caret_in_view
                    .after(bevy::ui::UiSystems::Layout)
                    .before(bevy::ui::UiSystems::PostLayout),
            );
            let mut buffer = TextBuffer::new(sample);
            if move_to_start {
                buffer.place(0, None);
            }
            let metrics = Metrics::of(1600.0);
            let look = FieldLook {
                buffer: &buffer,
                focused: true,
                mask: None,
                press: Press::Shared(SharedPress::Focus(Field::Username)),
                tail: None,
                lead: Some(lead_glyph::USER),
                hint: None,
            };
            let mut commands = app.world_mut().commands();
            let holder = commands
                .spawn(Node {
                    width: px(320),
                    height: px(120),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .id();
            let field = text_field(&mut commands, &fonts, metrics, "USERNAME", &look);
            commands.entity(holder).add_child(field);
            app.world_mut().flush();
            for _ in 0..4 {
                app.update();
            }
            let world = app.world_mut();
            let boxed_h = world
                .query::<(&Press, &ComputedNode)>()
                .iter(world)
                .find(|(p, _)| **p == Press::Shared(SharedPress::Focus(Field::Username)))
                .map(|(_, c)| c.size().y)
                .expect("field box");
            let (track_rect, track_content_w, scroll_x) = world
                .query_filtered::<(
                    &ComputedNode,
                    &UiGlobalTransform,
                    &ScrollPosition,
                ), With<FieldTrack>>()
                .single(world)
                .map(|(c, t, s)| {
                    (
                        Rect::from_center_size(t.translation, c.size()),
                        c.content_size().x,
                        s.x,
                    )
                })
                .expect("FieldTrack");
            let caret_x = world
                .query_filtered::<&UiGlobalTransform, With<Caret>>()
                .single(world)
                .expect("Caret")
                .translation
                .x;
            (boxed_h, track_rect, track_content_w, scroll_x, caret_x)
        };

        let (short_h, ..) = measure("alice", false);
        let hyphenated = "qa-test-long-username-1234567890-abcdefghijklmno-60character-extra-overflow-check-0123456789-end";
        let (hyph_h, hyph_rect, hyph_content_w, hyph_scroll, hyph_caret_x) =
            measure(hyphenated, false);
        assert!(
            (hyph_h - short_h).abs() < 0.5,
            "hyphenated long text must never wrap (short {short_h} vs long {hyph_h})"
        );
        assert!(hyph_content_w > hyph_rect.width());
        assert!(
            hyph_scroll > 0.0,
            "overflowing field scrolls right to reveal the caret"
        );
        assert!(
            hyph_caret_x >= hyph_rect.min.x && hyph_caret_x <= hyph_rect.max.x,
            "caret x {hyph_caret_x} must stay inside {hyph_rect:?}"
        );

        let unbroken =
            "qatest0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz";
        let (unb_h, unb_rect, unb_content_w, unb_scroll, unb_caret_x) = measure(unbroken, false);
        assert!((unb_h - short_h).abs() < 0.5);
        assert!(unb_content_w > unb_rect.width());
        assert!(unb_scroll > 0.0);
        assert!(
            unb_caret_x >= unb_rect.min.x && unb_caret_x <= unb_rect.max.x,
            "unbroken caret x {unb_caret_x} must stay inside {unb_rect:?}"
        );

        // Moving caret back to the start keeps it visible at the left edge.
        let (_, start_rect, _, start_scroll, start_caret_x) = measure(unbroken, true);
        assert!(start_scroll.abs() < 0.5);
        assert!(
            start_caret_x >= start_rect.min.x && start_caret_x <= start_rect.max.x,
            "start caret x {start_caret_x} must stay inside {start_rect:?}"
        );
    }
}
