//! The report's text field (window B): the paragraph cut into spans around
//! the caret and the finished references, the caret, the box's own scroll,
//! and the suggestions — a popover under the caret under a pointer, a row of
//! chips under the field under a finger.

use baylee_client_core::bugreport::Suggestions;
use baylee_client_core::bugreport::refs::{self, CardRef, Target};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;

use crate::hud::tf;

use super::ReportDesk;
use super::form::{DeskPress, REPORT, suggestion_words};
use crate::shellkit::controls::{self, Kit};
use crate::shellkit::focus::Stop;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::tokens::{self, RADIUS_CONTROL};
use crate::shellkit::{Frame, Role};

/// The report's text box: it scrolls on its own, and keeps the caret in view
/// ([`place_the_caret`]). The [`REPORT`] table's `text` stop.
#[derive(Component)]
pub(crate) struct DeskBox;

/// The paragraph inside [`DeskBox`]: the text as spans — the part before the
/// selection, the selection, the part after it, each cut where a reference
/// begins and ends — which is what the caret is placed between. The counts
/// say how many spans each of the three parts took.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DeskText {
    /// Spans before the selection.
    pub(crate) head: usize,
    /// Spans of the selection.
    pub(crate) selected: usize,
    /// Spans after it.
    pub(crate) tail: usize,
}

/// The caret in [`DeskBox`]: a bar of its own, stood where the paragraph was
/// laid out ([`place_the_caret`]) and blinked by [`blink`].
///
/// It was the glyph `▏` inside the text until #320, which neither of the
/// faces this client ships has, so nothing was drawn where the caret was.
#[derive(Component, Default)]
pub(crate) struct DeskCaret {
    /// Whether it has been stood where the text is. Until then it is not
    /// drawn: a bar at the box's corner over a line of text is a caret in
    /// the wrong place for a frame.
    pub(crate) placed: bool,
}

/// The suggestions under the caret (a pointer's popover); stood where the
/// caret is by [`place_the_caret`].
#[derive(Component)]
pub(crate) struct DeskSuggest;

/// A popover row's second column (where the card is, or its type line):
/// it gives way to the name and wraps inside the popover.
#[derive(Component)]
pub(crate) struct DeskSuggestMeta;

/// A finished card reference's span in the text: hovering it previews the
/// card ([`super::refs`]).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReportLink(pub(crate) CardRef);

/// The text box's height in lines before it scrolls, and the height it
/// keeps when empty: six to twelve, two to four on a phone (§B.3).
const BOX_LINES: (f32, f32) = (6.0, 12.0);

const PHONE_BOX_LINES: (f32, f32) = (2.0, 4.0);

/// How many suggestions a phone's chip row holds.
const PHONE_CHIPS: usize = 4;

/// The caret's width, in logical pixels.
const CARET_WIDTH: f32 = 1.5;

/// The popover's width, in logical pixels at step 4.
const POPOVER_WIDTH: f32 = 360.0;

/// The text box: a paragraph that wraps inside it, a caret, a scroll of its
/// own once the text is longer than the box, and — under a pointer, while a
/// reference is being typed — the suggestions under the caret.
///
/// The paragraph is the text cut into spans, the caret between two of them
/// ([`DeskText`], [`DeskCaret`]); a finished reference is a span of its own,
/// a card's in the accent as the log draws a link and carrying a
/// [`ReportLink`] for its preview, a player's in bold. The empty box shows
/// the prompt as the text after the caret, muted, the way a placeholder
/// stands behind a caret.
pub(super) fn text_box(
    commands: &mut Commands,
    desk: &ReportDesk,
    kit: Kit,
    lang: Lang,
    suggestions: Option<&Suggestions>,
) -> Entity {
    let font = tf(kit.fonts, kit.m.text);
    let line = line_height(&font);
    let phone = kit.m.frame == Frame::Phone;
    // On a phone with the chips up the field gives them its second line, so
    // field and chips stay above the keyboard (y 195; the caret's line is
    // the one kept in view).
    let (fewest, most) = match (phone, suggestions.is_some() && kit.m.touch()) {
        (true, true) => (1.0, 1.0),
        (true, false) => PHONE_BOX_LINES,
        (false, _) => BOX_LINES,
    };
    let inset = if phone { 4.0 } else { kit.m.gap };
    let wrapper = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let field = commands
        .spawn((
            DeskBox,
            Role::Field,
            Stop::new(REPORT.name, "text"),
            Node {
                width: percent(100),
                min_height: px(line * fewest + inset * 2.0),
                max_height: px(line * most + inset * 2.0),
                padding: UiRect::all(px(inset)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            // Where the box was scrolled to before this rebuild, so a
            // keystroke does not throw a long text back to its first line.
            ScrollPosition(Vec2::new(0.0, desk.box_scroll)),
            BorderColor::all(tokens::ACCENT),
            BackgroundColor(crate::hud::palette::PANEL_LIT),
        ))
        .id();
    let page = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let (spans, counts) = paragraph(commands, desk, kit, lang);
    let linked = spans.1;
    let text = commands
        .spawn((
            counts,
            Text::new(""),
            font,
            TextColor(tokens::INK),
            TextLayout::linebreak(bevy::text::LineBreak::WordOrCharacter),
            Node {
                width: percent(100),
                ..default()
            },
            // Hoverable where a reference is, for its preview; never in the
            // way of the box under it.
            if linked {
                Pickable {
                    should_block_lower: false,
                    is_hoverable: true,
                }
            } else {
                Pickable::IGNORE
            },
        ))
        .id();
    commands.entity(text).add_children(&spans.0);
    let caret = commands
        .spawn((
            DeskCaret::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(CARET_WIDTH),
                height: px(line),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(page).add_children(&[text, caret]);
    commands.entity(field).add_child(page);
    commands.entity(wrapper).add_child(field);
    if !kit.m.touch()
        && let Some(list) = suggestions
    {
        let popover = popover(commands, desk, kit, lang, list);
        commands.entity(wrapper).add_child(popover);
    }
    wrapper
}

/// The paragraph's spans and how many each part took; `true` beside them
/// when a card reference is among them.
fn paragraph(
    commands: &mut Commands,
    desk: &ReportDesk,
    kit: Kit,
    lang: Lang,
) -> ((Vec<Entity>, bool), DeskText) {
    let font = tf(kit.fonts, kit.m.text);
    let buffer = &desk.form.text;
    let plain = |commands: &mut Commands, text: &str, ink: Color| {
        commands
            .spawn((TextSpan::new(text), font.clone(), TextColor(ink)))
            .id()
    };
    if buffer.is_empty() {
        let spans = vec![
            plain(commands, "", tokens::INK),
            plain(commands, "", tokens::INK),
            plain(commands, Phrase::ReportTextHint.text(lang), tokens::MUTED),
        ];
        let counts = DeskText {
            head: 1,
            selected: 1,
            tail: 1,
        };
        return ((spans, false), counts);
    }
    let text = buffer.text();
    let seg = buffer.segments();
    let head = 0..seg.head.len();
    let selected = head.end..head.end + seg.selected.len();
    let tail = selected.end..text.len();
    let found = refs::parse(text, &desk.gathered.refs, &desk.form.picked);
    let mut spans = Vec::new();
    let mut linked = false;
    let mut counts = DeskText::default();
    for (part, range) in [head, selected, tail].into_iter().enumerate() {
        let mut pieces: Vec<(std::ops::Range<usize>, Option<&Target>)> = Vec::new();
        let mut at = range.start;
        for span in &found {
            let from = span.at.start.max(range.start);
            let to = span.at.end.min(range.end);
            if from >= to {
                continue;
            }
            if from > at {
                pieces.push((at..from, None));
            }
            pieces.push((from..to, Some(&span.target)));
            at = to;
        }
        if at < range.end || pieces.is_empty() {
            pieces.push((at..range.end, None));
        }
        // One space the player never sees ends the tail, so the text after
        // the caret is never empty and its first run is where the caret
        // stands, even at the very end after a line break
        // (`baylee_client_core::caretspot`).
        let last_plain = pieces.last().is_some_and(|(_, t)| t.is_none());
        let mut count = 0;
        let pieces_len = pieces.len();
        for (i, (bytes, target)) in pieces.into_iter().enumerate() {
            let mut said = text[bytes].to_string();
            if part == 2 && i + 1 == pieces_len && last_plain {
                said.push(' ');
            }
            let mut span = commands.spawn((TextSpan::new(said), font.clone()));
            match target {
                Some(Target::Card(card)) => {
                    linked = true;
                    span.insert((
                        TextFont {
                            font: bevy::text::FontSource::Handle(kit.fonts.medium.clone()),
                            ..font.clone()
                        },
                        TextColor(tokens::ACCENT),
                        ReportLink(card.clone()),
                    ));
                }
                Some(Target::Player(_)) => {
                    span.insert((
                        TextFont {
                            font: bevy::text::FontSource::Handle(kit.fonts.bold.clone()),
                            ..font.clone()
                        },
                        TextColor(tokens::INK),
                    ));
                }
                None => {
                    span.insert(TextColor(tokens::INK));
                }
            }
            if part == 1 {
                span.insert(bevy::text::TextBackgroundColor(
                    crate::hud::palette::SELECTION,
                ));
            }
            spans.push(span.id());
            count += 1;
        }
        if part == 2 && !last_plain {
            spans.push(plain(commands, " ", tokens::INK));
            count += 1;
        }
        match part {
            0 => counts.head = count,
            1 => counts.selected = count,
            _ => counts.tail = count,
        }
    }
    ((spans, linked), counts)
}

/// The suggestions under the caret, under a pointer: a name and where it is,
/// the row the keys are on lit. Stood at the caret by [`place_the_caret`].
fn popover(
    commands: &mut Commands,
    desk: &ReportDesk,
    kit: Kit,
    lang: Lang,
    list: &Suggestions,
) -> Entity {
    let chosen = desk.form.chosen_in(list);
    let surface = commands
        .spawn((
            DeskSuggest,
            Role::Opaque,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: kit.m.px(POPOVER_WIDTH),
                max_width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(4.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            GlobalZIndex(1001),
            // Hidden until the caret has been found, so it never flashes at
            // the box's corner.
            Visibility::Hidden,
        ))
        .id();
    for (i, suggestion) in list.rows.iter().enumerate() {
        let (name, meta) = suggestion_words(desk, *suggestion, lang);
        let row = commands
            .spawn((
                Role::MenuItem,
                Node {
                    min_height: px_fixed(kit.m.control),
                    padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: kit.m.px(16.0),
                    border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL - 2.0)),
                    ..default()
                },
                BackgroundColor(if i == chosen {
                    tokens::SELECTED
                } else {
                    Color::NONE
                }),
                DeskPress::Take(i),
            ))
            .id();
        let name = controls::label(commands, kit, &name, kit.m.text, tokens::INK);
        commands.entity(name).insert(Node {
            flex_shrink: 0.0,
            ..default()
        });
        // The second column gives way to the name and wraps inside the
        // popover: a long type line ("Legendary Artifact Creature — Human")
        // never runs over its edge.
        let meta = commands
            .spawn((
                Text::new(meta),
                tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                TextLayout::justify(Justify::Right),
                Node {
                    flex_shrink: 1.0,
                    min_width: px_fixed(0.0),
                    padding: UiRect::axes(px_fixed(0.0), kit.m.px(4.0)),
                    ..default()
                },
                DeskSuggestMeta,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(&[name, meta]);
        commands.entity(surface).add_child(row);
    }
    surface
}

/// The suggestions under a finger: a row of chips under the field, at most
/// four, sideways if they run over (the vertical popover is a pointer's).
pub(super) fn chip_row(
    commands: &mut Commands,
    desk: &ReportDesk,
    kit: Kit,
    lang: Lang,
    list: &Suggestions,
) -> Entity {
    let row = commands
        .spawn((
            DeskSuggest,
            // It scrolls sideways when the chips run over (the checks read
            // it so).
            Role::Scroll,
            Node {
                width: percent(100),
                column_gap: kit.m.px(8.0),
                overflow: Overflow::scroll_x(),
                flex_shrink: 0.0,
                ..default()
            },
            ScrollPosition::default(),
        ))
        .id();
    for (i, suggestion) in list.rows.iter().take(PHONE_CHIPS).enumerate() {
        let (name, meta) = suggestion_words(desk, *suggestion, lang);
        let said = if meta.is_empty() {
            name
        } else {
            format!("{name} \u{b7} {meta}")
        };
        let face = commands
            .spawn((
                Node {
                    min_height: px_fixed(36.0),
                    padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                    align_items: AlignItems::Center,
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL),
                BorderColor::all(tokens::BORDER),
            ))
            .id();
        let words = controls::label(commands, kit, &said, kit.m.small, tokens::INK);
        commands.entity(face).add_child(words);
        let hit = controls::hit(commands, kit, face, DeskPress::Take(i));
        commands.entity(row).add_child(hit);
    }
    row
}

/// How tall a line of `font` is laid out, in logical pixels: bevy's default
/// line height is 1.2 times the size.
fn line_height(font: &TextFont) -> f32 {
    match font.font_size {
        bevy::text::FontSize::Px(size) => size * 1.2,
        _ => 20.0,
    }
}

/// Stands the caret where the paragraph was laid out, scrolls the box so the
/// caret is in it, and stands a pointer's suggestions under it.
///
/// After the text is laid out (`UiSystems::PostLayout`): the runs it reads
/// are this frame's, and what it writes is laid out on the next.
#[allow(clippy::type_complexity)] // Bevy queries: the caret and the popover
pub(super) fn place_the_caret(
    mut desk: ResMut<ReportDesk>,
    texts: Query<(&bevy::text::TextLayoutInfo, &DeskText)>,
    mut carets: Query<(&mut Node, &mut DeskCaret)>,
    mut popovers: Query<
        (&mut Node, &mut Visibility),
        (With<DeskSuggest>, With<GlobalZIndex>, Without<DeskCaret>),
    >,
    mut boxes: Query<(&mut ScrollPosition, &ComputedNode), With<DeskBox>>,
) {
    let (Ok((layout, counts)), Ok((mut node, mut caret))) = (texts.single(), carets.single_mut())
    else {
        return;
    };
    if layout.run_geometry.is_empty() && !desk.form.text.is_empty() {
        // Not laid out yet: no font, or the first frame of the tree.
        return;
    }
    let scale = layout.scale_factor.max(f32::EPSILON);
    let runs: Vec<baylee_client_core::caretspot::Run> = layout
        .run_geometry
        .iter()
        .map(|run| baylee_client_core::caretspot::Run {
            section: run.section_index,
            left: run.bounds.min.x / scale,
            top: run.bounds.min.y / scale,
            right: run.bounds.max.x / scale,
            bottom: run.bounds.max.y / scale,
        })
        .collect();
    // The sections, numbered as the renderer numbers them: the empty root
    // is 0, the spans from 1 in order.
    let head: Vec<usize> = (1..=counts.head).collect();
    let selected: Vec<usize> = (counts.head + 1..=counts.head + counts.selected).collect();
    let tail: Vec<usize> =
        (counts.head + counts.selected + 1..=counts.head + counts.selected + counts.tail).collect();
    let seg = desk.form.text.segments();
    let (before, after, seam) = if seg.caret_after_selection {
        let before_text = format!("{}{}", seg.head, seg.selected);
        (
            [head, selected].concat(),
            tail,
            baylee_client_core::caretspot::Seam::between(&before_text, seg.tail),
        )
    } else {
        let after_text = format!("{}{}", seg.selected, seg.tail);
        (
            head,
            [selected, tail].concat(),
            baylee_client_core::caretspot::Seam::between(seg.head, &after_text),
        )
    };
    let line = match &node.height {
        Val::Px(height) => *height,
        _ => 20.0,
    };
    let spot = baylee_client_core::caretspot::spot(&runs, &before, &after, seam, line);
    node.left = px(spot.x - CARET_WIDTH / 2.0);
    node.top = px(spot.top);
    node.height = px(spot.height);
    caret.placed = true;
    let Ok((mut scroll, computed)) = boxes.single_mut() else {
        return;
    };
    let inverse = computed.inverse_scale_factor();
    let inset = computed.padding().min_inset.y
        + computed.padding().max_inset.y
        + computed.border().min_inset.y
        + computed.border().max_inset.y;
    let viewport = (computed.size().y - inset) * inverse;
    if viewport <= 0.0 {
        // Not measured yet: nothing to keep the caret inside.
        return;
    }
    let followed = baylee_client_core::caretspot::follow(scroll.y, viewport, spot);
    if (followed - scroll.y).abs() > f32::EPSILON {
        scroll.y = followed;
    }
    desk.box_scroll = followed;
    // The popover stands under the caret's line, inside the box's width.
    if let Ok((mut popover, mut shown)) = popovers.single_mut() {
        let lead = (computed.padding().min_inset.x + computed.border().min_inset.x) * inverse;
        let top = (computed.padding().min_inset.y + computed.border().min_inset.y) * inverse;
        let width = computed.size().x * inverse;
        let wide = match popover.width {
            Val::Px(w) => w,
            _ => POPOVER_WIDTH,
        };
        let left = (lead + spot.x).min((width - wide).max(0.0));
        let below = top + spot.top + spot.height - followed + 4.0;
        let place = (px(left), px(below));
        if (popover.left, popover.top) != place {
            popover.left = place.0;
            popover.top = place.1;
        }
        shown.set_if_neq(Visibility::Inherited);
    }
}

/// What the caret last stood at: cursor, selection, text length. A change
/// in any of them is a caret that moved, lit again from the start.
type Drawn = (usize, Option<std::ops::Range<usize>>, usize);

/// Blinks the report's caret: lit while it moves, then on and off at the
/// rate every other text box here blinks at, and still under
/// `reduce_motion` (`lobby::caret_lit`).
pub(super) fn blink(
    time: Res<Time>,
    desk: Res<ReportDesk>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut carets: Query<(&DeskCaret, &mut BackgroundColor)>,
    mut since: Local<f32>,
    mut drawn: Local<Option<Drawn>>,
) {
    let Ok((caret, mut colour)) = carets.single_mut() else {
        *drawn = None;
        return;
    };
    let text = &desk.form.text;
    let now = Some((text.cursor(), text.selection(), text.text().len()));
    *since = if *drawn == now {
        *since + time.delta_secs()
    } else {
        0.0
    };
    *drawn = now;
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let want = if caret.placed && crate::lobby::caret_lit(*since, still) {
        tokens::INK
    } else {
        Color::NONE
    };
    if colour.0 != want {
        colour.0 = want;
    }
}
