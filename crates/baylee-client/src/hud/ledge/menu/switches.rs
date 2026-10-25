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

pub(super) fn priority_switch(commands: &mut Commands, fonts: &UiFonts) -> Entity {
    let metrics = crate::lobby::Metrics::of(MENU_W);
    let sound = crate::lobby::button(
        commands,
        fonts,
        metrics,
        "",
        crate::lobby::Press::Shared(crate::lobby::SharedPress::PickerNothing),
        palette::PANEL,
        true,
    );
    commands
        .entity(sound)
        .remove::<crate::lobby::Press>()
        .insert(UiButton)
        .with_child((
            Text::new(""),
            crate::hud::tf(fonts, metrics.text),
            TextColor(palette::INK),
            PrioritySwitch,
            Pickable::IGNORE,
        ))
        .observe(
            |mut click: On<PointerClick>, mut settings: ResMut<crate::settings::ClientSettings>| {
                click.propagate(false);
                settings.audio.priority_cue = !settings.audio.priority_cue;
                settings.save();
            },
        );
    let view = crate::lobby::button(
        commands,
        fonts,
        metrics,
        "",
        crate::lobby::Press::Shared(crate::lobby::SharedPress::PickerNothing),
        palette::PANEL,
        true,
    );
    commands
        .entity(view)
        .remove::<crate::lobby::Press>()
        .insert(UiButton)
        .with_child((
            Text::new(""),
            crate::hud::tf(fonts, metrics.text),
            TextColor(palette::INK),
            TextViewSwitch,
            Pickable::IGNORE,
        ))
        .observe(
            |mut click: On<PointerClick>, mut settings: ResMut<crate::settings::ClientSettings>| {
                click.propagate(false);
                settings.prefer_text_view = !settings.prefer_text_view;
                settings.save();
            },
        );
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
    commands.entity(row).add_children(&[sound, view]);
    row
}

/// Writes the switches' words when the setting or the language changed.
pub fn show_priority_switch(
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut labels: Query<&mut Text, (With<PrioritySwitch>, Without<TextViewSwitch>)>,
    mut view_labels: Query<&mut Text, (With<TextViewSwitch>, Without<PrioritySwitch>)>,
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
    let view_words = match (lang, settings.prefer_text_view) {
        (Lang::De, true) => "Kartenansicht: Text (V)",
        (Lang::De, false) => "Kartenansicht: Bild (V)",
        (Lang::En, true) => "Card view: Text (V)",
        (Lang::En, false) => "Card view: Art (V)",
    };
    for mut text in &mut view_labels {
        if text.0 != view_words {
            text.0 = view_words.to_string();
        }
    }
}

