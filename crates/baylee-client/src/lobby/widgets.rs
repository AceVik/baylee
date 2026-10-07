//! The lobby's node makers: buttons, panels, headings, rows and chips.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// One row per chair: who is in it, what they brought, and — for the host —
/// A wrapping row of controls.
pub(crate) fn row(commands: &mut Commands, metrics: Metrics, wrap: bool) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap * 0.5),
                row_gap: px(metrics.gap * 0.5),
                flex_wrap: if wrap {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A list that scrolls inside its panel rather than pushing it off screen.
///
/// Deliberately *not* `Pickable::IGNORE`: a wheel over the gap between two
/// rows has to land on something, and [`scrolls`] walks up from whatever the
/// pointer hit to find this.
pub(crate) fn scroller(commands: &mut Commands, metrics: Metrics, which: List, at: f32) -> Entity {
    commands
        .spawn((
            Scrollable(which),
            // Not implied by the overflow: Bevy reads this component when it
            // has one and never adds it, so a list without it clips its rows
            // away and nothing can bring them back. It is seeded from where
            // the player left this list, because adding a card rebuilds the
            // tree and a list that jumped to the top on every tap would be
            // unusable.
            ScrollPosition(Vec2::new(0.0, at)),
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.35),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id()
}

/// A small toggle. Same shape as [`button`], sized for a row of them.
pub(crate) fn chip(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    on: bool,
) -> Entity {
    let height = if metrics.frame == Frame::Compact {
        metrics.tap
    } else {
        metrics.tap * 0.8
    };
    let id = crate::hud::answer_sized(
        commands,
        fonts,
        label,
        crate::hud::ButtonWeight::Secondary,
        None,
        height,
        metrics.small,
    );
    if on {
        super::button_style::primary(commands, id);
    }
    super::button_style::icon(commands, fonts, id, press, metrics.small);
    commands.entity(id).insert(press);
    id
}

/// What colour the line under a form is written in.
///
/// The only thing a [`Tone`] changes: a refusal is the one line a player has
/// to do something about, and in the same grey as "signing in…" it was read
/// straight past. `DANGER` is this palette's word for *this is what went
/// wrong* — the same one the table writes lethal damage in.
pub(super) fn status_ink(tone: Tone) -> Color {
    match tone {
        Tone::Note => palette::MUTED,
        Tone::Refusal => palette::DANGER,
    }
}

/// A button. A disabled one carries no [`Press`], so a click cannot find it.
pub(crate) fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    tone: Color,
    enabled: bool,
) -> Entity {
    let weight = if !enabled {
        crate::hud::ButtonWeight::Dead
    } else if tone == palette::DANGER {
        crate::hud::ButtonWeight::Danger
    } else {
        crate::hud::ButtonWeight::Secondary
    };
    let id = crate::hud::answer_sized(
        commands,
        fonts,
        label,
        weight,
        None,
        metrics.tap,
        metrics.text,
    );
    commands
        .entity(id)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.justify_content = JustifyContent::Center;
        });
    if enabled && (tone == palette::ACCENT || tone == palette::ACTIVE) {
        super::button_style::primary(commands, id);
    }
    super::button_style::icon(commands, fonts, id, press, metrics.small);
    if enabled {
        commands.entity(id).insert(press);
    }
    id
}

/// A column panel: a fixed width beside its neighbour, or the full width
/// above it.
pub(crate) fn panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                // A panel that grows to fill the row is also the one that has
                // to give way, and `min_width` has to be told: a flex item's
                // default minimum is its *content*, so a panel holding a row
                // wider than the window silently pushed the window's edge
                // instead of letting that row wrap. Seven table sizes made
                // that visible; two never had.
                flex_shrink: if grow > 0.0 && !metrics.stacked() {
                    1.0
                } else {
                    0.0
                },
                min_width: px(0),
                // Height comes from the content, with the screen as a floor.
                // Stretched to the row instead — which is what a flex item
                // does unasked — a panel is exactly one screen tall while its
                // rows carry on past the bottom of it, so a scrolled list
                // leaves the panel behind and is drawn on the backdrop.
                align_self: AlignSelf::Start,
                min_height: if metrics.stacked() {
                    px(0)
                } else {
                    percent(100)
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.8),
                padding: UiRect::all(px(metrics.pad * 1.5)),
                border_radius: BorderRadius::all(px(14)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::SANCTUARY_PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.4)),
            Pickable::IGNORE,
        ))
        .id()
}

/// A panel heading.
pub(crate) fn heading(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.head),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
}

/// A muted line where a list would be.
pub(crate) fn note(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// A deck row's printing, short enough to sit at the end of a list line.
///
/// Not the row's own text form: that repeats the count and the name, both of
/// which are already on the line, and it would be the widest thing on it.
pub(crate) fn print_mark(print: &baylee_core::deckrow::PrintChoice) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(set) = &print.set {
        parts.push(match &print.collector_number {
            Some(number) => format!("{set} {number}"),
            None => set.clone(),
        });
    } else if print.scryfall_id.is_some() {
        // A row pinned to one printing by id has nothing readable to show; it
        // still must not look like the plain row next to it.
        parts.push("pinned".to_string());
    }
    if let Some(lang) = &print.lang {
        parts.push(lang.to_uppercase());
    }
    match print.finish {
        Some(Finish::Foil) => parts.push("foil".to_string()),
        Some(Finish::Etched) => parts.push("etched".to_string()),
        Some(Finish::Holographic) => parts.push("holographic".to_string()),
        Some(Finish::Glitter) => parts.push("glitter".to_string()),
        Some(Finish::Galaxy) => parts.push("galaxy".to_string()),
        Some(Finish::Normal) | None => {}
    }
    parts.join(" \u{b7} ")
}

/// The stretch between the left and right halves of a row.
pub(crate) fn spacer() -> Node {
    Node {
        flex_grow: 1.0,
        ..default()
    }
}

/// A shared HUD surface used by the front door and account library.
pub(super) fn surface(commands: &mut Commands, metrics: Metrics) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id()
}
