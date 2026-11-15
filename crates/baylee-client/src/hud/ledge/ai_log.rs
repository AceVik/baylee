//! The AI log: what an AI seat's mind said beside its answers.
//!
//! A debugging and sparring tool, in debug builds only (`docs/protocol.md`
//! §"An AI seat's reasoning"): a debug engine forwards every seat's
//! reasoning (`v1::AiLog`) to every other seat at the table, the other
//! side's included, and a release build has none of it: its engine drops
//! them, its client queues none and stands no panel. The lines are worked
//! out in `baylee_client_core::aisaid`; this draws them, in a panel beside
//! the game log that opens from its own door in the tray. The door stands
//! only once something has been said ([`Duel::ai_log_heard`]), so a table
//! without an AI mind shows no door to an empty panel.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use crate::ui_button::UiButton;
use baylee_client_core::aisaid::{self, Kind};

use super::log::{LOG_H, LOG_HEAD_H, LOG_PAD, LOG_ROW_GAP};
const LINE_PT: f32 = 13.0;
const CLOSE_SIZE: f32 = LOG_HEAD_H - 2.0;
const CLOSE_PT: f32 = 10.0;

/// The panel.
#[derive(Component)]
pub struct AiLogPanel;

/// The scrolling list inside [`AiLogPanel`].
#[derive(Component)]
pub(crate) struct AiLogList;

/// The panel's header title, kept in step with the active language.
#[derive(Component)]
pub struct AiLogTitle;

/// Spawns the panel, hidden; [`update_ai_log`] dresses its interior on first
/// use and shows it while it is open.
pub(in crate::hud) fn spawn(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            AiLogPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H + LEDGE_PAD_Y),
                right: px(EDGE + menu::BURGER + 6.0 + log::LOG_W_MAX + LOG_PAD),
                width: px(log::LOG_W),
                height: px(LOG_H),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::top(px(STRIP_R)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            log::log_glass_ground(),
            BorderColor::all(palette::LOG_GLASS_EDGE),
            log::log_glass_shadow(),
            ZIndex(Z_LOG),
            Visibility::Hidden,
        ))
        .id()
}

fn title_words(lang: Lang) -> String {
    format!(
        "{} · {}",
        Phrase::RoomAi.text(lang),
        Phrase::GameLogTitle.text(lang)
    )
}

fn ai_log_head(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    let title = commands
        .spawn((
            AiLogTitle,
            Text::new(title_words(lang)),
            tf(fonts, LABEL_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        ))
        .id();
    let close = commands
        .spawn((
            UiButton,
            MenuButton {
                action: MenuAction::ToggleAiLog,
            },
            Node {
                width: px(CLOSE_SIZE),
                height: px(CLOSE_SIZE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::LOG_GLASS_TOP.with_alpha(0.55)),
            BorderColor::all(palette::LOG_GLASS_RULE),
            Feel::new(palette::LOG_GLASS_TOP.with_alpha(0.55)),
            crate::hud::Hint(Phrase::SheetClose.text(lang).to_string()),
            children![(
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, CLOSE_PT),
                TextColor(palette::LOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands
        .spawn((
            Node {
                height: px(LOG_HEAD_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::new(px(LOG_PAD), px(1), px(0), px(0)),
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::top(px(STRIP_R)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            log::log_head_ground(),
            BorderColor::all(palette::LOG_GLASS_RULE),
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .add_children(&[title, close])
        .id()
}

fn build_inside(commands: &mut Commands, panel: Entity, fonts: &UiFonts, lang: Lang) -> Entity {
    let head = ai_log_head(commands, fonts, lang);
    let list = commands
        .spawn((
            AiLogList,
            Scrolls,
            bevy::ui::ScrollPosition::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                min_height: px(0),
                row_gap: px(LOG_ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let track = log::scrollbar(
        commands,
        list,
        (
            palette::LOG_GLASS_EDGE.with_alpha(0.32),
            palette::CANDLE.with_alpha(0.84),
        ),
        log::at_rest,
    );
    commands
        .entity(track)
        .insert((log::OnDemandScrollbar(list), Visibility::Hidden));
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::all(px(LOG_PAD)),
                column_gap: px(4),
                ..default()
            },
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .add_children(&[list, track])
        .id();
    let rim = commands
        .spawn(crate::social::glass::layer(
            crate::social::glass::Glass::Panel,
            px(STRIP_R),
        ))
        .id();
    crate::hud::add_children_if_alive(commands, panel, &[head, body, rim]);
    list
}

/// The ink a line is drawn in.
fn ink(kind: Kind) -> Color {
    match kind {
        Kind::Head => palette::DIALOG_INK,
        Kind::Thinking | Kind::Cost => palette::LOG_SOFT,
        Kind::Chose | Kind::Say => palette::LEDGE_SOFT,
    }
}

/// Shows or hides the panel, and appends each reasoning that arrived.
pub fn update_ai_log(
    mut commands: Commands,
    mut panels: Query<(Entity, &mut Visibility), With<AiLogPanel>>,
    lists: Query<Entity, With<AiLogList>>,
    mut titles: Query<&mut Text, With<AiLogTitle>>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut duel: ResMut<Duel>,
) {
    let lang = Lang::of(&settings.lang);
    // Held while no panel stands (the overlay builds it with its root).
    let said = if duel.ai_said.is_empty() || panels.is_empty() {
        Vec::new()
    } else {
        std::mem::take(&mut duel.ai_said)
    };
    let lines: Vec<aisaid::Line> = said
        .iter()
        .flat_map(|log| {
            let seat = u8::try_from(log.seat).unwrap_or(u8::MAX);
            let who = baylee_client_core::i18n::seat_name(
                lang,
                duel.statics.as_ref(),
                PlayerId::new(seat),
            );
            aisaid::lines(lang, &who, &log.note, &log.thinking)
        })
        .collect();
    let shown = if duel.ai_log_open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let existing = lists.iter().next();
    if existing.is_some() {
        let want = title_words(lang);
        for mut title in &mut titles {
            if title.0 != want {
                title.0.clone_from(&want);
            }
        }
    }
    for (panel, mut seen) in &mut panels {
        if *seen != shown {
            *seen = shown;
        }
        if !duel.ai_log_open && lines.is_empty() {
            continue;
        }
        let target = existing.unwrap_or_else(|| build_inside(&mut commands, panel, &fonts, lang));
        for line in &lines {
            let row = commands
                .spawn((
                    Text::new(line.text.clone()),
                    tf(&fonts, LINE_PT),
                    TextColor(ink(line.kind)),
                    Node {
                        margin: UiRect::vertical(px(2.0)),
                        overflow: log::TEXT_OWN_HEIGHT,
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            crate::hud::add_child_if_alive(&mut commands, target, row);
        }
    }
}
