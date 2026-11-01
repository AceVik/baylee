//! The game menu’s device settings.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The priority sound's switch (DESIGN-v7 §4.5, D22), beside the music's:
/// this device's `AudioMix::priority_cue`, saved with its settings. Its words
/// say the state and are kept current by [`show_priority_switch`].
#[derive(Component)]
pub struct PrioritySwitch;

/// Quick toggle for card text vs. art view (`ClientSettings::prefer_text_view`)
/// directly inside the in-game menu.
#[derive(Component)]
pub struct TextViewSwitch;

/// *Tisch folgt dem Zug* (`TableSettings::follow`), at the table itself
/// (report 30.10.2026): the camera's swing to whoever is on turn could be
/// switched off only in Settings › Graphics, away from the table.
#[derive(Component)]
pub struct FollowSwitch;

/// One switch of the menu: a row whose words a system keeps current (the
/// marker `M`), and a press that flips one setting of this device and saves.
fn switch<M: Component>(
    commands: &mut Commands,
    fonts: &UiFonts,
    marker: M,
    flip: fn(&mut crate::settings::ClientSettings),
) -> Entity {
    let metrics = crate::lobby::Metrics::of(MENU_W);
    let button = crate::lobby::button(
        commands,
        fonts,
        metrics,
        "",
        crate::lobby::Press::Shared(crate::lobby::SharedPress::PickerNothing),
        palette::PANEL,
        true,
    );
    commands
        .entity(button)
        .remove::<crate::lobby::Press>()
        .insert(UiButton)
        .with_child((
            Text::new(""),
            crate::hud::tf(fonts, metrics.text),
            TextColor(palette::INK),
            marker,
            Pickable::IGNORE,
        ))
        .observe(
            move |mut click: On<PointerClick>,
                  mut settings: ResMut<crate::settings::ClientSettings>| {
                click.propagate(false);
                if click.button != bevy::picking::pointer::PointerButton::Primary {
                    return;
                }
                flip(&mut settings);
                settings.save();
            },
        );
    button
}

pub(super) fn priority_switch(commands: &mut Commands, fonts: &UiFonts) -> Entity {
    let sound = switch(commands, fonts, PrioritySwitch, |s| {
        s.audio.priority_cue = !s.audio.priority_cue;
    });
    let view = switch(commands, fonts, TextViewSwitch, |s| {
        s.prefer_text_view = !s.prefer_text_view;
    });
    let follow = switch(commands, fonts, FollowSwitch, |s| {
        s.table.follow = !s.table.follow;
    });
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(MENU_ROW_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[sound, view, follow]);
    row
}

/// Writes the switches' words when the setting or the language changed.
#[allow(clippy::type_complexity)] // three switches, each its own words
pub fn show_priority_switch(
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut labels: Query<
        &mut Text,
        (
            With<PrioritySwitch>,
            Without<TextViewSwitch>,
            Without<FollowSwitch>,
        ),
    >,
    mut view_labels: Query<
        &mut Text,
        (
            With<TextViewSwitch>,
            Without<PrioritySwitch>,
            Without<FollowSwitch>,
        ),
    >,
    mut follow_labels: Query<
        &mut Text,
        (
            With<FollowSwitch>,
            Without<PrioritySwitch>,
            Without<TextViewSwitch>,
        ),
    >,
) {
    let Some(settings) = settings else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let words = if settings.audio.priority_cue {
        Phrase::PriorityCueOn
    } else {
        Phrase::PriorityCueOff
    }
    .text(lang);
    for mut text in &mut labels {
        if text.0 != words {
            text.0 = words.to_string();
        }
    }
    let view_words = if settings.prefer_text_view {
        Phrase::CardViewText
    } else {
        Phrase::CardViewArt
    }
    .text(lang);
    for mut text in &mut view_labels {
        if text.0 != view_words {
            text.0 = view_words.to_string();
        }
    }
    let follow_words = if settings.table.follow {
        Phrase::FollowTurnOn
    } else {
        Phrase::FollowTurnOff
    }
    .text(lang);
    for mut text in &mut follow_labels {
        if text.0 != follow_words {
            text.0 = follow_words.to_string();
        }
    }
}
